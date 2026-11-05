use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock},
    time::{Duration, SystemTime},
};

use futures::{FutureExt as _, future::BoxFuture};
use smol::lock::Semaphore;
use url::Url;

use super::{
    curl::{Outgoing, Transport},
    policy::{Admission, AllowList, check_headers, redirect_target},
    request::{ApiRequest, ApiResponse, Method, NetError},
    services::{AccountView, Cancel, ConnectorId, HostServices, Level},
};
use crate::accounts::{Account, AccountBook, AccountId, Secrets};

const USER_AGENT: (&str, &str) = ("User-Agent", concat!("FastGit/", env!("CARGO_PKG_VERSION")));
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const REQUESTS_AT_ONCE_PER_ACCOUNT: usize = 4;
const REDIRECTS_FOLLOWED: usize = 5;
const LOG_LINES_KEPT: usize = 200;
const HIDDEN: &str = "•••";
const SET_COOKIE: &str = "set-cookie";

pub(crate) struct Gateway {
    book: Arc<RwLock<AccountBook>>,
    secrets: Arc<dyn Secrets>,
    transport: Arc<dyn Transport>,
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    paused: HashMap<AccountId, SystemTime>,
    gates: HashMap<AccountId, Arc<Semaphore>>,
    logs: HashMap<ConnectorId, VecDeque<String>>,
    hidden: Vec<String>,
}

struct Trip {
    allowed: AllowList,
    token_header: Option<(String, String)>,
    method: Method,
    url: Url,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    timeout: Duration,
}

impl Gateway {
    pub(crate) fn new(
        book: Arc<RwLock<AccountBook>>,
        secrets: Arc<dyn Secrets>,
        transport: Arc<dyn Transport>,
    ) -> Self {
        Self {
            book,
            secrets,
            transport,
            state: Mutex::default(),
        }
    }

    pub(crate) fn log_lines(&self, connector: &ConnectorId) -> Vec<String> {
        self.state()
            .logs
            .get(connector)
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn prepare(&self, account: &AccountId, request: ApiRequest) -> Result<Trip, NetError> {
        let book = self.book.read().unwrap_or_else(PoisonError::into_inner);
        let found = book.account(account).ok_or(NetError::NotSignedIn)?;
        if let Some(until) = self.paused_until(account) {
            return Err(NetError::RateLimitedUntil(until));
        }
        let url = Url::parse(&request.url)
            .map_err(|_| NetError::ServerNotAllowed(request.url.clone()))?;
        let allowed = AllowList::of(found);
        let admission = allowed
            .admit(&url)
            .ok_or_else(|| NetError::ServerNotAllowed(url.to_string()))?;
        check_headers(&request.headers, &found.scheme)?;
        let token_header = match admission {
            Admission::WithToken => Some(self.token_header(found)?),
            Admission::WithoutToken => None,
        };
        Ok(Trip {
            allowed,
            token_header,
            method: request.method,
            url,
            headers: request.headers,
            body: request.body,
            timeout: request.timeout.unwrap_or(DEFAULT_TIMEOUT),
        })
    }

    fn token_header(&self, account: &Account) -> Result<(String, String), NetError> {
        let token = self
            .secrets
            .read(&account.id)
            .ok()
            .flatten()
            .ok_or(NetError::NotSignedIn)?;
        let header = account.scheme.header(token.reveal());
        let mut state = self.state();
        for secret in [token.reveal(), header.1.as_str()] {
            if !state.hidden.iter().any(|known| known == secret) {
                state.hidden.push(secret.to_owned());
            }
        }
        Ok(header)
    }

    fn paused_until(&self, account: &AccountId) -> Option<SystemTime> {
        self.state()
            .paused
            .get(account)
            .copied()
            .filter(|until| *until > SystemTime::now())
    }

    fn gate(&self, account: &AccountId) -> Arc<Semaphore> {
        self.state()
            .gates
            .entry(account.clone())
            .or_insert_with(|| Arc::new(Semaphore::new(REQUESTS_AT_ONCE_PER_ACCOUNT)))
            .clone()
    }
}

async fn travel(
    trip: Trip,
    transport: Arc<dyn Transport>,
    gate: Arc<Semaphore>,
    cancel: Cancel,
) -> Result<ApiResponse, NetError> {
    let _turn = gate.acquire_arc().await;
    let mut url = trip.url.clone();
    for _ in 0..=REDIRECTS_FOLLOWED {
        let admission = trip
            .allowed
            .admit(&url)
            .ok_or_else(|| NetError::ServerNotAllowed(url.to_string()))?;
        let mut headers = trip.headers.clone();
        headers.push((USER_AGENT.0.to_owned(), USER_AGENT.1.to_owned()));
        if admission == Admission::WithToken {
            headers.extend(trip.token_header.clone());
        }
        let outgoing = Outgoing {
            method: trip.method,
            url: url.clone(),
            headers,
            body: trip.body.clone(),
            timeout: trip.timeout,
        };
        let mut response = transport.exchange(outgoing, cancel.clone()).await?;
        match redirect_target(&response, trip.method, &url) {
            Some(next) => url = next,
            None => {
                response
                    .headers
                    .retain(|(name, _)| !name.eq_ignore_ascii_case(SET_COOKIE));
                return Ok(response);
            }
        }
    }
    Err(NetError::Unreachable(
        "The server redirected too many times".to_owned(),
    ))
}

fn hidden(message: &str, secrets: &[String]) -> String {
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(message.to_owned(), |text, secret| text.replace(secret.as_str(), HIDDEN))
}

impl HostServices for Gateway {
    fn account(&self, account: &AccountId) -> Option<AccountView> {
        let book = self.book.read().unwrap_or_else(PoisonError::into_inner);
        book.account(account).map(|found| AccountView {
            id: found.id.clone(),
            server: found.server.clone(),
            endpoints: found
                .endpoints
                .iter()
                .map(|endpoint| (endpoint.name.clone(), endpoint.url.clone()))
                .collect(),
            login: found.login.clone(),
        })
    }

    fn request(
        &self,
        account: &AccountId,
        request: ApiRequest,
        cancel: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>> {
        match self.prepare(account, request) {
            Ok(trip) => travel(trip, self.transport.clone(), self.gate(account), cancel).boxed(),
            Err(error) => futures::future::ready(Err(error)).boxed(),
        }
    }

    fn pause(&self, account: &AccountId, until: SystemTime) {
        self.state().paused.insert(account.clone(), until);
    }

    fn log(&self, connector: &ConnectorId, level: Level, message: &str) {
        let mut state = self.state();
        let line = format!("{level:?} {}", hidden(message, &state.hidden));
        let lines = state.logs.entry(connector.clone()).or_default();
        if lines.len() == LOG_LINES_KEPT {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex, RwLock},
        time::{Duration, SystemTime},
    };

    use futures::{FutureExt as _, future::BoxFuture};

    use super::Gateway;
    use crate::{
        accounts::{AccountBook, MemorySecrets, Secrets, fixtures},
        net::{
            curl::{Curl, Outgoing, Transport},
            request::{ApiRequest, ApiResponse, Method, NetError},
            services::{Cancel, ConnectorId, HostServices as _, Level, cancel_pair},
        },
        test_server::{Reply, TestServer},
    };

    #[derive(Default)]
    struct Recorder {
        replies: Mutex<VecDeque<ApiResponse>>,
        sent: Mutex<Vec<Outgoing>>,
    }

    impl Recorder {
        fn answering(replies: Vec<ApiResponse>) -> Arc<Self> {
            Arc::new(Self {
                replies: Mutex::new(replies.into()),
                sent: Mutex::default(),
            })
        }

        fn sent(&self) -> Vec<Outgoing> {
            self.sent.lock().unwrap().clone()
        }
    }

    impl Transport for Recorder {
        fn exchange(
            &self,
            outgoing: Outgoing,
            _: Cancel,
        ) -> BoxFuture<'static, Result<ApiResponse, NetError>> {
            self.sent.lock().unwrap().push(outgoing);
            let reply = self.replies.lock().unwrap().pop_front().unwrap_or_else(|| answer(200, &[]));
            futures::future::ready(Ok(reply)).boxed()
        }
    }

    fn answer(status: u16, headers: &[(&str, &str)]) -> ApiResponse {
        ApiResponse {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: Vec::new(),
        }
    }

    fn gateway(transport: Arc<dyn Transport>, secrets: Arc<dyn Secrets>) -> Gateway {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        Gateway::new(Arc::new(RwLock::new(book)), secrets, transport)
    }

    fn signed_in(transport: Arc<dyn Transport>) -> Gateway {
        let account = fixtures::github("1").id;
        gateway(transport, Arc::new(MemorySecrets::holding(&account, "gho_secret")))
    }

    fn send(gateway: &Gateway, request: ApiRequest) -> Result<ApiResponse, NetError> {
        let (_keep, cancel) = cancel_pair();
        smol::block_on(gateway.request(&fixtures::github("1").id, request, cancel))
    }

    fn header<'a>(outgoing: &'a Outgoing, name: &str) -> Option<&'a str> {
        outgoing
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn the_token_goes_only_to_endpoints_that_carry_it() {
        let recorder = Recorder::answering(Vec::new());
        let gateway = signed_in(recorder.clone());
        send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/user")).unwrap();
        send(&gateway, ApiRequest::new(Method::Get, "https://avatars.githubusercontent.com/u/1")).unwrap();
        let sent = recorder.sent();
        assert_eq!(header(&sent[0], "authorization"), Some("Bearer gho_secret"));
        assert_eq!(header(&sent[1], "authorization"), None);
        assert!(header(&sent[0], "user-agent").unwrap().starts_with("FastGit/"));
    }

    #[test]
    fn a_server_that_is_not_the_accounts_is_refused_without_being_contacted() {
        let recorder = Recorder::answering(Vec::new());
        let gateway = signed_in(recorder.clone());
        let refused = send(&gateway, ApiRequest::new(Method::Get, "https://evil.example/collect"));
        assert_eq!(refused, Err(NetError::ServerNotAllowed("https://evil.example/collect".into())));
        assert!(recorder.sent().is_empty());
    }

    #[test]
    fn a_connector_cannot_set_its_own_authorization() {
        let recorder = Recorder::answering(Vec::new());
        let gateway = signed_in(recorder.clone());
        let request = ApiRequest::new(Method::Get, "https://api.github.com/user")
            .header("Authorization", "Bearer stolen");
        assert_eq!(
            send(&gateway, request),
            Err(NetError::HeaderNotAllowed("Authorization".into()))
        );
        assert!(recorder.sent().is_empty());
    }

    #[test]
    fn a_redirect_to_another_server_is_refused_before_the_token_can_follow() {
        let recorder = Recorder::answering(vec![answer(302, &[("Location", "https://evil.example/x")])]);
        let gateway = signed_in(recorder.clone());
        let result = send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/user"));
        assert_eq!(result, Err(NetError::ServerNotAllowed("https://evil.example/x".into())));
        assert_eq!(recorder.sent().len(), 1);
    }

    #[test]
    fn a_redirect_within_the_endpoint_is_followed_with_the_token() {
        let recorder = Recorder::answering(vec![
            answer(301, &[("Location", "/repositories/7")]),
            answer(200, &[("Set-Cookie", "session=1"), ("ETag", "\"x\"")]),
        ]);
        let gateway = signed_in(recorder.clone());
        let response = send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/repos/a/b")).unwrap();
        let sent = recorder.sent();
        assert_eq!(sent[1].url.as_str(), "https://api.github.com/repositories/7");
        assert_eq!(header(&sent[1], "authorization"), Some("Bearer gho_secret"));
        assert_eq!(response.header("set-cookie"), None);
        assert_eq!(response.header("etag"), Some("\"x\""));
    }

    #[test]
    fn a_redirect_answering_a_write_is_handed_back_unfollowed() {
        let recorder = Recorder::answering(vec![answer(303, &[("Location", "/done")])]);
        let gateway = signed_in(recorder.clone());
        let request = ApiRequest::new(Method::Post, "https://api.github.com/user/repos").body("{}");
        assert_eq!(send(&gateway, request).unwrap().status, 303);
        assert_eq!(recorder.sent().len(), 1);
    }

    #[test]
    fn an_account_without_a_token_is_not_signed_in() {
        let recorder = Recorder::answering(Vec::new());
        let gateway = gateway(recorder.clone(), Arc::new(MemorySecrets::default()));
        let result = send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/user"));
        assert_eq!(result, Err(NetError::NotSignedIn));
        assert!(recorder.sent().is_empty());
    }

    #[test]
    fn a_paused_account_fails_at_once_until_the_pause_ends() {
        let recorder = Recorder::answering(Vec::new());
        let gateway = signed_in(recorder.clone());
        let until = SystemTime::now() + Duration::from_secs(60);
        gateway.pause(&fixtures::github("1").id, until);
        let result = send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/user"));
        assert_eq!(result, Err(NetError::RateLimitedUntil(until)));
        gateway.pause(&fixtures::github("1").id, SystemTime::now() - Duration::from_secs(1));
        assert!(send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/user")).is_ok());
    }

    #[test]
    fn logs_keep_the_latest_lines_and_never_show_a_token() {
        let gateway = signed_in(Recorder::answering(Vec::new()));
        send(&gateway, ApiRequest::new(Method::Get, "https://api.github.com/user")).unwrap();
        let connector = ConnectorId("github".into());
        gateway.log(&connector, Level::Info, "started");
        gateway.log(&connector, Level::Warn, "slow answer");
        for line in 0..250 {
            gateway.log(&connector, Level::Debug, &format!("line {line}"));
        }
        gateway.log(&connector, Level::Error, "echoed gho_secret back");
        let lines = gateway.log_lines(&connector);
        assert_eq!(lines.len(), 200);
        assert_eq!(lines[198], "Debug line 249");
        assert_eq!(lines[199], "Error echoed ••• back");
    }

    #[test]
    fn a_connector_sees_its_account_but_never_its_token() {
        let gateway = signed_in(Recorder::answering(Vec::new()));
        let view = gateway.account(&fixtures::github("1").id).unwrap();
        assert_eq!(view.login, "login-1");
        assert_eq!(view.server.as_str(), "https://github.com/");
        assert_eq!(
            view.endpoints.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
            ["api", "graphql", "images"]
        );
        assert!(!format!("{view:?}").contains("gho_secret"));
        assert!(gateway.now() <= SystemTime::now());
    }

    #[test]
    fn the_gateway_and_curl_bring_the_bearer_token_to_a_real_server() {
        let server = TestServer::start(|seen| match seen.header("authorization") {
            Some("Bearer gho_secret") => Reply::status(200).body("{\"login\":\"octo\"}"),
            _ => Reply::status(401),
        });
        let mut account = fixtures::account(&server.url(""), "1", &[("api", &server.url("/api/"), true)]);
        account.id = fixtures::github("1").id;
        let mut book = AccountBook::default();
        book.put(account);
        let secrets = Arc::new(MemorySecrets::holding(&fixtures::github("1").id, "gho_secret"));
        let gateway = Gateway::new(Arc::new(RwLock::new(book)), secrets, Arc::new(Curl::ignoring_git_config()));
        let response = send(&gateway, ApiRequest::new(Method::Get, server.url("/api/user"))).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"{\"login\":\"octo\"}");
        assert_eq!(server.seen()[0].target, "/api/user");
    }
}
