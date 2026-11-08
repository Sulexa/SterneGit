use std::fmt;

const RUNNING: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Version {
    numbers: [u64; 3],
    label: Option<String>,
}

impl Version {
    pub(crate) fn running() -> Self {
        Self::parse(RUNNING).expect("Cargo.toml's version is X.Y.Z or X.Y.Z-label")
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (numbers, label) = match text.split_once('-') {
            Some((numbers, label)) => (numbers, Some(label)),
            None => (text, None),
        };
        if label.is_some_and(|label| label.is_empty() || label.contains(char::is_whitespace)) {
            return None;
        }
        let mut parts = numbers.split('.').map(number);
        let numbers = [parts.next()??, parts.next()??, parts.next()??];
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            numbers,
            label: label.map(str::to_owned),
        })
    }

    pub(crate) fn is_pre_release(&self) -> bool {
        self.label.is_some()
    }

    pub(crate) fn replaces(&self, running: &Version) -> bool {
        !self.is_pre_release()
            && (self.numbers > running.numbers
                || (self.numbers == running.numbers && running.is_pre_release()))
    }
}

fn number(text: &str) -> Option<u64> {
    let digits_only = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    let no_leading_zero = text == "0" || !text.starts_with('0');
    (digits_only && no_leading_zero).then(|| text.parse().ok())?
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [major, minor, patch] = self.numbers;
        write!(f, "{major}.{minor}.{patch}")?;
        if let Some(label) = &self.label {
            write!(f, "-{label}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn the_running_version_is_the_one_in_cargo_toml() {
        assert_eq!(Version::running().to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn versions_read_back_as_written() {
        for text in ["0.1.0", "12.0.3", "0.3.0-rc.1"] {
            assert_eq!(version(text).to_string(), text);
        }
    }

    #[test]
    fn anything_but_three_plain_numbers_and_an_optional_label_is_refused() {
        for text in ["", "1", "1.2", "1.2.3.4", "v1.2.3", "1.02.3", "1.2.x", "1.2.3-", "1.2.+3", "1.2.3- rc"] {
            assert_eq!(Version::parse(text), None, "{text}");
        }
    }

    #[test]
    fn a_higher_release_replaces_the_running_one() {
        let running = version("0.2.5");
        assert!(version("0.2.6").replaces(&running));
        assert!(version("0.3.0").replaces(&running));
        assert!(version("1.0.0").replaces(&running));
        assert!(version("0.10.0").replaces(&version("0.9.0")));
    }

    #[test]
    fn the_same_or_an_older_release_never_replaces_the_running_one() {
        let running = version("0.2.5");
        assert!(!version("0.2.5").replaces(&running));
        assert!(!version("0.2.4").replaces(&running));
        assert!(!version("0.1.9").replaces(&running));
    }

    #[test]
    fn a_pre_release_is_never_offered() {
        assert!(!version("9.0.0-rc.1").replaces(&version("0.1.0")));
    }

    #[test]
    fn the_release_replaces_its_own_pre_release() {
        assert!(version("0.3.0").replaces(&version("0.3.0-rc.2")));
    }
}
