use gpui_kit::SharedString;

use crate::git::Commit;

pub(crate) fn commit_subject(commit: &Commit) -> SharedString {
    if commit.subject.is_empty() {
        "(no message)".into()
    } else {
        commit.subject.clone()
    }
}

pub(crate) fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (ix, digit) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub(crate) fn counted(n: usize, singular: &str, plural: &str) -> String {
    format!(
        "{} {}",
        thousands(n),
        if n == 1 { singular } else { plural }
    )
}

#[cfg(test)]
mod tests {
    use super::{counted, thousands};

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn only_one_takes_the_singular() {
        assert_eq!(counted(0, "file", "files"), "0 files");
        assert_eq!(counted(1, "file", "files"), "1 file");
        assert_eq!(counted(1_200, "file", "files"), "1,200 files");
    }
}
