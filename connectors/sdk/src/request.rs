use sternegit_protocol::{Failure, RequestHead, ResponseHead};
use serde::{Serialize, de::DeserializeOwned};

use crate::host;

const LINK: &str = "link";
const NEXT: &str = "next";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    method: &'static str,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self::new("GET", url)
    }

    pub fn head(url: impl Into<String>) -> Self {
        Self::new("HEAD", url)
    }

    pub fn post(url: impl Into<String>) -> Self {
        Self::new("POST", url)
    }

    pub fn put(url: impl Into<String>) -> Self {
        Self::new("PUT", url)
    }

    pub fn patch(url: impl Into<String>) -> Self {
        Self::new("PATCH", url)
    }

    pub fn delete(url: impl Into<String>) -> Self {
        Self::new("DELETE", url)
    }

    fn new(method: &'static str, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn json(self, body: &impl Serialize) -> Self {
        let body = serde_json::to_vec(body).expect("a request body serializes as JSON");
        self.header("Content-Type", "application/json").body(body)
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn send(self) -> Sent {
        let head = RequestHead {
            method: self.method.to_owned(),
            url: self.url,
            headers: self.headers,
        };
        let head = serde_json::to_vec(&head).expect("a request head serializes as JSON");
        Sent(host::send(&head, &self.body))
    }

    pub fn fetch(self) -> Response {
        self.send().wait()
    }
}

#[must_use = "a sent request is read with wait"]
pub struct Sent(i32);

impl Sent {
    pub fn wait(self) -> Response {
        Response::unframed(&host::wait(self.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    fn unframed(framed: &[u8]) -> Self {
        let (head, body) = ResponseHead::unframe(framed).expect("SterneGit frames every response");
        Self {
            status: head.status,
            headers: head.headers,
            body: body.to_vec(),
        }
    }

    pub fn succeeded(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Failure> {
        serde_json::from_slice(&self.body).map_err(|error| Failure::Protocol {
            message: format!("the server's answer isn't the JSON expected: {error}"),
        })
    }

    pub fn next_page(&self) -> Option<String> {
        self.header(LINK).and_then(next_link)
    }
}

fn next_link(link: &str) -> Option<String> {
    let mut rest = link;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let end = after.find('>')?;
        let parameters = &after[end + 1..];
        let parameters = parameters
            .find('<')
            .map_or(parameters, |next| &parameters[..next]);
        if parameters.split(';').any(names_next) {
            return Some(after[..end].to_owned());
        }
        rest = &after[end + 1..];
    }
    None
}

fn names_next(parameter: &str) -> bool {
    let Some((name, value)) = parameter.split_once('=') else {
        return false;
    };
    let value = value.trim().trim_end_matches(',').trim().trim_matches('"');
    name.trim().eq_ignore_ascii_case("rel")
        && value
            .split_ascii_whitespace()
            .any(|relation| relation.eq_ignore_ascii_case(NEXT))
}

#[cfg(test)]
mod tests {
    use super::{Response, next_link};

    fn linked(link: &str) -> Response {
        Response {
            status: 200,
            headers: vec![("Link".into(), link.into())],
            body: Vec::new(),
        }
    }

    #[test]
    fn the_next_page_is_the_link_whose_relation_is_next() {
        let github = r#"<https://api.github.com/user/repos?page=2>; rel="next", <https://api.github.com/user/repos?page=5>; rel="last""#;
        assert_eq!(
            linked(github).next_page().as_deref(),
            Some("https://api.github.com/user/repos?page=2")
        );
        let unquoted = r#"<https://example.com/a?page=1>; rel="prev", <https://example.com/a?page=3>; rel=next"#;
        assert_eq!(next_link(unquoted).as_deref(), Some("https://example.com/a?page=3"));
        let several = r#"<https://example.com/b?x=1,2>; title="list"; rel="next last""#;
        assert_eq!(next_link(several).as_deref(), Some("https://example.com/b?x=1,2"));
    }

    #[test]
    fn the_last_page_has_no_next() {
        assert_eq!(linked(r#"<https://example.com/a?page=1>; rel="prev""#).next_page(), None);
        assert_eq!(linked("garbage").next_page(), None);
        assert_eq!(linked("<unclosed; rel=\"next\"").next_page(), None);
        let unlinked = Response {
            status: 200,
            headers: Vec::new(),
            body: Vec::new(),
        };
        assert_eq!(unlinked.next_page(), None);
    }

    #[test]
    fn a_body_reads_as_json_or_says_why_not() {
        let response = Response {
            status: 201,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: br#"{"id":7}"#.to_vec(),
        };
        assert!(response.succeeded());
        assert_eq!(response.header("content-type"), Some("application/json"));
        assert_eq!(response.json::<serde_json::Value>().unwrap()["id"], 7);
        let failure = response.json::<Vec<u8>>().unwrap_err();
        assert!(failure.to_string().contains("isn't the JSON expected"), "{failure}");
    }
}
