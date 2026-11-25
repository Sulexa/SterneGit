use std::{
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const ACCOUNT: &str = "github:github.com:1";
const QUESTION: &str = "protocol=https\nhost=github.com\n\n";

const SEEDED_ACCOUNTS: &str = r#"{
  "version": 1,
  "accounts": [
    {
      "id": "github:github.com:1",
      "connector": "github",
      "service": "github.com",
      "server": "https://github.com/",
      "endpoints": [{ "name": "api", "url": "https://api.github.com/", "token": true }],
      "login": "login-1",
      "git_user": "x-access-token",
      "sign_in": "pasted_token",
      "scheme": { "kind": "bearer" },
      "scopes": ["repo"]
    }
  ],
  "remote_owners": []
}
"#;

fn settings_folder(label: &str) -> PathBuf {
    let settings =
        std::env::temp_dir().join(format!("sternegit-helper-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&settings);
    std::fs::create_dir_all(&settings).unwrap();
    settings
}

fn accounts_file(settings: &Path) -> PathBuf {
    let config = if cfg!(windows) {
        settings.join("SterneGit")
    } else if cfg!(target_os = "macos") {
        settings.join("Library/Application Support/SterneGit")
    } else {
        settings.join("sternegit")
    };
    config.join("accounts.json")
}

fn sternegit_answering(label: &str, arguments: &[&str], question: &str) -> (String, bool) {
    let settings = settings_folder(label);
    let answered = sternegit_answering_in(&settings, arguments, question);
    let _ = std::fs::remove_dir_all(&settings);
    answered
}

fn sternegit_answering_in(settings: &Path, arguments: &[&str], question: &str) -> (String, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sternegit"))
        .args(arguments)
        .env("APPDATA", settings)
        .env("XDG_CONFIG_HOME", settings)
        .env("HOME", settings)
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
    (String::from_utf8(output.stdout).unwrap(), output.status.success())
}

#[test]
fn git_is_told_to_stop_when_sternegit_has_no_such_account() {
    let answered = sternegit_answering("get", &["--git-credential", ACCOUNT, "get"], QUESTION);
    assert_eq!(answered, ("quit=1\n".to_owned(), true));
}

#[test]
fn what_git_stores_or_erases_leaves_sternegit_untouched() {
    for operation in ["store", "erase"] {
        let question = "protocol=https\nhost=github.com\nusername=x\npassword=y\n\n";
        let settings = settings_folder(operation);
        let accounts = accounts_file(&settings);
        std::fs::create_dir_all(accounts.parent().unwrap()).unwrap();
        std::fs::write(&accounts, SEEDED_ACCOUNTS).unwrap();
        let answered = sternegit_answering_in(
            &settings,
            &["--git-credential", ACCOUNT, operation],
            question,
        );
        let kept = std::fs::read(&accounts);
        let _ = std::fs::remove_dir_all(&settings);
        assert_eq!(answered, (String::new(), true), "{operation}");
        assert_eq!(kept.unwrap(), SEEDED_ACCOUNTS.as_bytes(), "{operation}");
    }
}

#[test]
fn a_credential_question_with_missing_arguments_is_refused() {
    let (_, finished) = sternegit_answering("misuse", &["--git-credential", ACCOUNT], QUESTION);
    assert!(!finished);
}
