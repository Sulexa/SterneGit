use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use url::Url;

use super::curl::quoted;
use crate::git::url_settings::url_settings;

const FRESH_FOR: Duration = Duration::from_secs(60);
const HTTP_SECTION: &str = "http";

type Remembered = HashMap<String, (Instant, Arc<Vec<String>>)>;

pub(crate) struct Proxies {
    reads_git_config: bool,
    remembered: Mutex<Remembered>,
}

impl Proxies {
    pub(crate) fn from_git_config() -> Self {
        Self {
            reads_git_config: true,
            remembered: Mutex::default(),
        }
    }

    pub(crate) fn none() -> Self {
        Self {
            reads_git_config: false,
            remembered: Mutex::default(),
        }
    }

    pub(crate) fn curl_options(&self, url: &Url) -> Arc<Vec<String>> {
        if !self.reads_git_config {
            return Arc::default();
        }
        let origin = url.origin().ascii_serialization();
        let mut remembered = self.remembered.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((read_at, options)) = remembered.get(&origin)
            && read_at.elapsed() < FRESH_FOR
        {
            return options.clone();
        }
        let options = Arc::new(curl_options(&url_settings(HTTP_SECTION, url.as_str())));
        remembered.insert(origin, (Instant::now(), options.clone()));
        options
    }
}

fn curl_options(settings: &[(String, String)]) -> Vec<String> {
    settings
        .iter()
        .filter_map(|(key, value)| match key.as_str() {
            "http.proxy" if !value.is_empty() => Some(format!("proxy = {}", quoted(value))),
            "http.proxyauthmethod" => proxy_auth(value).map(str::to_owned),
            "http.sslcainfo" if !value.is_empty() => Some(format!("cacert = {}", quoted(value))),
            "http.schannelcheckrevoke" if is_false(value) => Some("ssl-no-revoke".to_owned()),
            "http.sslverify" if is_false(value) => Some("insecure".to_owned()),
            _ => None,
        })
        .collect()
}

fn proxy_auth(method: &str) -> Option<&'static str> {
    match method.to_ascii_lowercase().as_str() {
        "anyauth" => Some("proxy-anyauth"),
        "basic" => Some("proxy-basic"),
        "digest" => Some("proxy-digest"),
        "negotiate" => Some("proxy-negotiate"),
        "ntlm" => Some("proxy-ntlm"),
        _ => None,
    }
}

fn is_false(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "false" | "no" | "off" | "0")
}

#[cfg(test)]
mod tests {
    use super::curl_options;

    fn settings(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn gits_http_settings_become_the_same_curl_options() {
        let options = curl_options(&settings(&[
            ("http.proxy", r"http://proxy.corp:3128"),
            ("http.proxyauthmethod", "anyauth"),
            ("http.sslcainfo", r"C:\certs\corp.pem"),
            ("http.schannelcheckrevoke", "false"),
            ("http.sslverify", "true"),
            ("http.postbuffer", "524288000"),
        ]));
        assert_eq!(
            options,
            [
                r#"proxy = "http://proxy.corp:3128""#,
                "proxy-anyauth",
                r#"cacert = "C:\\certs\\corp.pem""#,
                "ssl-no-revoke",
            ]
        );
    }

    #[test]
    fn turning_off_certificate_checks_in_git_turns_them_off_for_curl_too() {
        assert_eq!(curl_options(&settings(&[("http.sslverify", "no")])), ["insecure"]);
    }
}
