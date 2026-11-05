use url::Url;

use super::request::{ApiResponse, Method, NetError};
use crate::accounts::{Account, TokenScheme, is_header_name};

const HEADERS_ONLY_FASTGIT_SETS: [&str; 10] = [
    "authorization",
    "proxy-authorization",
    "cookie",
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "expect",
    "user-agent",
    "te",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    WithToken,
    WithoutToken,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    base: Url,
    carries_token: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AllowList {
    entries: Vec<Entry>,
}

impl AllowList {
    pub(crate) fn of(account: &Account) -> Self {
        Self {
            entries: account
                .endpoints
                .iter()
                .map(|endpoint| Entry {
                    base: endpoint.url.clone(),
                    carries_token: endpoint.carries_token,
                })
                .collect(),
        }
    }

    pub(crate) fn admit(&self, url: &Url) -> Option<Admission> {
        if !url.username().is_empty() || url.password().is_some() {
            return None;
        }
        self.entries
            .iter()
            .filter(|entry| covers(&entry.base, url))
            .max_by_key(|entry| entry.base.path().len())
            .map(|entry| match entry.carries_token {
                true => Admission::WithToken,
                false => Admission::WithoutToken,
            })
    }
}

fn covers(base: &Url, url: &Url) -> bool {
    let base_path = base.path();
    let path = url.path();
    let under = path == base_path
        || path.strip_prefix(base_path).is_some_and(|rest| {
            base_path.ends_with('/') || rest.starts_with('/')
        });
    base.scheme() == url.scheme()
        && base.host_str() == url.host_str()
        && base.port_or_known_default() == url.port_or_known_default()
        && under
}

pub(crate) fn check_headers(
    headers: &[(String, String)],
    scheme: &TokenScheme,
) -> Result<(), NetError> {
    let token_header = scheme.header_name();
    let refused = headers.iter().find(|(name, value)| {
        let lower = name.to_ascii_lowercase();
        !is_header_name(name)
            || !value.bytes().all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
            || HEADERS_ONLY_FASTGIT_SETS.contains(&lower.as_str())
            || name.eq_ignore_ascii_case(token_header)
    });
    match refused {
        Some((name, _)) => Err(NetError::HeaderNotAllowed(name.clone())),
        None => Ok(()),
    }
}

pub(crate) fn redirect_target(response: &ApiResponse, method: Method, current: &Url) -> Option<Url> {
    let follows = match response.status {
        301..=303 => method.only_reads(),
        307 | 308 => true,
        _ => false,
    };
    if !follows {
        return None;
    }
    current.join(response.header("location")?).ok()
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::{Admission, AllowList, check_headers, redirect_target};
    use crate::{
        accounts::{TokenScheme, fixtures},
        net::request::{ApiResponse, Method, NetError},
    };

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn only_the_accounts_endpoints_are_admitted_and_only_some_get_the_token() {
        let allowed = AllowList::of(&fixtures::github("1"));
        let admit = |text: &str| allowed.admit(&url(text));
        assert_eq!(admit("https://api.github.com/user/repos?page=2"), Some(Admission::WithToken));
        assert_eq!(admit("https://api.github.com/graphql"), Some(Admission::WithToken));
        assert_eq!(
            admit("https://avatars.githubusercontent.com/u/1"),
            Some(Admission::WithoutToken)
        );
        for refused in [
            "http://api.github.com/user",
            "https://api.github.com:444/user",
            "https://evil.example/user",
            "https://api.github.com.evil.example/user",
            "https://user:pass@api.github.com/user",
        ] {
            assert_eq!(admit(refused), None, "{refused}");
        }
    }

    #[test]
    fn an_endpoint_without_a_trailing_slash_covers_only_whole_path_segments() {
        let allowed = AllowList::of(&fixtures::account(
            "https://ghe.example.com",
            "1",
            &[("graphql", "https://ghe.example.com/api/graphql", true)],
        ));
        assert!(allowed.admit(&url("https://ghe.example.com/api/graphql")).is_some());
        assert!(allowed.admit(&url("https://ghe.example.com/api/graphql/x")).is_some());
        assert!(allowed.admit(&url("https://ghe.example.com/api/graphqlx")).is_none());
        assert!(allowed.admit(&url("https://ghe.example.com/api/v3/user")).is_none());
    }

    #[test]
    fn connectors_may_not_set_authorization_cookies_or_the_token_header() {
        let bearer = TokenScheme::Bearer;
        assert_eq!(check_headers(&headers(&[("Accept", "application/json")]), &bearer), Ok(()));
        for name in ["Authorization", "cookie", "Host", "User-Agent"] {
            assert_eq!(
                check_headers(&headers(&[(name, "x")]), &bearer),
                Err(NetError::HeaderNotAllowed(name.into()))
            );
        }
        let gitlab = TokenScheme::Header { name: "PRIVATE-TOKEN".into() };
        assert!(check_headers(&headers(&[("private-token", "x")]), &gitlab).is_err());
    }

    #[test]
    fn header_values_cannot_smuggle_extra_lines() {
        assert!(check_headers(&headers(&[("X-A", "1\r\nX-B: 2")]), &TokenScheme::Bearer).is_err());
        assert!(check_headers(&headers(&[("Bad Name", "1")]), &TokenScheme::Bearer).is_err());
    }

    fn redirect(status: u16, location: &str) -> ApiResponse {
        ApiResponse {
            status,
            headers: headers(&[("Location", location)]),
            body: Vec::new(),
        }
    }

    #[test]
    fn reads_follow_every_redirect_but_writes_only_those_that_keep_the_method() {
        let here = url("https://api.github.com/repos/a/b");
        assert_eq!(
            redirect_target(&redirect(301, "/repositories/1"), Method::Get, &here),
            Some(url("https://api.github.com/repositories/1"))
        );
        assert_eq!(redirect_target(&redirect(302, "/x"), Method::Post, &here), None);
        assert_eq!(
            redirect_target(&redirect(307, "https://api.github.com/y"), Method::Post, &here),
            Some(url("https://api.github.com/y"))
        );
        assert_eq!(redirect_target(&redirect(200, "/x"), Method::Get, &here), None);
    }
}
