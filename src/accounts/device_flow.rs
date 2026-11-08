use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use url::Url;

use super::{
    Tokens,
    oauth::{self, Client},
};
use crate::net::{curl::Transport, services::Cancel};

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const SLOWER_BY: Duration = Duration::from_secs(5);
const USUAL_INTERVAL: Duration = Duration::from_secs(5);
const USUAL_LIFETIME: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub(crate) struct DeviceFlow {
    transport: Arc<dyn Transport>,
    client: Client,
    scopes: Vec<String>,
    code_url: Url,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeviceCode {
    pub(crate) device_code: String,
    pub(crate) user_code: String,
    pub(crate) verification_uri: Url,
    pub(crate) lifetime: Duration,
    pub(crate) interval: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Polled {
    Waiting,
    SlowDown,
    Granted(Tokens),
    Expired,
    Denied,
    Failed(String),
}

impl Polled {
    pub(crate) fn next_interval(&self, interval: Duration) -> Duration {
        match self {
            Polled::SlowDown => interval + SLOWER_BY,
            _ => interval,
        }
    }

    pub(crate) fn keeps_waiting(&self) -> bool {
        matches!(self, Polled::Waiting | Polled::SlowDown)
    }
}

impl DeviceFlow {
    pub(crate) fn new(
        transport: Arc<dyn Transport>,
        client_id: String,
        scopes: Vec<String>,
        code_url: Url,
        token_url: Url,
    ) -> Self {
        Self {
            transport,
            client: Client {
                id: client_id,
                token_url,
            },
            scopes,
            code_url,
        }
    }

    pub(crate) fn ask_for_code(
        &self,
        cancel: Cancel,
    ) -> impl Future<Output = Result<DeviceCode, String>> + Send + 'static {
        let scopes = self.scopes.join(" ");
        let sent = oauth::post(
            &self.transport,
            &self.code_url,
            &[("client_id", &self.client.id), ("scope", &scopes)],
            cancel,
        );
        async move {
            let answer = sent.await?;
            let text = |name: &str| answer[name].as_str().map(str::to_owned);
            let seconds = |name: &str| answer[name].as_u64().map(Duration::from_secs);
            let issued = (text("device_code"), text("user_code"), text("verification_uri"));
            let (Some(device_code), Some(user_code), Some(page)) = issued else {
                return Err(oauth::refusal(&answer, "The server gave no code to enter"));
            };
            let verification_uri = Url::parse(&page)
                .ok()
                .filter(|url| matches!(url.scheme(), "https" | "http"))
                .ok_or_else(|| format!("The server named a strange page to open: {page}"))?;
            Ok(DeviceCode {
                device_code,
                user_code,
                verification_uri,
                lifetime: seconds("expires_in").unwrap_or(USUAL_LIFETIME),
                interval: seconds("interval").unwrap_or(USUAL_INTERVAL),
            })
        }
    }

    pub(crate) fn poll(
        &self,
        code: &DeviceCode,
        cancel: Cancel,
    ) -> impl Future<Output = Polled> + Send + 'static {
        let sent = oauth::post(
            &self.transport,
            &self.client.token_url,
            &[
                ("client_id", &self.client.id),
                ("device_code", &code.device_code),
                ("grant_type", GRANT_TYPE),
            ],
            cancel,
        );
        let client = self.client.clone();
        async move {
            let answer = match sent.await {
                Ok(answer) => answer,
                Err(problem) => return Polled::Failed(problem),
            };
            if answer.get("access_token").is_some() {
                return match client.tokens_in(&answer, SystemTime::now()) {
                    Some(tokens) => Polled::Granted(tokens),
                    None => Polled::Failed("The server's token can't be used".to_owned()),
                };
            }
            match answer["error"].as_str() {
                Some("authorization_pending") => Polled::Waiting,
                Some("slow_down") => Polled::SlowDown,
                Some("expired_token") => Polled::Expired,
                Some("access_denied") => Polled::Denied,
                _ => Polled::Failed(oauth::refusal(&answer, "The server answered without a token")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, SystemTime},
    };

    use url::Url;

    use super::{DeviceCode, DeviceFlow, Polled};
    use crate::{
        accounts::{Token, Tokens},
        net::{curl::Curl, services::cancel_pair},
        test_server::{Reply, Seen, TestServer},
    };

    fn flow(server: &TestServer) -> DeviceFlow {
        DeviceFlow::new(
            Arc::new(Curl::ignoring_git_config()),
            "Iv1.sternegit".into(),
            vec!["repo".into(), "read:org".into()],
            Url::parse(&server.url("/login/device/code")).unwrap(),
            Url::parse(&server.url("/login/oauth/access_token")).unwrap(),
        )
    }

    fn json(body: &str) -> Reply {
        Reply::status(200).header("Content-Type", "application/json").body(body)
    }

    fn code() -> DeviceCode {
        DeviceCode {
            device_code: "dc-1".into(),
            user_code: "8F2C-K7QD".into(),
            verification_uri: Url::parse("https://github.com/login/device").unwrap(),
            lifetime: Duration::from_secs(900),
            interval: Duration::from_secs(5),
        }
    }

    fn form_of(seen: &Seen) -> Vec<(String, String)> {
        url::form_urlencoded::parse(&seen.body).into_owned().collect()
    }

    #[test]
    fn the_code_request_names_the_client_and_its_scopes_and_reads_the_code_back() {
        let server = TestServer::start(|_| {
            json(r#"{"device_code":"dc-1","user_code":"8F2C-K7QD","verification_uri":"https://github.com/login/device","expires_in":900,"interval":5}"#)
        });
        let (_keep, cancel) = cancel_pair();
        let code = smol::block_on(flow(&server).ask_for_code(cancel)).unwrap();
        assert_eq!(code.user_code, "8F2C-K7QD");
        assert_eq!(code.verification_uri.as_str(), "https://github.com/login/device");
        assert_eq!((code.lifetime, code.interval), (Duration::from_secs(900), Duration::from_secs(5)));
        let seen = &server.seen()[0];
        assert_eq!((seen.method.as_str(), seen.target.as_str()), ("POST", "/login/device/code"));
        assert_eq!(seen.header("accept"), Some("application/json"));
        assert_eq!(
            form_of(seen),
            [("client_id".into(), "Iv1.sternegit".into()), ("scope".into(), "repo read:org".into())]
        );
    }

    #[test]
    fn polling_waits_slows_down_and_finally_hands_over_the_token() {
        let polls = Arc::new(AtomicUsize::new(0));
        let counted = polls.clone();
        let server = TestServer::start(move |_| match counted.fetch_add(1, Ordering::SeqCst) {
            0 => json(r#"{"error":"authorization_pending"}"#),
            1 => json(r#"{"error":"slow_down","interval":10}"#),
            _ => json(r#"{"access_token":"gho_device","token_type":"bearer","scope":"repo,read:org"}"#),
        });
        let flow = flow(&server);
        let mut answers = Vec::new();
        for _ in 0..3 {
            let (_keep, cancel) = cancel_pair();
            answers.push(smol::block_on(flow.poll(&code(), cancel)));
        }
        assert_eq!(
            answers,
            [Polled::Waiting, Polled::SlowDown, Polled::Granted(Tokens::lasting(Token::new("gho_device").unwrap()))]
        );
        assert_eq!(Polled::SlowDown.next_interval(Duration::from_secs(5)), Duration::from_secs(10));
        assert_eq!(Polled::Waiting.next_interval(Duration::from_secs(5)), Duration::from_secs(5));
        let form = form_of(&server.seen()[0]);
        assert!(form.contains(&("device_code".into(), "dc-1".into())));
        assert!(form.contains(&("grant_type".into(), "urn:ietf:params:oauth:grant-type:device_code".into())));
    }

    #[test]
    fn an_expiring_token_comes_with_its_refresh_token_and_the_address_to_renew_it_at() {
        let server = TestServer::start(|_| {
            json(r#"{"access_token":"gho_short","expires_in":28800,"refresh_token":"ghr_long","refresh_token_expires_in":15897600,"token_type":"bearer","scope":"repo"}"#)
        });
        let (_keep, cancel) = cancel_pair();
        let before = SystemTime::now();
        let Polled::Granted(tokens) = smol::block_on(flow(&server).poll(&code(), cancel)) else {
            panic!("no token granted");
        };
        let refresh = tokens.refresh.unwrap();
        assert_eq!((tokens.access.reveal(), refresh.token.reveal()), ("gho_short", "ghr_long"));
        assert!(refresh.access_expires_at >= before + Duration::from_secs(28800));
        assert!(refresh.access_expires_at <= SystemTime::now() + Duration::from_secs(28800));
        assert_eq!(refresh.token_url.as_str(), server.url("/login/oauth/access_token"));
        assert_eq!(refresh.client_id, "Iv1.sternegit");
    }

    #[test]
    fn an_expired_or_declined_code_ends_the_wait() {
        for (body, expected) in [
            (r#"{"error":"expired_token"}"#, Polled::Expired),
            (r#"{"error":"access_denied"}"#, Polled::Denied),
            (r#"{"error":"incorrect_client_credentials","error_description":"The client_id is wrong"}"#, Polled::Failed("The client_id is wrong".into())),
        ] {
            let server = TestServer::start(move |_| json(body));
            let (_keep, cancel) = cancel_pair();
            let polled = smol::block_on(flow(&server).poll(&code(), cancel));
            assert!(!polled.keeps_waiting());
            assert_eq!(polled, expected);
        }
    }

    #[test]
    fn a_server_that_answers_without_json_is_reported_with_its_status() {
        let server = TestServer::start(|_| Reply::status(503).body("down"));
        let (_keep, cancel) = cancel_pair();
        assert_eq!(
            smol::block_on(flow(&server).ask_for_code(cancel)),
            Err("The server refused with 503".to_owned())
        );
    }
}
