use std::{collections::BTreeMap, sync::Arc};

use fastgit_protocol::{Failure, Identity};
use serde_json::Map;
use url::Url;

use super::{
    CallError, Forge, Started,
    manifest::{Field, FieldKind, Service},
    template,
};
use crate::{
    accounts::{Account, AccountId, Endpoint, SignInMethod, Token, authority},
    net::{
        gateway::Gateway,
        services::{ConnectorId, HostServices},
    },
};

const SIGNING_IN: &str = "signing-in";
const TYPED_SCHEMES: [&str; 2] = ["https", "http"];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Draft {
    pub(crate) account: Account,
    pub(crate) urls: Vec<(String, Url)>,
    pub(crate) client_id: Option<String>,
    pub(crate) scopes: Vec<String>,
}

impl Draft {
    pub(crate) fn url(&self, name: &str) -> Option<&Url> {
        self.urls
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, url)| url)
    }
}

pub(crate) fn draft(
    connector: &ConnectorId,
    service: &Service,
    method: SignInMethod,
    typed: &BTreeMap<String, String>,
) -> Result<Draft, String> {
    let sign_in = service
        .sign_in
        .iter()
        .find(|sign_in| sign_in.method == method)
        .ok_or_else(|| format!("{} doesn't offer this way to sign in", service.label))?;
    let values = service
        .fields
        .iter()
        .map(|field| Ok((field.id.clone(), field_value(field, typed.get(&field.id))?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let endpoints = service
        .endpoints
        .iter()
        .map(|(name, template)| {
            Ok(Endpoint {
                name: name.clone(),
                url: expanded_url(template, &values, &[])?,
                carries_token: service.token_endpoints.contains(name),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let named: Vec<(String, Url)> = endpoints
        .iter()
        .map(|endpoint| (endpoint.name.clone(), endpoint.url.clone()))
        .collect();
    let urls = sign_in
        .urls
        .iter()
        .map(|(name, template)| Ok((name.clone(), expanded_url(template, &values, &named)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let server = server_of(service, &values)?;
    let host = authority(&server).ok_or_else(|| format!("{server} names no server"))?;
    let id = AccountId::new(&connector.0, &host, SIGNING_IN)
        .ok_or_else(|| format!("{host} can't name an account"))?;
    Ok(Draft {
        account: Account {
            id,
            connector: connector.0.clone(),
            service: service.id.clone(),
            server,
            endpoints,
            login: String::new(),
            name: None,
            git_user: service.git_user.clone(),
            sign_in: method,
            scheme: sign_in.scheme.clone(),
            scopes: Vec::new(),
            ssh_key: None,
            ssh_repositories: Vec::new(),
            clone_over_ssh: false,
            others: Map::new(),
        },
        urls,
        client_id: sign_in.client_id.clone(),
        scopes: sign_in.scopes.clone(),
    })
}

pub(crate) fn signed_in(mut account: Account, identity: &Identity) -> Option<Account> {
    let host = authority(&account.server)?;
    account.id = AccountId::new(&account.connector, &host, &identity.id)?;
    account.login = identity.login.clone();
    account.name = identity.name.clone().filter(|name| !name.trim().is_empty());
    account.scopes = identity.scopes.clone();
    Some(account)
}

pub(crate) async fn identify(
    started: Arc<Started>,
    gateway: Arc<Gateway>,
    draft: Account,
    token: Token,
) -> Result<Account, CallError> {
    let lent = gateway.lend(draft.clone(), token);
    let host: Arc<dyn HostServices> = gateway;
    let forge = Forge::new(started, host, draft.id.clone(), None);
    let identity = forge.who_am_i().await;
    drop(lent);
    signed_in(draft, &identity?).ok_or_else(|| {
        CallError::Failed(Failure::Protocol {
            message: "The connector named no user FastGit can keep".to_owned(),
        })
    })
}

fn field_value(field: &Field, typed: Option<&String>) -> Result<String, String> {
    let typed = typed.map(|text| text.trim()).unwrap_or_default();
    if typed.is_empty() {
        return Err(format!("{} is missing", field.label));
    }
    match field.kind {
        FieldKind::Text => Ok(typed.to_owned()),
        FieldKind::Url => typed_server(typed)
            .map(|url| url.as_str().trim_end_matches('/').to_owned())
            .ok_or_else(|| format!("{typed} isn't a server address")),
    }
}

fn typed_server(typed: &str) -> Option<Url> {
    let typed = typed.trim();
    let with_scheme = match typed.contains("://") {
        true => typed.to_owned(),
        false => format!("https://{typed}"),
    };
    let url = Url::parse(&with_scheme).ok()?;
    let plain = TYPED_SCHEMES.contains(&url.scheme())
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none();
    plain.then_some(url)
}

fn expanded_url(
    template: &str,
    values: &BTreeMap<String, String>,
    endpoints: &[(String, Url)],
) -> Result<Url, String> {
    let expanded = template::expand(template, |name| {
        values.get(name).cloned().or_else(|| {
            endpoints
                .iter()
                .find(|(endpoint, _)| endpoint == name)
                .map(|(_, url)| url.to_string())
        })
    })?;
    Url::parse(&expanded).map_err(|error| format!("{expanded} isn't an address: {error}"))
}

fn server_of(service: &Service, values: &BTreeMap<String, String>) -> Result<Url, String> {
    let typed = service
        .fields
        .iter()
        .find(|field| field.kind == FieldKind::Url)
        .and_then(|field| values.get(&field.id));
    let text = match typed {
        Some(server) => format!("{server}/"),
        None => service
            .remotes
            .iter()
            .flat_map(|remote| &remote.hosts)
            .find(|host| !host.contains('{'))
            .map(|host| format!("https://{host}/"))
            .ok_or_else(|| format!("{} names no server", service.label))?,
    };
    Url::parse(&text).map_err(|error| format!("{text} isn't an address: {error}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fastgit_protocol::Identity;

    use super::{draft, signed_in, typed_server};
    use crate::{
        accounts::{SignInMethod, TokenScheme},
        connect::manifest::{Manifest, samples},
    };

    fn github() -> Manifest {
        Manifest::parse(samples::github().to_string().as_bytes()).unwrap()
    }

    fn typed(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())).collect()
    }

    fn identity(id: &str, login: &str) -> Identity {
        Identity {
            id: id.into(),
            login: login.into(),
            name: Some("Octo Cat".into()),
            avatar_url: None,
            scopes: vec!["repo".into()],
        }
    }

    #[test]
    fn a_github_com_draft_takes_its_server_from_the_remotes_and_its_token_endpoints_from_the_manifest() {
        let manifest = github();
        let drafted = draft(&manifest.id, &manifest.services[0], SignInMethod::PastedToken, &typed(&[])).unwrap();
        let account = &drafted.account;
        assert_eq!(account.id.as_str(), "github:github.com:signing-in");
        assert_eq!(account.server.as_str(), "https://github.com/");
        assert_eq!(account.scheme, TokenScheme::Bearer);
        assert_eq!(account.git_user, "x-access-token");
        let carrying: Vec<_> = account
            .endpoints
            .iter()
            .map(|endpoint| (endpoint.name.as_str(), endpoint.url.as_str(), endpoint.carries_token))
            .collect();
        assert_eq!(
            carrying,
            [
                ("api", "https://api.github.com/", true),
                ("graphql", "https://api.github.com/graphql", true),
                ("web", "https://github.com/", false),
            ]
        );
        assert_eq!(
            drafted.url("create_url").unwrap().as_str(),
            "https://github.com/settings/tokens/new?scopes=repo,read:org,write:public_key,workflow&description=FastGit"
        );
    }

    #[test]
    fn an_enterprise_draft_reads_the_typed_server_everywhere() {
        let manifest = github();
        let drafted = draft(
            &manifest.id,
            &manifest.services[1],
            SignInMethod::PastedToken,
            &typed(&[("server", " github.acme.dev/ ")]),
        )
        .unwrap();
        assert_eq!(drafted.account.server.as_str(), "https://github.acme.dev/");
        assert_eq!(drafted.account.endpoints[0].url.as_str(), "https://github.acme.dev/api/v3/");
        assert_eq!(drafted.url("create_url").unwrap().as_str(), "https://github.acme.dev/settings/tokens/new");
        assert_eq!(drafted.account.id.as_str(), "github:github.acme.dev:signing-in");
    }

    #[test]
    fn a_missing_or_strange_server_is_refused_with_its_label() {
        let manifest = github();
        let service = &manifest.services[1];
        assert_eq!(
            draft(&manifest.id, service, SignInMethod::PastedToken, &typed(&[])).err(),
            Some("Server address is missing".to_owned())
        );
        assert!(draft(&manifest.id, service, SignInMethod::PastedToken, &typed(&[("server", "ftp://x")])).is_err());
        assert!(draft(&manifest.id, service, SignInMethod::Device, &typed(&[("server", "x.dev")])).is_err());
    }

    #[test]
    fn signing_in_names_the_account_after_the_users_id() {
        let manifest = github();
        let drafted = draft(&manifest.id, &manifest.services[0], SignInMethod::GitCredential, &typed(&[])).unwrap();
        let account = signed_in(drafted.account, &identity("583231", "octocat")).unwrap();
        assert_eq!(account.id.as_str(), "github:github.com:583231");
        assert_eq!((account.login.as_str(), account.name.as_deref()), ("octocat", Some("Octo Cat")));
        assert_eq!(account.scopes, ["repo"]);
        assert_eq!(account.sign_in, SignInMethod::GitCredential);
        let drafted = draft(&manifest.id, &manifest.services[0], SignInMethod::PastedToken, &typed(&[])).unwrap();
        assert_eq!(signed_in(drafted.account, &identity("12 34", "x")), None);
    }

    #[test]
    fn a_typed_server_may_leave_out_https_but_not_carry_a_user() {
        assert_eq!(typed_server("ghe.example.com").unwrap().as_str(), "https://ghe.example.com/");
        assert_eq!(typed_server("http://10.0.0.5:8080").unwrap().as_str(), "http://10.0.0.5:8080/");
        assert_eq!(typed_server("https://me@ghe.example.com"), None);
        assert_eq!(typed_server("ssh://ghe.example.com"), None);
        assert_eq!(typed_server(""), None);
    }
}
