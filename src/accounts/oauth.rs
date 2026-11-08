use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use serde_json::Value;
use url::{Url, form_urlencoded};

use super::{Refresh, Token, Tokens};
use crate::net::{
    curl::{Outgoing, Transport},
    request::Method,
    services::Cancel,
};

const FORM: &str = "application/x-www-form-urlencoded";
const JSON: &str = "application/json";
const ANSWER_WITHIN: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Client {
    pub(crate) id: String,
    pub(crate) token_url: Url,
}

impl Client {
    pub(crate) fn tokens_in(&self, answer: &Value, now: SystemTime) -> Option<Tokens> {
        let access = Token::new(answer["access_token"].as_str()?)?;
        let refresh = answer["refresh_token"]
            .as_str()
            .and_then(Token::new)
            .zip(answer["expires_in"].as_u64())
            .map(|(token, seconds)| Refresh {
                token,
                access_expires_at: now + Duration::from_secs(seconds),
                token_url: self.token_url.clone(),
                client_id: self.id.clone(),
            });
        Some(Tokens { access, refresh })
    }
}

impl From<&Refresh> for Client {
    fn from(refresh: &Refresh) -> Self {
        Self {
            id: refresh.client_id.clone(),
            token_url: refresh.token_url.clone(),
        }
    }
}

pub(crate) fn post(
    transport: &Arc<dyn Transport>,
    url: &Url,
    pairs: &[(&str, &str)],
    cancel: Cancel,
) -> impl Future<Output = Result<Value, String>> + Send + use<> {
    let form = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    let outgoing = Outgoing {
        method: Method::Post,
        url: url.clone(),
        headers: vec![
            ("Accept".to_owned(), JSON.to_owned()),
            ("Content-Type".to_owned(), FORM.to_owned()),
        ],
        body: Some(form.into_bytes()),
        timeout: ANSWER_WITHIN,
    };
    let exchange = transport.exchange(outgoing, cancel);
    async move {
        let response = exchange.await.map_err(|error| error.to_string())?;
        serde_json::from_slice(&response.body).map_err(|_| match response.status {
            200..=299 => "The server's answer can't be read".to_owned(),
            status => format!("The server refused with {status}"),
        })
    }
}

pub(crate) fn refusal(answer: &Value, otherwise: &str) -> String {
    answer["error_description"]
        .as_str()
        .or(answer["error"].as_str())
        .unwrap_or(otherwise)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use serde_json::json;
    use url::Url;

    use super::Client;
    use crate::accounts::{Token, Tokens};

    fn client() -> Client {
        Client {
            id: "Ov23li.sternegit".into(),
            token_url: Url::parse("https://github.com/login/oauth/access_token").unwrap(),
        }
    }

    #[test]
    fn a_token_without_a_refresh_token_lasts() {
        let answer = json!({ "access_token": "gho_lasting", "token_type": "bearer", "scope": "repo" });
        assert_eq!(
            client().tokens_in(&answer, SystemTime::now()),
            Some(Tokens::lasting(Token::new("gho_lasting").unwrap()))
        );
    }

    #[test]
    fn an_expiring_token_keeps_how_to_renew_it() {
        let now = SystemTime::now();
        let answer = json!({
            "access_token": "gho_short", "expires_in": 28800,
            "refresh_token": "ghr_long", "refresh_token_expires_in": 15897600,
            "token_type": "bearer", "scope": "repo"
        });
        let tokens = client().tokens_in(&answer, now).unwrap();
        let refresh = tokens.refresh.unwrap();
        assert_eq!(tokens.access.reveal(), "gho_short");
        assert_eq!(refresh.token.reveal(), "ghr_long");
        assert_eq!(refresh.access_expires_at, now + Duration::from_secs(28800));
        assert_eq!(Client::from(&refresh), client());
    }

    #[test]
    fn an_answer_without_a_usable_token_grants_nothing() {
        assert_eq!(client().tokens_in(&json!({ "error": "authorization_pending" }), SystemTime::now()), None);
        assert_eq!(client().tokens_in(&json!({ "access_token": "with space" }), SystemTime::now()), None);
    }
}
