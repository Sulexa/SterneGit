use std::{
    io::Write as _,
    process::{Command, Stdio},
};

const ACCOUNT: &str = "github:github.com:1";
const QUESTION: &str = "protocol=https\nhost=github.com\n\n";

fn fastgit_answering(label: &str, arguments: &[&str], question: &str) -> (String, bool) {
    let settings =
        std::env::temp_dir().join(format!("fastgit-helper-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&settings).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_fastgit"))
        .args(arguments)
        .env("APPDATA", &settings)
        .env("XDG_CONFIG_HOME", &settings)
        .env("HOME", &settings)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(question.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&settings);
    (String::from_utf8(output.stdout).unwrap(), output.status.success())
}

#[test]
fn git_is_told_to_stop_when_fastgit_has_no_such_account() {
    let answered = fastgit_answering("get", &["--git-credential", ACCOUNT, "get"], QUESTION);
    assert_eq!(answered, ("quit=1\n".to_owned(), true));
}

#[test]
fn what_git_stores_or_erases_leaves_fastgit_untouched() {
    for operation in ["store", "erase"] {
        let question = "protocol=https\nhost=github.com\nusername=x\npassword=y\n\n";
        let answered = fastgit_answering(
            operation,
            &["--git-credential", ACCOUNT, operation],
            question,
        );
        assert_eq!(answered, (String::new(), true), "{operation}");
    }
}

#[test]
fn a_credential_question_with_missing_arguments_is_refused() {
    let (_, finished) = fastgit_answering("misuse", &["--git-credential", ACCOUNT], QUESTION);
    assert!(!finished);
}
