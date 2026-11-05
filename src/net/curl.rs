use std::{path::PathBuf, sync::Arc, time::Duration};

use futures::{
    FutureExt as _,
    future::{BoxFuture, Either, select},
};
use smol::{
    io::AsyncWriteExt as _,
    process::{Command, Stdio},
};
use url::Url;

use super::{
    proxy::Proxies,
    request::{ApiResponse, Method, NetError},
    services::Cancel,
};

const NO_CURL: &str = "Install curl to use accounts";
const ONLY_TEXT_BODIES: &str = "Only text can be sent to a server";
const CONNECT_TIMEOUT_SECONDS: u64 = 15;
const LARGEST_BODY_BYTES: usize = 32 * 1024 * 1024;
const END_OF_HEADERS: &[u8] = b"\r\n\r\n";
const NO_REVOCATION_CHECK: &str = "ssl-no-revoke";
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outgoing {
    pub(crate) method: Method,
    pub(crate) url: Url,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) timeout: Duration,
}

pub(crate) trait Transport: Send + Sync + 'static {
    fn exchange(
        &self,
        outgoing: Outgoing,
        cancel: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>>;
}

pub(crate) struct Curl {
    program: PathBuf,
    proxies: Arc<Proxies>,
}

impl Curl {
    pub(crate) fn system() -> Self {
        Self {
            program: system_curl(),
            proxies: Arc::new(Proxies::from_git_config()),
        }
    }

    #[cfg(test)]
    pub(crate) fn ignoring_git_config() -> Self {
        Self {
            program: system_curl(),
            proxies: Arc::new(Proxies::none()),
        }
    }
}

impl Transport for Curl {
    fn exchange(
        &self,
        outgoing: Outgoing,
        cancel: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>> {
        let program = self.program.clone();
        let proxies = self.proxies.clone();
        async move {
            let url = outgoing.url.clone();
            let proxy = smol::unblock(move || proxies.curl_options(&url)).await;
            let config = config_for(&outgoing, &proxy)?;
            run_curl(program, config, cancel).await
        }
        .boxed()
    }
}

fn config_for(outgoing: &Outgoing, proxy: &[String]) -> Result<String, NetError> {
    let mut lines = vec![
        format!("url = {}", quoted(outgoing.url.as_str())),
        "silent".to_owned(),
        "show-error".to_owned(),
        "include".to_owned(),
        "suppress-connect-headers".to_owned(),
        format!("proto = {}", quoted(&format!("={}", outgoing.url.scheme()))),
        format!("max-time = {}", outgoing.timeout.as_secs().max(1)),
        format!("connect-timeout = {CONNECT_TIMEOUT_SECONDS}"),
        format!("max-filesize = {LARGEST_BODY_BYTES}"),
        format!("header = {}", quoted("Expect:")),
    ];
    match outgoing.method {
        Method::Get => {}
        Method::Head => lines.push("head".to_owned()),
        other => lines.push(format!("request = {}", quoted(other.name()))),
    }
    for (name, value) in &outgoing.headers {
        lines.push(format!("header = {}", quoted(&format!("{name}: {value}"))));
    }
    if let Some(body) = &outgoing.body {
        let text = std::str::from_utf8(body)
            .ok()
            .filter(|text| !text.contains('\0'))
            .ok_or_else(|| NetError::NoTransport(ONLY_TEXT_BODIES.to_owned()))?;
        lines.push(format!("data-binary = {}", quoted(text)));
    }
    if cfg!(windows) && !proxy.iter().any(|option| option == NO_REVOCATION_CHECK) {
        lines.push("ssl-revoke-best-effort".to_owned());
    }
    lines.extend(proxy.iter().cloned());
    let mut config = lines.join("\n");
    config.push('\n');
    Ok(config)
}

pub(super) fn quoted(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str(r"\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str(r"\n"),
            '\r' => quoted.push_str(r"\r"),
            '\t' => quoted.push_str(r"\t"),
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

async fn run_curl(program: PathBuf, config: String, cancel: Cancel) -> Result<ApiResponse, NetError> {
    let mut command = Command::new(&program);
    command
        .args(["-q", "--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    detach(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| NetError::NoTransport(NO_CURL.to_owned()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| NetError::NoTransport(NO_CURL.to_owned()))?;
    let exchange = async move {
        stdin.write_all(config.as_bytes()).await?;
        stdin.close().await?;
        drop(stdin);
        child.output().await
    };
    let output = match select(Box::pin(exchange), Box::pin(cancel.cancelled())).await {
        Either::Left((output, _)) => {
            output.map_err(|error| NetError::Unreachable(error.to_string()))?
        }
        Either::Right(_) => return Err(NetError::Cancelled),
    };
    match output.status.code() {
        Some(0) => parse_response(&output.stdout),
        Some(code) => Err(failure(code, &output.stderr)),
        None => Err(NetError::Cancelled),
    }
}

fn parse_response(printed: &[u8]) -> Result<ApiResponse, NetError> {
    let unreadable = || NetError::Unreachable("curl's answer couldn't be read".to_owned());
    let mut rest = printed;
    loop {
        let end = find(rest, END_OF_HEADERS).ok_or_else(unreadable)?;
        let head = String::from_utf8_lossy(&rest[..end]);
        let body = &rest[end + END_OF_HEADERS.len()..];
        let mut lines = head.split("\r\n");
        let status = lines.next().and_then(status_of).ok_or_else(unreadable)?;
        if (100..200).contains(&status) {
            rest = body;
            continue;
        }
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
            .collect();
        return Ok(ApiResponse {
            status,
            headers,
            body: body.to_vec(),
        });
    }
}

fn status_of(line: &str) -> Option<u16> {
    let mut words = line.split_whitespace();
    words.next().filter(|version| version.starts_with("HTTP/"))?;
    words.next()?.parse().ok()
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn failure(code: i32, stderr: &[u8]) -> NetError {
    let message = String::from_utf8_lossy(stderr).trim().to_owned();
    match code {
        28 => NetError::Timeout,
        5 | 97 => NetError::Proxy(message),
        35 | 51 | 53 | 54 | 58 | 59 | 60 | 64 | 66 | 77 | 80 | 82 | 83 | 90 | 91 => {
            NetError::Tls(message)
        }
        63 => NetError::TooLarge,
        _ => NetError::Unreachable(message),
    }
}

#[cfg(windows)]
fn system_curl() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from)
        .join(r"System32\curl.exe")
}

#[cfg(target_os = "macos")]
fn system_curl() -> PathBuf {
    PathBuf::from("/usr/bin/curl")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_curl() -> PathBuf {
    PathBuf::from("curl")
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use smol::process::windows::CommandExt as _;
    command.creation_flags(DETACHED_PROCESS);
}

#[cfg(not(windows))]
fn detach(_: &mut Command) {}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use url::Url;

    use super::{Curl, Outgoing, Transport as _, config_for, failure, parse_response, quoted};
    use crate::{
        net::{
            request::{Method, NetError},
            services::cancel_pair,
        },
        test_server::{Reply, TestServer},
    };

    fn outgoing(method: Method, url: &str) -> Outgoing {
        Outgoing {
            method,
            url: Url::parse(url).unwrap(),
            headers: Vec::new(),
            body: None,
            timeout: Duration::from_secs(10),
        }
    }

    #[test]
    fn config_values_escape_what_curl_would_otherwise_misread() {
        assert_eq!(quoted("a\"b\\c\nd\te"), r#""a\"b\\c\nd\te""#);
    }

    #[test]
    fn the_request_and_its_headers_go_into_the_config_never_the_command_line() {
        let mut request = outgoing(Method::Post, "https://api.github.com/user/repos");
        request.headers.push(("Authorization".into(), "Bearer gho_secret".into()));
        request.body = Some(br#"{"name":"a \"quoted\" name"}"#.to_vec());
        let config = config_for(&request, &[]).unwrap();
        assert!(config.contains("url = \"https://api.github.com/user/repos\"\n"));
        assert!(config.contains("proto = \"=https\"\n"));
        assert!(config.contains("request = \"POST\"\n"));
        assert!(config.contains("header = \"Authorization: Bearer gho_secret\"\n"));
        assert!(config.contains(r#"data-binary = "{\"name\":\"a \\\"quoted\\\" name\"}""#));
    }

    #[test]
    fn each_method_is_named_the_way_curl_expects() {
        let named = |method| config_for(&outgoing(method, "https://api.github.com/x"), &[]).unwrap();
        assert!(!named(Method::Get).contains("request ="));
        assert!(named(Method::Head).contains("\nhead\n"));
        for (method, name) in [
            (Method::Put, "PUT"),
            (Method::Patch, "PATCH"),
            (Method::Delete, "DELETE"),
        ] {
            assert!(named(method).contains(&format!("request = \"{name}\"\n")), "{name}");
        }
    }

    #[test]
    fn fastgit_runs_the_curl_that_comes_with_the_system() {
        let program = Curl::system().program;
        let name = program.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(name, if cfg!(windows) { "curl.exe" } else { "curl" });
    }

    #[test]
    fn a_body_curl_cannot_carry_is_refused() {
        let mut request = outgoing(Method::Post, "https://api.github.com/x");
        request.body = Some(vec![0xff, 0x00]);
        assert!(matches!(config_for(&request, &[]), Err(NetError::NoTransport(_))));
    }

    #[test]
    fn informational_answers_are_skipped_and_headers_and_body_kept() {
        let printed = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/2 201 \r\nlocation: /x\r\nlink: <a>; rel=\"next\"\r\n\r\n{\"ok\":true}";
        let response = parse_response(printed).unwrap();
        assert_eq!(response.status, 201);
        assert_eq!(response.header("Location"), Some("/x"));
        assert_eq!(response.header("link"), Some("<a>; rel=\"next\""));
        assert_eq!(response.body, b"{\"ok\":true}");
    }

    #[test]
    fn curls_exit_codes_say_what_went_wrong() {
        assert_eq!(failure(28, b""), NetError::Timeout);
        assert_eq!(failure(63, b""), NetError::TooLarge);
        assert!(matches!(failure(60, b"bad certificate"), NetError::Tls(message) if message == "bad certificate"));
        assert!(matches!(failure(7, b""), NetError::Unreachable(_)));
        assert!(matches!(failure(97, b""), NetError::Proxy(_)));
    }

    #[test]
    fn curl_sends_the_method_headers_and_exact_body_and_reads_the_answer() {
        let server = TestServer::start(|_| {
            Reply::status(201)
                .header("X-Answer", "made")
                .body("{\"id\":7}")
        });
        let body = "{\"title\":\"quotes \\\" backslash \\\\ tab\\t é ✓ #hash\"}\n";
        let mut request = outgoing(Method::Post, &server.url("/repos/a/b/pulls"));
        request.headers.push(("Accept".into(), "application/json".into()));
        request.body = Some(body.as_bytes().to_vec());
        let (_keep, cancel) = cancel_pair();
        let response = smol::block_on(Curl::ignoring_git_config().exchange(request, cancel)).unwrap();
        let seen = server.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].target, "/repos/a/b/pulls");
        assert_eq!(seen[0].header("accept"), Some("application/json"));
        assert_eq!(seen[0].header("expect"), None);
        assert_eq!(seen[0].body, body.as_bytes());
        assert_eq!(response.status, 201);
        assert_eq!(response.header("x-answer"), Some("made"));
        assert_eq!(response.body, b"{\"id\":7}");
    }

    #[test]
    fn curl_hands_redirects_back_instead_of_following_them() {
        let server = TestServer::start(|_| Reply::status(302).header("Location", "/elsewhere"));
        let (_keep, cancel) = cancel_pair();
        let response = smol::block_on(
            Curl::ignoring_git_config().exchange(outgoing(Method::Get, &server.url("/here")), cancel),
        )
        .unwrap();
        assert_eq!(response.status, 302);
        assert_eq!(server.seen().len(), 1);
    }

    #[test]
    fn a_server_slower_than_the_timeout_is_a_timeout() {
        let server = TestServer::start(|_| Reply::status(200).after(Duration::from_secs(4)));
        let mut request = outgoing(Method::Get, &server.url("/slow"));
        request.timeout = Duration::from_secs(1);
        let (_keep, cancel) = cancel_pair();
        let result = smol::block_on(Curl::ignoring_git_config().exchange(request, cancel));
        assert_eq!(result, Err(NetError::Timeout));
    }

    #[test]
    fn dropping_the_cancel_handle_stops_the_request_at_once() {
        let server = TestServer::start(|_| Reply::status(200).after(Duration::from_secs(8)));
        let (keep, cancel) = cancel_pair();
        let started = Instant::now();
        let request = Curl::ignoring_git_config().exchange(outgoing(Method::Get, &server.url("/slow")), cancel);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(keep);
        });
        assert_eq!(smol::block_on(request), Err(NetError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(4));
    }
}
