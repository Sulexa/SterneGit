use fastgit_sdk::{AccountContext, Failure, RepoRef, Request};
use serde_json::Value;

use crate::{api, refusal::succeeded};

const API_ENDPOINT: &str = "api";
const PAGES_READ_AT_MOST: usize = 10;
const UNRESERVED: &[u8] = b"-._~";

pub(crate) fn repo_path(repo: &RepoRef) -> String {
    format!("repos/{}/{}", encoded(&repo.owner), encoded(&repo.name))
}

pub(crate) fn cursor_on_api(account: &AccountContext, cursor: String) -> Result<String, Failure> {
    on_api(account, Some(cursor)).ok_or_else(|| Failure::Protocol {
        message: "The page asked for isn't on GitHub's API".to_owned(),
    })
}

pub(crate) fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

pub(crate) fn written(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
}

pub(crate) fn every_page(account: &AccountContext, first: &str) -> Result<Vec<Value>, Failure> {
    let mut items = Vec::new();
    let mut next = Some(first.to_owned());
    for _ in 0..PAGES_READ_AT_MOST {
        let Some(url) = next.take() else {
            break;
        };
        let response = succeeded(api(Request::get(url)).fetch())?;
        let page: Vec<Value> = response.json()?;
        items.extend(page);
        next = on_api(account, response.next_page());
    }
    Ok(items)
}

pub(crate) fn on_api(account: &AccountContext, link: Option<String>) -> Option<String> {
    let api = account.endpoint(API_ENDPOINT)?;
    link.filter(|url| url.starts_with(api))
}

pub(crate) fn id_of(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

pub(crate) fn encoded(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || UNRESERVED.contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use fastgit_sdk::AccountContext;
    use serde_json::json;

    use super::{encoded, id_of, on_api};

    #[test]
    fn query_text_is_percent_encoded_except_unreserved_bytes() {
        assert_eq!(encoded("fast git"), "fast%20git");
        assert_eq!(encoded("a+b/c:d"), "a%2Bb%2Fc%3Ad");
        assert_eq!(encoded("été"), "%C3%A9t%C3%A9");
        assert_eq!(encoded("x-y_z.w~"), "x-y_z.w~");
    }

    #[test]
    fn ids_read_from_numbers_or_strings() {
        assert_eq!(id_of(&json!(583231)).as_deref(), Some("583231"));
        assert_eq!(id_of(&json!("MDQ6VXNlcjE=")).as_deref(), Some("MDQ6VXNlcjE="));
        assert_eq!(id_of(&json!(null)), None);
        assert_eq!(id_of(&json!("")), None);
    }

    #[test]
    fn a_next_page_off_the_api_endpoint_is_dropped() {
        let account = AccountContext {
            endpoints: [("api".to_owned(), "https://api.github.com/".to_owned())].into(),
            ..AccountContext::default()
        };
        let next = |url: &str| on_api(&account, Some(url.to_owned()));
        assert_eq!(next("https://api.github.com/user/repos?page=2").as_deref(), Some("https://api.github.com/user/repos?page=2"));
        assert_eq!(next("https://evil.example/user/repos?page=2"), None);
    }
}
