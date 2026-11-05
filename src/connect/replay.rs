use std::{
    sync::{Mutex, MutexGuard, PoisonError},
    time::SystemTime,
};

use fastgit_protocol::{AccountContext, Cassette, Exchange, Reel};
use futures::{FutureExt as _, future::BoxFuture};
use url::Url;

use crate::{
    accounts::{AccountId, TokenScheme},
    net::{
        policy::{AllowList, check_headers},
        request::{ApiRequest, ApiResponse, NetError},
        services::{AccountView, Cancel, ConnectorId, HostServices, Level},
    },
};

const WEB_ENDPOINT: &str = "web";

pub(crate) struct ReplayHost {
    account: Option<AccountView>,
    allowed: AllowList,
    reel: Mutex<Reel>,
    log: Mutex<Vec<String>>,
}

impl ReplayHost {
    pub(crate) fn idle() -> Self {
        Self {
            account: None,
            allowed: AllowList::of_endpoints([]),
            reel: Mutex::default(),
            log: Mutex::default(),
        }
    }

    pub(crate) fn playing(cassette: &Cassette) -> Result<Self, String> {
        let account = view_of(&cassette.account)?;
        Ok(Self {
            allowed: AllowList::of_endpoints(account.endpoints.iter().map(|(_, url)| url.clone())),
            account: Some(account),
            reel: Mutex::new(cassette.reel()),
            log: Mutex::default(),
        })
    }

    pub(crate) fn account_id(&self) -> Option<AccountId> {
        self.account.as_ref().map(|account| account.id.clone())
    }

    pub(crate) fn unplayed(&self) -> Vec<Exchange> {
        locked(&self.reel).unplayed().cloned().collect()
    }

    pub(crate) fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *locked(&self.log))
    }

    fn answer(&self, account: &AccountId, request: &ApiRequest) -> Result<ApiResponse, NetError> {
        if self.account(account).is_none() {
            return Err(NetError::NotSignedIn);
        }
        let url = Url::parse(&request.url)
            .map_err(|_| NetError::ServerNotAllowed(request.url.clone()))?;
        self.allowed
            .admit(&url)
            .ok_or_else(|| NetError::ServerNotAllowed(url.to_string()))?;
        check_headers(&request.headers, &TokenScheme::Bearer)?;
        let method = request.method.name();
        let mut reel = locked(&self.reel);
        let exchange = reel.play(method, &request.url).ok_or_else(|| {
            NetError::Unreachable(format!(
                "The cassette holds no answer to {method} {}",
                request.url
            ))
        })?;
        Ok(ApiResponse {
            status: exchange.status,
            headers: exchange.headers.clone(),
            body: exchange.body_bytes(),
        })
    }
}

impl HostServices for ReplayHost {
    fn account(&self, account: &AccountId) -> Option<AccountView> {
        self.account.clone().filter(|known| known.id == *account)
    }

    fn request(
        &self,
        account: &AccountId,
        request: ApiRequest,
        _: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>> {
        futures::future::ready(self.answer(account, &request)).boxed()
    }

    fn pause(&self, _: &AccountId, _: SystemTime) {}

    fn log(&self, _: &ConnectorId, level: Level, message: &str) {
        locked(&self.log).push(format!("{level:?} {message}"));
    }

    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn view_of(account: &AccountContext) -> Result<AccountView, String> {
    let id = AccountId::parse(&account.id).ok_or_else(|| {
        format!(
            "The cassette's account id {} isn't connector:host:user",
            account.id
        )
    })?;
    let endpoints = account
        .endpoints
        .iter()
        .map(|(name, url)| {
            Url::parse(url)
                .map(|url| (name.clone(), url))
                .map_err(|error| format!("The cassette's {name} endpoint {url} isn't a URL: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let server = endpoints
        .iter()
        .find(|(name, _)| name == WEB_ENDPOINT)
        .or_else(|| endpoints.first())
        .map(|(_, url)| url.clone())
        .ok_or_else(|| "The cassette's account has no endpoints".to_owned())?;
    Ok(AccountView {
        id,
        server,
        endpoints,
        login: account.login.clone(),
    })
}

#[cfg(test)]
mod tests {
    use fastgit_protocol::Cassette;
    use serde_json::json;

    use super::ReplayHost;
    use crate::{
        accounts::AccountId,
        net::{
            request::{ApiRequest, ApiResponse, Method, NetError},
            services::{HostServices as _, cancel_pair},
        },
    };

    fn cassette() -> Cassette {
        serde_json::from_value(json!({
            "op": "account.who_am_i",
            "account": {
                "id": "fake:example.com:1",
                "login": "octo",
                "endpoints": { "api": "https://api.example.com/v1/", "web": "https://example.com/" }
            },
            "exchanges": [
                { "method": "GET", "url": "https://api.example.com/v1/user", "status": 200, "body": { "login": "octo" } },
                { "method": "GET", "url": "https://api.example.com/v1/user", "status": 304, "headers": [["etag", "\"x\""]] },
                { "method": "POST", "url": "https://api.example.com/v1/keys", "status": 201, "body": "plain text" }
            ]
        }))
        .unwrap()
    }

    fn send(host: &ReplayHost, request: ApiRequest) -> Result<ApiResponse, NetError> {
        let (_keep, cancel) = cancel_pair();
        let account = AccountId::parse("fake:example.com:1").unwrap();
        smol::block_on(host.request(&account, request, cancel))
    }

    #[test]
    fn a_cassette_answers_each_request_once_in_order() {
        let host = ReplayHost::playing(&cassette()).unwrap();
        let user = || ApiRequest::new(Method::Get, "https://api.example.com/v1/user");
        let first = send(&host, user()).unwrap();
        assert_eq!((first.status, first.body), (200, br#"{"login":"octo"}"#.to_vec()));
        let second = send(&host, user()).unwrap();
        assert_eq!((second.status, second.header("etag")), (304, Some("\"x\"")));
        assert!(second.body.is_empty());
        assert_eq!(
            send(&host, user()),
            Err(NetError::Unreachable(
                "The cassette holds no answer to GET https://api.example.com/v1/user".into()
            ))
        );
        let unplayed = host.unplayed();
        assert_eq!(unplayed.len(), 1);
        assert_eq!(unplayed[0].body_bytes(), b"plain text");
    }

    #[test]
    fn a_replayed_request_follows_the_gateways_rules() {
        let host = ReplayHost::playing(&cassette()).unwrap();
        let stolen = ApiRequest::new(Method::Get, "https://api.example.com/v1/user")
            .header("Authorization", "Bearer stolen");
        assert_eq!(send(&host, stolen), Err(NetError::HeaderNotAllowed("Authorization".into())));
        assert_eq!(
            send(&host, ApiRequest::new(Method::Get, "https://evil.example/collect")),
            Err(NetError::ServerNotAllowed("https://evil.example/collect".into()))
        );
        assert_eq!(host.unplayed().len(), 3);
    }

    #[test]
    fn an_idle_host_has_no_account_to_lend() {
        let host = ReplayHost::idle();
        assert_eq!(host.account_id(), None);
        assert_eq!(
            send(&host, ApiRequest::new(Method::Get, "https://api.example.com/v1/user")),
            Err(NetError::NotSignedIn)
        );
    }

    #[test]
    fn a_cassette_account_must_have_a_proper_id_and_endpoints() {
        let mut broken = cassette();
        broken.account.id = "not an id".into();
        assert!(ReplayHost::playing(&broken).is_err());
        let mut bare = cassette();
        bare.account.endpoints.clear();
        assert!(ReplayHost::playing(&bare).is_err());
    }
}
