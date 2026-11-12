use std::path::PathBuf;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Map, Value, json};
use url::Url;

const AUTHORIZATION: &str = "Authorization";
const HEADER_NAME_SYMBOLS: &[u8] = b"!#$%&'*+-.^_`|~";
const KNOWN_FIELDS: [&str; 14] = [
    "id",
    "connector",
    "service",
    "server",
    "endpoints",
    "login",
    "name",
    "git_user",
    "sign_in",
    "scheme",
    "scopes",
    "ssh_key",
    "ssh_repositories",
    "clone_over_ssh",
];

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct AccountId(String);

impl AccountId {
    pub(crate) fn new(connector: &str, host: &str, user: &str) -> Option<Self> {
        Self::parse(&format!("{connector}:{host}:{user}"))
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (connector, rest) = text.split_once(':')?;
        let (host, user) = rest.rsplit_once(':')?;
        let plain = |part: &str, also: &[u8]| {
            !part.is_empty()
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || b"._-".contains(&byte) || also.contains(&byte)
                })
        };
        (plain(connector, b"") && plain(host, b":[]") && plain(user, b""))
            .then(|| Self(text.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TokenScheme {
    Bearer,
    Header { name: String },
    Basic { username: String },
}

impl TokenScheme {
    pub(crate) fn header_name(&self) -> &str {
        match self {
            TokenScheme::Header { name } => name,
            TokenScheme::Bearer | TokenScheme::Basic { .. } => AUTHORIZATION,
        }
    }

    pub(crate) fn header(&self, token: &str) -> (String, String) {
        let value = match self {
            TokenScheme::Bearer => format!("Bearer {token}"),
            TokenScheme::Header { .. } => token.to_owned(),
            TokenScheme::Basic { username } => {
                format!("Basic {}", STANDARD.encode(format!("{username}:{token}")))
            }
        };
        (self.header_name().to_owned(), value)
    }

    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        let text = |name: &str| value.get(name)?.as_str();
        match text("kind")? {
            "bearer" => Some(TokenScheme::Bearer),
            "header" => text("name")
                .filter(|name| is_header_name(name))
                .map(|name| TokenScheme::Header {
                    name: name.to_owned(),
                }),
            "basic" => text("username")
                .filter(|username| !username.contains(':'))
                .map(|username| TokenScheme::Basic {
                    username: username.to_owned(),
                }),
            _ => None,
        }
    }

    fn to_json(&self) -> Value {
        match self {
            TokenScheme::Bearer => json!({ "kind": "bearer" }),
            TokenScheme::Header { name } => json!({ "kind": "header", "name": name }),
            TokenScheme::Basic { username } => json!({ "kind": "basic", "username": username }),
        }
    }
}

pub(crate) fn is_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || HEADER_NAME_SYMBOLS.contains(&byte))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SignInMethod {
    Device,
    PastedToken,
    GitCredential,
}

impl SignInMethod {
    fn name(self) -> &'static str {
        match self {
            SignInMethod::Device => "device",
            SignInMethod::PastedToken => "pasted_token",
            SignInMethod::GitCredential => "git_credential",
        }
    }

    pub(crate) fn named(name: &str) -> Option<Self> {
        [
            SignInMethod::Device,
            SignInMethod::PastedToken,
            SignInMethod::GitCredential,
        ]
        .into_iter()
        .find(|method| method.name() == name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Endpoint {
    pub(crate) name: String,
    pub(crate) url: Url,
    pub(crate) carries_token: bool,
}

impl Endpoint {
    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            name: value.get("name")?.as_str()?.to_owned(),
            url: Url::parse(value.get("url")?.as_str()?).ok()?,
            carries_token: value.get("token").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    fn to_json(&self) -> Value {
        json!({ "name": self.name, "url": self.url.as_str(), "token": self.carries_token })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Account {
    pub(crate) id: AccountId,
    pub(crate) connector: String,
    pub(crate) service: String,
    pub(crate) server: Url,
    pub(crate) endpoints: Vec<Endpoint>,
    pub(crate) login: String,
    pub(crate) name: Option<String>,
    pub(crate) git_user: String,
    pub(crate) sign_in: SignInMethod,
    pub(crate) scheme: TokenScheme,
    pub(crate) scopes: Vec<String>,
    pub(crate) ssh_key: Option<String>,
    pub(crate) ssh_repositories: Vec<PathBuf>,
    pub(crate) clone_over_ssh: bool,
    pub(crate) others: Map<String, Value>,
}

impl Account {
    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        let fields = value.as_object()?;
        let text = |name: &str| fields.get(name).and_then(Value::as_str);
        let owned = |name: &str| text(name).map(str::to_owned);
        Some(Self {
            id: AccountId::parse(text("id")?)?,
            connector: owned("connector")?,
            service: owned("service")?,
            server: Url::parse(text("server")?).ok()?,
            endpoints: fields
                .get("endpoints")?
                .as_array()?
                .iter()
                .map(Endpoint::from_json)
                .collect::<Option<_>>()?,
            login: owned("login")?,
            name: owned("name"),
            git_user: owned("git_user").filter(|user| !user.is_empty())?,
            sign_in: SignInMethod::named(text("sign_in")?)?,
            scheme: TokenScheme::from_json(fields.get("scheme")?)?,
            scopes: fields
                .get("scopes")
                .and_then(Value::as_array)
                .map(|scopes| scopes.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default(),
            ssh_key: owned("ssh_key"),
            ssh_repositories: fields
                .get("ssh_repositories")
                .and_then(Value::as_array)
                .map(|paths| paths.iter().filter_map(Value::as_str).map(PathBuf::from).collect())
                .unwrap_or_default(),
            clone_over_ssh: fields.get("clone_over_ssh").and_then(Value::as_bool).unwrap_or(false),
            others: fields
                .iter()
                .filter(|(key, _)| !KNOWN_FIELDS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        })
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut fields = Map::new();
        fields.insert("id".into(), self.id.as_str().into());
        fields.insert("connector".into(), self.connector.as_str().into());
        fields.insert("service".into(), self.service.as_str().into());
        fields.insert("server".into(), self.server.as_str().into());
        fields.insert(
            "endpoints".into(),
            self.endpoints.iter().map(Endpoint::to_json).collect(),
        );
        fields.insert("login".into(), self.login.as_str().into());
        if let Some(name) = &self.name {
            fields.insert("name".into(), name.as_str().into());
        }
        fields.insert("git_user".into(), self.git_user.as_str().into());
        fields.insert("sign_in".into(), self.sign_in.name().into());
        fields.insert("scheme".into(), self.scheme.to_json());
        fields.insert("scopes".into(), self.scopes.clone().into());
        if let Some(ssh_key) = &self.ssh_key {
            fields.insert("ssh_key".into(), ssh_key.as_str().into());
        }
        if !self.ssh_repositories.is_empty() {
            let paths = self.ssh_repositories.iter().map(|path| path.to_string_lossy().into_owned());
            fields.insert("ssh_repositories".into(), paths.collect());
        }
        if self.clone_over_ssh {
            fields.insert("clone_over_ssh".into(), true.into());
        }
        fields.extend(self.others.clone());
        Value::Object(fields)
    }

    pub(crate) fn covers(&self, path: &str) -> bool {
        let mut parts = path.trim_matches('/').split('/');
        self.server
            .path_segments()
            .into_iter()
            .flatten()
            .filter(|segment| !segment.is_empty())
            .all(|segment| parts.next().is_some_and(|part| part.eq_ignore_ascii_case(segment)))
    }
}

pub(crate) fn authority(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

#[cfg(test)]
pub(crate) mod fixtures {
    use serde_json::Map;
    use url::Url;

    use super::{Account, AccountId, Endpoint, SignInMethod, TokenScheme};

    pub(crate) fn account(server: &str, user: &str, endpoints: &[(&str, &str, bool)]) -> Account {
        let server = Url::parse(server).unwrap();
        let host = super::authority(&server).unwrap();
        Account {
            id: AccountId::new("github", &host, user).unwrap(),
            connector: "github".into(),
            service: "github.com".into(),
            server,
            endpoints: endpoints
                .iter()
                .map(|(name, url, carries_token)| Endpoint {
                    name: (*name).into(),
                    url: Url::parse(url).unwrap(),
                    carries_token: *carries_token,
                })
                .collect(),
            login: format!("login-{user}"),
            name: None,
            git_user: "x-access-token".into(),
            sign_in: SignInMethod::PastedToken,
            scheme: TokenScheme::Bearer,
            scopes: vec!["repo".into()],
            ssh_key: None,
            ssh_repositories: Vec::new(),
            clone_over_ssh: false,
            others: Map::new(),
        }
    }

    pub(crate) fn github(user: &str) -> Account {
        account(
            "https://github.com",
            user,
            &[
                ("api", "https://api.github.com/", true),
                ("graphql", "https://api.github.com/graphql", true),
                ("images", "https://avatars.githubusercontent.com/", false),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Account, AccountId, TokenScheme, fixtures, is_header_name};

    #[test]
    fn account_ids_join_connector_host_and_user_and_allow_a_port() {
        let id = AccountId::new("github", "ghe.example.com:8443", "583231").unwrap();
        assert_eq!(id.as_str(), "github:ghe.example.com:8443:583231");
        assert_eq!(AccountId::parse(id.as_str()), Some(id));
    }

    #[test]
    fn account_ids_refuse_anything_a_shell_or_a_file_name_could_misread() {
        for text in ["github:host", "gith ub:host:1", "github:host:1'", "github:host:", ":host:1", "a:b:c/d"] {
            assert_eq!(AccountId::parse(text), None, "{text}");
        }
    }

    #[test]
    fn each_scheme_puts_the_token_in_its_own_header() {
        assert_eq!(
            TokenScheme::Bearer.header("gho_x"),
            ("Authorization".into(), "Bearer gho_x".into())
        );
        let gitlab = TokenScheme::Header { name: "PRIVATE-TOKEN".into() };
        assert_eq!(gitlab.header("glpat"), ("PRIVATE-TOKEN".into(), "glpat".into()));
        let azure = TokenScheme::Basic { username: String::new() };
        assert_eq!(azure.header("pat"), ("Authorization".into(), "Basic OnBhdA==".into()));
    }

    #[test]
    fn a_scheme_naming_an_impossible_header_is_refused() {
        assert_eq!(
            TokenScheme::from_json(&json!({ "kind": "header", "name": "Bad Header" })),
            None
        );
        assert_eq!(
            TokenScheme::from_json(&json!({ "kind": "basic", "username": "a:b" })),
            None
        );
        assert!(is_header_name("PRIVATE-TOKEN"));
        assert!(!is_header_name("X:Y"));
    }

    #[test]
    fn an_account_on_a_whole_server_covers_every_path_on_it() {
        let account = fixtures::github("1");
        for path in ["octo/app", "/octo", "", "anything/at/all/"] {
            assert!(account.covers(path), "{path}");
        }
    }

    #[test]
    fn an_account_on_a_server_path_covers_only_paths_under_it() {
        let account = fixtures::account("https://dev.azure.com/contoso/", "1", &[]);
        for path in ["contoso/Web", "/Contoso/Web/_git/app", "contoso"] {
            assert!(account.covers(path), "{path}");
        }
        for path in ["fabrikam/Web", "contosos/Web", ""] {
            assert!(!account.covers(path), "{path}");
        }
        let collection = fixtures::account("https://devops.example.com:8443/tfs/DefaultCollection", "1", &[]);
        assert!(collection.covers("tfs/defaultcollection/Game/_git/client"));
        assert!(!collection.covers("tfs/Other/Game"));
    }

    #[test]
    fn an_account_reads_back_what_it_wrote_including_fields_it_does_not_know() {
        let mut account = fixtures::github("583231");
        account.name = Some("Octo Cat".into());
        account.others.insert("from_a_newer_sternegit".into(), json!({ "a": 1 }));
        account.ssh_key = Some("C:/keys/github-github.com-583231_ed25519".into());
        account.ssh_repositories = vec!["E:/Projects/sternegit".into()];
        account.clone_over_ssh = true;
        assert_eq!(Account::from_json(&account.to_json()), Some(account));
    }

    #[test]
    fn an_account_missing_what_it_needs_is_skipped() {
        let mut written = fixtures::github("1").to_json();
        written.as_object_mut().unwrap().remove("git_user");
        assert_eq!(Account::from_json(&written), None);
    }
}
