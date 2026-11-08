use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use url::Url;

use super::Token;

const ACCESS_TOKEN: &str = "access_token";
const REFRESH_TOKEN: &str = "refresh_token";
const EXPIRES_AT: &str = "expires_at";
const TOKEN_URL: &str = "token_url";
const CLIENT_ID: &str = "client_id";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tokens {
    pub(crate) access: Token,
    pub(crate) refresh: Option<Refresh>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Refresh {
    pub(crate) token: Token,
    pub(crate) access_expires_at: SystemTime,
    pub(crate) token_url: Url,
    pub(crate) client_id: String,
}

impl Tokens {
    pub(crate) fn lasting(access: Token) -> Self {
        Self {
            access,
            refresh: None,
        }
    }

    pub(crate) fn is_due(&self, margin: Duration, now: SystemTime) -> bool {
        self.refresh
            .as_ref()
            .is_some_and(|refresh| refresh.access_expires_at <= now + margin)
    }

    pub(crate) fn to_blob(&self) -> String {
        let Some(refresh) = &self.refresh else {
            return self.access.reveal().to_owned();
        };
        let mut fields = Map::new();
        fields.insert(ACCESS_TOKEN.into(), self.access.reveal().into());
        fields.insert(REFRESH_TOKEN.into(), refresh.token.reveal().into());
        fields.insert(EXPIRES_AT.into(), unix_seconds(refresh.access_expires_at).into());
        fields.insert(TOKEN_URL.into(), refresh.token_url.as_str().into());
        fields.insert(CLIENT_ID.into(), refresh.client_id.as_str().into());
        Value::Object(fields).to_string()
    }

    pub(crate) fn from_blob(text: &str) -> Option<Self> {
        let text = text.trim();
        if !text.starts_with('{') {
            return Token::new(text).map(Self::lasting);
        }
        let fields: Value = serde_json::from_str(text).ok()?;
        let text_of = |name: &str| fields.get(name)?.as_str();
        Some(Self {
            access: Token::new(text_of(ACCESS_TOKEN)?)?,
            refresh: Some(Refresh {
                token: Token::new(text_of(REFRESH_TOKEN)?)?,
                access_expires_at: UNIX_EPOCH + Duration::from_secs(fields.get(EXPIRES_AT)?.as_u64()?),
                token_url: Url::parse(text_of(TOKEN_URL)?).ok()?,
                client_id: text_of(CLIENT_ID)?.to_owned(),
            }),
        })
    }
}

fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

#[cfg(test)]
pub(crate) mod fixtures {
    use std::time::SystemTime;

    use url::Url;

    use super::{Refresh, Tokens};
    use crate::accounts::Token;

    pub(crate) fn renewable(access: &str, refresh: &str, access_expires_at: SystemTime, token_url: &str) -> Tokens {
        Tokens {
            access: Token::new(access).unwrap(),
            refresh: Some(Refresh {
                token: Token::new(refresh).unwrap(),
                access_expires_at,
                token_url: Url::parse(token_url).unwrap(),
                client_id: "Ov23li.sternegit".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::{Tokens, fixtures::renewable};
    use crate::accounts::Token;

    const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

    #[test]
    fn a_lasting_token_is_kept_bare_so_entries_from_before_renewal_still_read() {
        let lasting = Tokens::lasting(Token::new("gho_lasting").unwrap());
        assert_eq!(lasting.to_blob(), "gho_lasting");
        assert_eq!(Tokens::from_blob(" gho_lasting\r\n"), Some(lasting));
    }

    #[test]
    fn a_renewable_pair_reads_back_whole() {
        let tokens = renewable("gho_access", "ghr_refresh", UNIX_EPOCH + Duration::from_secs(1_794_000_000), TOKEN_URL);
        assert_eq!(Tokens::from_blob(&tokens.to_blob()), Some(tokens));
    }

    #[test]
    fn a_renewable_pair_is_due_once_its_access_token_expires_within_the_margin() {
        let expires_at = UNIX_EPOCH + Duration::from_secs(10_000);
        let tokens = renewable("gho_access", "ghr_refresh", expires_at, TOKEN_URL);
        let margin = Duration::from_secs(120);
        assert!(!tokens.is_due(margin, expires_at - Duration::from_secs(121)));
        assert!(tokens.is_due(margin, expires_at - Duration::from_secs(120)));
        assert!(tokens.is_due(margin, expires_at + Duration::from_secs(3600)));
        assert!(!Tokens::lasting(Token::new("gho_x").unwrap()).is_due(margin, expires_at));
    }

    #[test]
    fn a_damaged_entry_reads_as_nothing_rather_than_as_a_strange_token() {
        assert_eq!(Tokens::from_blob(r#"{"access_token":"gho_a","refresh_token":"ghr_b"}"#), None);
        assert_eq!(Tokens::from_blob("{not json"), None);
    }

    #[test]
    fn neither_token_is_ever_printed() {
        let tokens = renewable("gho_access", "ghr_refresh", UNIX_EPOCH, TOKEN_URL);
        let printed = format!("{tokens:?}");
        assert!(!printed.contains("gho_access") && !printed.contains("ghr_refresh"), "{printed}");
    }
}
