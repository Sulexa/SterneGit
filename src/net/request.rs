use std::{
    fmt,
    time::{Duration, SystemTime},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Method {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    const ALL: [Method; 6] = [
        Method::Get,
        Method::Head,
        Method::Post,
        Method::Put,
        Method::Patch,
        Method::Delete,
    ];

    pub(crate) fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|method| method.name().eq_ignore_ascii_case(name))
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }

    pub(crate) fn only_reads(self) -> bool {
        matches!(self, Method::Get | Method::Head)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApiRequest {
    pub(crate) method: Method,
    pub(crate) url: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) timeout: Option<Duration>,
}

impl ApiRequest {
    pub(crate) fn new(method: Method, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout: None,
        }
    }

    pub(crate) fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub(crate) fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApiResponse {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl ApiResponse {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NetError {
    NotSignedIn,
    ServerNotAllowed(String),
    HeaderNotAllowed(String),
    RateLimitedUntil(SystemTime),
    Timeout,
    Cancelled,
    Unreachable(String),
    Tls(String),
    Proxy(String),
    TooLarge,
    NoTransport(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::NotSignedIn => f.write_str("Not signed in"),
            NetError::ServerNotAllowed(url) => write!(f, "{url} isn't one of this account's servers"),
            NetError::HeaderNotAllowed(name) => write!(f, "A connector can't send the {name} header"),
            NetError::RateLimitedUntil(_) => f.write_str("Too many requests; try again later"),
            NetError::Timeout => f.write_str("The server took too long to answer"),
            NetError::Cancelled => f.write_str("Cancelled"),
            NetError::Unreachable(reason)
            | NetError::Tls(reason)
            | NetError::Proxy(reason)
            | NetError::NoTransport(reason) => f.write_str(reason),
            NetError::TooLarge => f.write_str("The answer was too large"),
        }
    }
}

impl std::error::Error for NetError {}
