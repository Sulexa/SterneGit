use std::io::{self, BufRead, Write};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CredentialRequest {
    pub(crate) protocol: Option<String>,
    pub(crate) host: Option<String>,
}

impl CredentialRequest {
    pub(crate) fn read(input: impl BufRead) -> io::Result<Self> {
        let mut request = Self::default();
        for line in input.lines() {
            let line = line?;
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                break;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = Some(value.to_owned());
            match key {
                "protocol" => request.protocol = value,
                "host" => request.host = value,
                _ => {}
            }
        }
        Ok(request)
    }

    pub(crate) fn is_for(&self, protocol: &str, host: Option<&str>) -> bool {
        let same_host = match (self.host.as_deref(), host) {
            (Some(asked), Some(served)) => asked.eq_ignore_ascii_case(served),
            _ => false,
        };
        same_host && self.protocol.as_deref() == Some(protocol)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CredentialAnswer {
    Give { username: String, password: String },
    Quit,
    Nothing,
}

impl CredentialAnswer {
    pub(crate) fn write(&self, mut output: impl Write) -> io::Result<()> {
        match self {
            CredentialAnswer::Give { username, password }
                if fits_on_a_line(username) && fits_on_a_line(password) =>
            {
                writeln!(output, "username={username}\npassword={password}")?
            }
            CredentialAnswer::Give { .. } | CredentialAnswer::Quit => writeln!(output, "quit=1")?,
            CredentialAnswer::Nothing => {}
        }
        output.flush()
    }
}

fn fits_on_a_line(value: &str) -> bool {
    !value.contains(['\n', '\r', '\0'])
}

#[cfg(test)]
mod tests {
    use super::{CredentialAnswer, CredentialRequest};

    fn written(answer: CredentialAnswer) -> String {
        let mut output = Vec::new();
        answer.write(&mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn git_s_question_is_read_up_to_the_blank_line() {
        let input = "protocol=https\r\nhost=github.com\npath=a/b.git\nwwwauth[]=Basic\n\nignored=1\n";
        assert_eq!(
            CredentialRequest::read(input.as_bytes()).unwrap(),
            CredentialRequest {
                protocol: Some("https".into()),
                host: Some("github.com".into()),
            }
        );
    }

    #[test]
    fn a_question_is_for_a_server_when_protocol_and_host_match() {
        let request = CredentialRequest {
            protocol: Some("https".into()),
            host: Some("GitHub.com".into()),
        };
        assert!(request.is_for("https", Some("github.com")));
        assert!(!request.is_for("http", Some("github.com")));
        assert!(!request.is_for("https", Some("github.com:8443")));
        assert!(!request.is_for("https", None));
    }

    #[test]
    fn answers_are_written_the_way_git_reads_them() {
        let give = CredentialAnswer::Give {
            username: "x-access-token".into(),
            password: "gho_1".into(),
        };
        assert_eq!(written(give), "username=x-access-token\npassword=gho_1\n");
        assert_eq!(written(CredentialAnswer::Quit), "quit=1\n");
        assert_eq!(written(CredentialAnswer::Nothing), "");
    }

    #[test]
    fn a_value_that_would_add_lines_to_the_answer_makes_git_stop_instead() {
        let sneaky = CredentialAnswer::Give {
            username: "user\nhost=elsewhere".into(),
            password: "token".into(),
        };
        assert_eq!(written(sneaky), "quit=1\n");
    }
}
