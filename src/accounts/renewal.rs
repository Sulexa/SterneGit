use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use smol::lock::Mutex;

use super::{Account, AccountBook, AccountId, Refresh, Secrets, Token, Tokens, oauth, os_secrets};
use crate::{
    git::CredentialScope,
    net::{
        curl::{Curl, Transport},
        request::NetError,
        services::cancel_pair,
    },
};

pub(crate) const BEFORE_A_CALL: Duration = Duration::from_secs(2 * 60);
const BEFORE_GIT: Duration = Duration::from_secs(10 * 60);
const REFRESH_GRANT: &str = "refresh_token";
const REFUSALS: [&str; 2] = ["bad_refresh_token", "invalid_grant"];

static ONE_RENEWAL_AT_A_TIME: Mutex<()> = Mutex::new(());

#[derive(Clone)]
pub(crate) struct Renewer {
    secrets: Arc<dyn Secrets>,
    transport: Arc<dyn Transport>,
}

enum Renewed {
    Granted(Tokens),
    Refused,
    Failed(String),
}

impl Renewer {
    pub(crate) fn new(secrets: Arc<dyn Secrets>, transport: Arc<dyn Transport>) -> Self {
        Self { secrets, transport }
    }

    pub(crate) async fn token(&self, account: &Account, margin: Duration) -> Result<Token, NetError> {
        let kept = self.kept(&account.id)?;
        if !kept.is_due(margin, SystemTime::now()) {
            return Ok(kept.access);
        }
        let _turn = ONE_RENEWAL_AT_A_TIME.lock().await;
        let kept = self.kept(&account.id)?;
        match kept.is_due(margin, SystemTime::now()) {
            true => self.renew(account, kept).await,
            false => Ok(kept.access),
        }
    }

    pub(crate) async fn after_refusal(&self, account: &Account, refused: &Token) -> Result<Option<Token>, NetError> {
        let _turn = ONE_RENEWAL_AT_A_TIME.lock().await;
        let kept = self.kept(&account.id)?;
        if &kept.access != refused {
            return Ok(Some(kept.access));
        }
        match kept.refresh {
            Some(_) => self.renew(account, kept).await.map(Some),
            None => Ok(None),
        }
    }

    fn kept(&self, account: &AccountId) -> Result<Tokens, NetError> {
        self.secrets.read(account).ok().flatten().ok_or(NetError::NotSignedIn)
    }

    async fn renew(&self, account: &Account, kept: Tokens) -> Result<Token, NetError> {
        let Some(refresh) = kept.refresh else {
            return Ok(kept.access);
        };
        match refreshed(&self.transport, &refresh).await {
            Renewed::Granted(tokens) => {
                self.secrets
                    .write(&account.id, &account.login, &tokens)
                    .map_err(|error| NetError::Unreachable(format!("SterneGit couldn't keep the renewed token: {error}")))?;
                Ok(tokens.access)
            }
            Renewed::Refused => {
                let now_kept = self.kept(&account.id)?;
                let renewed_elsewhere = now_kept
                    .refresh
                    .as_ref()
                    .is_some_and(|newer| newer.token != refresh.token);
                match renewed_elsewhere {
                    true => Ok(now_kept.access),
                    false => Err(NetError::NotSignedIn),
                }
            }
            Renewed::Failed(problem) => Err(NetError::Unreachable(problem)),
        }
    }
}

fn refreshed(transport: &Arc<dyn Transport>, refresh: &Refresh) -> impl Future<Output = Renewed> + Send + 'static {
    let (keep_asking, cancel) = cancel_pair();
    let client = oauth::Client::from(refresh);
    let asked = oauth::post(
        transport,
        &client.token_url,
        &[
            ("client_id", &client.id),
            ("grant_type", REFRESH_GRANT),
            ("refresh_token", refresh.token.reveal()),
        ],
        cancel,
    );
    async move {
        let answer = asked.await;
        drop(keep_asking);
        let answer = match answer {
            Ok(answer) => answer,
            Err(problem) => return Renewed::Failed(problem),
        };
        if let Some(tokens) = client.tokens_in(&answer, SystemTime::now()) {
            return Renewed::Granted(tokens);
        }
        match answer["error"].as_str() {
            Some(error) if REFUSALS.contains(&error) => Renewed::Refused,
            _ => Renewed::Failed(oauth::refusal(&answer, "The server renewed no token")),
        }
    }
}

pub(crate) fn renew_for_git(scopes: &[CredentialScope]) {
    let secrets = os_secrets();
    let now = SystemTime::now();
    let due: Vec<AccountId> = scopes
        .iter()
        .filter_map(|scope| AccountId::parse(&scope.account))
        .filter(|id| {
            secrets
                .read(id)
                .ok()
                .flatten()
                .is_some_and(|tokens| tokens.is_due(BEFORE_GIT, now))
        })
        .collect();
    if due.is_empty() {
        return;
    }
    let book = AccountBook::load();
    let renewer = Renewer::new(secrets, Arc::new(Curl::system()));
    for account in due.iter().filter_map(|id| book.account(id)) {
        let _ = futures::executor::block_on(renewer.token(account, BEFORE_GIT));
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, SystemTime},
    };

    use futures::{FutureExt as _, future::BoxFuture};

    use super::{BEFORE_A_CALL, Renewer};
    use crate::{
        accounts::{MemorySecrets, Secrets as _, Token, Tokens, fixtures, renewable},
        net::{
            curl::{Curl, Outgoing, Transport},
            request::{ApiResponse, NetError},
            services::Cancel,
        },
        test_server::{Reply, Seen, TestServer},
    };

    const RENEWED: &str = r#"{"access_token":"gho_new","expires_in":28800,"refresh_token":"ghr_new","refresh_token_expires_in":15897600,"token_type":"bearer"}"#;

    fn json(body: &str) -> Reply {
        Reply::status(200).header("Content-Type", "application/json").body(body)
    }

    fn form_of(seen: &Seen) -> Vec<(String, String)> {
        url::form_urlencoded::parse(&seen.body).into_owned().collect()
    }

    fn expiring_in(seconds: u64, server: &TestServer) -> Tokens {
        renewable(
            "gho_old",
            "ghr_old",
            SystemTime::now() + Duration::from_secs(seconds),
            &server.url("/login/oauth/access_token"),
        )
    }

    fn renewer(server_holding: Tokens) -> (Renewer, Arc<MemorySecrets>) {
        let secrets = Arc::new(MemorySecrets::keeping(&fixtures::github("1").id, server_holding));
        (Renewer::new(secrets.clone(), Arc::new(Curl::ignoring_git_config())), secrets)
    }

    fn kept(secrets: &MemorySecrets) -> Tokens {
        secrets.read(&fixtures::github("1").id).unwrap().unwrap()
    }

    #[test]
    fn a_token_far_from_expiring_is_used_without_asking_the_server() {
        let server = TestServer::start(|_| json(RENEWED));
        let (renewer, _) = renewer(expiring_in(3600, &server));
        let token = smol::block_on(renewer.token(&fixtures::github("1"), BEFORE_A_CALL)).unwrap();
        assert_eq!(token.reveal(), "gho_old");
        assert!(server.seen().is_empty());
    }

    #[test]
    fn a_token_about_to_expire_is_renewed_and_the_rotated_pair_kept() {
        let server = TestServer::start(|_| json(RENEWED));
        let (renewer, secrets) = renewer(expiring_in(60, &server));
        let token = smol::block_on(renewer.token(&fixtures::github("1"), BEFORE_A_CALL)).unwrap();
        assert_eq!(token.reveal(), "gho_new");
        let kept = kept(&secrets);
        let refresh = kept.refresh.unwrap();
        assert_eq!((kept.access.reveal(), refresh.token.reveal()), ("gho_new", "ghr_new"));
        assert!(refresh.access_expires_at > SystemTime::now() + Duration::from_secs(28000));
        let seen = &server.seen()[0];
        assert_eq!((seen.method.as_str(), seen.target.as_str()), ("POST", "/login/oauth/access_token"));
        assert_eq!(
            form_of(seen),
            [
                ("client_id".into(), "Ov23li.sternegit".into()),
                ("grant_type".into(), "refresh_token".into()),
                ("refresh_token".into(), "ghr_old".into()),
            ]
        );
    }

    #[test]
    fn a_refused_refresh_token_signs_the_account_out() {
        let server = TestServer::start(|_| json(r#"{"error":"bad_refresh_token","error_description":"The refresh token passed is incorrect or expired."}"#));
        let (renewer, _) = renewer(expiring_in(0, &server));
        let renewed = smol::block_on(renewer.token(&fixtures::github("1"), BEFORE_A_CALL));
        assert_eq!(renewed, Err(NetError::NotSignedIn));
    }

    #[test]
    fn a_server_that_cannot_be_reached_is_not_mistaken_for_a_sign_out() {
        let server = TestServer::start(|_| Reply::status(502).body("bad gateway"));
        let (renewer, _) = renewer(expiring_in(0, &server));
        let renewed = smol::block_on(renewer.token(&fixtures::github("1"), BEFORE_A_CALL));
        assert_eq!(renewed, Err(NetError::Unreachable("The server refused with 502".into())));
    }

    #[test]
    fn a_lasting_token_is_never_renewed_and_a_refusal_of_it_stands() {
        let (renewer, _) = {
            let secrets = Arc::new(MemorySecrets::holding(&fixtures::github("1").id, "gho_lasting"));
            (Renewer::new(secrets.clone(), Arc::new(Curl::ignoring_git_config())), secrets)
        };
        let account = fixtures::github("1");
        let token = smol::block_on(renewer.token(&account, BEFORE_A_CALL)).unwrap();
        assert_eq!(token.reveal(), "gho_lasting");
        assert_eq!(smol::block_on(renewer.after_refusal(&account, &token)), Ok(None));
    }

    #[test]
    fn a_refused_access_token_is_renewed_even_before_its_expiry() {
        let server = TestServer::start(|_| json(RENEWED));
        let (renewer, _) = renewer(expiring_in(3600, &server));
        let refused = Token::new("gho_old").unwrap();
        let renewed = smol::block_on(renewer.after_refusal(&fixtures::github("1"), &refused)).unwrap();
        assert_eq!(renewed.map(|token| token.reveal().to_owned()), Some("gho_new".into()));
    }

    #[test]
    fn a_refusal_of_a_token_already_renewed_hands_back_the_new_one_without_asking() {
        let server = TestServer::start(|_| json(RENEWED));
        let (renewer, _) = renewer(expiring_in(3600, &server));
        let stale = Token::new("gho_before_old").unwrap();
        let handed = smol::block_on(renewer.after_refusal(&fixtures::github("1"), &stale)).unwrap();
        assert_eq!(handed.map(|token| token.reveal().to_owned()), Some("gho_old".into()));
        assert!(server.seen().is_empty());
    }

    struct OtherWindow {
        secrets: Arc<MemorySecrets>,
        newer: Tokens,
        asked: AtomicUsize,
    }

    impl Transport for OtherWindow {
        fn exchange(&self, _: Outgoing, _: Cancel) -> BoxFuture<'static, Result<ApiResponse, NetError>> {
            self.asked.fetch_add(1, Ordering::SeqCst);
            self.secrets.write(&fixtures::github("1").id, "login", &self.newer).unwrap();
            let refused = ApiResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"error":"bad_refresh_token"}"#.to_vec(),
            };
            futures::future::ready(Ok(refused)).boxed()
        }
    }

    #[test]
    fn a_refresh_token_another_window_rotated_first_leads_to_that_windows_tokens() {
        let server = TestServer::start(|_| json(RENEWED));
        let secrets = Arc::new(MemorySecrets::keeping(&fixtures::github("1").id, expiring_in(0, &server)));
        let newer = renewable("gho_theirs", "ghr_theirs", SystemTime::now() + Duration::from_secs(28800), &server.url("/t"));
        let other = Arc::new(OtherWindow {
            secrets: secrets.clone(),
            newer,
            asked: AtomicUsize::new(0),
        });
        let renewer = Renewer::new(secrets.clone(), other.clone());
        let token = smol::block_on(renewer.token(&fixtures::github("1"), BEFORE_A_CALL)).unwrap();
        assert_eq!(token.reveal(), "gho_theirs");
        assert_eq!(other.asked.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn calls_that_find_the_token_due_together_renew_it_once() {
        let renewals = Arc::new(Mutex::new(0));
        let counted = renewals.clone();
        let server = TestServer::start(move |_| {
            *counted.lock().unwrap() += 1;
            json(RENEWED).after(Duration::from_millis(50))
        });
        let (renewer, _) = renewer(expiring_in(0, &server));
        let account = fixtures::github("1");
        let tokens = smol::block_on(futures::future::join_all(
            (0..4).map(|_| renewer.token(&account, BEFORE_A_CALL)),
        ));
        assert!(tokens.iter().all(|token| token.as_ref().unwrap().reveal() == "gho_new"));
        assert_eq!(*renewals.lock().unwrap(), 1);
    }
}
