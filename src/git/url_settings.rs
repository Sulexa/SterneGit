use super::command::{Exit, run_text};

pub(crate) fn url_settings(section: &str, url: &str) -> Vec<(String, String)> {
    run_text(
        &std::env::temp_dir(),
        &["config", "--get-urlmatch", section, url],
        Exit::SuccessOrDifference,
    )
    .map(|listed| settings_in(&listed))
    .unwrap_or_default()
}

fn settings_in(listed: &str) -> Vec<(String, String)> {
    listed
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            (!key.is_empty()).then(|| (key.to_ascii_lowercase(), value.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::settings_in;

    #[test]
    fn each_line_is_a_lowercased_key_and_its_value() {
        assert_eq!(
            settings_in("http.proxy http://proxy:3128\nhttp.sslVerify false\nhttp.emptyvalue\n"),
            [
                ("http.proxy".to_owned(), "http://proxy:3128".to_owned()),
                ("http.sslverify".to_owned(), "false".to_owned()),
                ("http.emptyvalue".to_owned(), String::new()),
            ]
        );
    }
}
