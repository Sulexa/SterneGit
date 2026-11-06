use std::{
    io::Write as _,
    process::{Command, Stdio},
};

use url::Url;

use super::{Token, authority};
use crate::git::without_console_window;

const ASK_WITHOUT_PROMPTS: [&str; 6] = [
    "-c",
    "credential.interactive=false",
    "-c",
    "core.askPass=",
    "credential",
    "fill",
];
const NO_PROMPTS: [(&str, &str); 2] = [("GCM_INTERACTIVE", "never"), ("GIT_TERMINAL_PROMPT", "0")];
const PASSWORD: &str = "password=";

pub(crate) fn token_git_holds(server: &Url) -> Option<Token> {
    token_git_holds_with(server, |_| {})
}

fn token_git_holds_with(server: &Url, adjust: impl FnOnce(&mut Command)) -> Option<Token> {
    let host = authority(server)?;
    let mut command = Command::new("git");
    command
        .args(ASK_WITHOUT_PROMPTS)
        .envs(NO_PROMPTS)
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    without_console_window(&mut command);
    adjust(&mut command);
    let mut child = command.spawn().ok()?;
    let asked = format!("protocol={}\nhost={host}\n\n", server.scheme());
    child.stdin.take()?.write_all(asked.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim_end_matches('\r').strip_prefix(PASSWORD))
        .and_then(Token::new)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use url::Url;

    use super::token_git_holds_with;
    use crate::accounts::Token;

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-git-token-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn asking(folder: &PathBuf, helper: &str, server: &str) -> Option<Token> {
        let config = folder.join("global.gitconfig");
        std::fs::write(&config, format!("[credential]\n\thelper = \"{helper}\"\n")).unwrap();
        token_git_holds_with(&Url::parse(server).unwrap(), |command| {
            command
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", &config)
                .current_dir(folder);
        })
    }

    #[test]
    fn the_token_a_helper_holds_for_the_server_is_read_back() {
        let folder = scratch("held");
        let helper = "!f() { test \\\"$1\\\" = get && echo username=octocat && echo password=gho_held; }; f";
        let token = asking(&folder, helper, "https://github.com/");
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(token, Token::new("gho_held"));
    }

    #[test]
    fn nothing_is_read_when_no_helper_holds_a_token_and_no_one_is_asked() {
        let folder = scratch("none");
        let token = asking(&folder, "!f() { :; }; f", "https://github.com/");
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(token, None);
    }
}
