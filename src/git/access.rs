use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub(crate) const GIT_CREDENTIAL_FLAG: &str = "--git-credential";
const NO_CREDENTIAL_WINDOWS: (&str, &str) = ("GCM_INTERACTIVE", "never");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Attendance {
    #[default]
    Attended,
    Unattended,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CredentialScope {
    pub(crate) prefix: String,
    pub(crate) account: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Access {
    pub(crate) helper: Option<PathBuf>,
    pub(crate) scopes: Vec<CredentialScope>,
    pub(crate) attendance: Attendance,
}

impl Access {
    pub(crate) fn attending(attendance: Attendance) -> Self {
        Self {
            attendance,
            ..Self::default()
        }
    }

    pub(crate) fn config_args(&self) -> Vec<OsString> {
        let Some(helper) = self.helper.as_deref().and_then(Path::to_str) else {
            return Vec::new();
        };
        self.scopes
            .iter()
            .flat_map(|scope| {
                let key = format!("credential.{}.helper", scope.prefix);
                [
                    "-c".into(),
                    format!("{key}=").into(),
                    "-c".into(),
                    format!("{key}={}", helper_command(helper, &scope.account)).into(),
                ]
            })
            .collect()
    }

    pub(crate) fn environment(&self) -> impl Iterator<Item = (&'static str, &'static str)> {
        (self.attendance == Attendance::Unattended)
            .then_some(NO_CREDENTIAL_WINDOWS)
            .into_iter()
    }
}

fn helper_command(helper: &str, account: &str) -> String {
    format!(
        "!{} {GIT_CREDENTIAL_FLAG} {}",
        quoted_for_sh(&helper.replace('\\', "/")),
        quoted_for_sh(account)
    )
}

fn quoted_for_sh(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
        process::Command,
    };

    use base64::{Engine as _, engine::general_purpose::STANDARD};

    use super::{Access, Attendance, CredentialScope};
    use crate::test_server::{Reply, TestServer};

    fn scoped(prefix: &str, account: &str) -> CredentialScope {
        CredentialScope {
            prefix: prefix.into(),
            account: account.into(),
        }
    }

    fn args(access: &Access) -> Vec<String> {
        access
            .config_args()
            .into_iter()
            .map(|arg| arg.into_string().unwrap())
            .collect()
    }

    #[test]
    fn without_scopes_git_gets_no_extra_settings() {
        let access = Access {
            helper: Some(PathBuf::from("sternegit")),
            ..Access::default()
        };
        assert!(access.config_args().is_empty());
    }

    #[test]
    fn each_scope_clears_the_users_helpers_there_and_names_sternegit_instead() {
        let access = Access {
            helper: Some(PathBuf::from(r"C:\Program Files\Sterne'Git\sternegit.exe")),
            scopes: vec![scoped("https://github.com/work-org", "github:github.com:2")],
            attendance: Attendance::Attended,
        };
        assert_eq!(
            args(&access),
            [
                "-c",
                "credential.https://github.com/work-org.helper=",
                "-c",
                r"credential.https://github.com/work-org.helper=!'C:/Program Files/Sterne'\''Git/sternegit.exe' --git-credential 'github:github.com:2'",
            ]
        );
    }

    #[test]
    fn only_unattended_git_is_kept_from_opening_credential_windows() {
        assert_eq!(
            Access::attending(Attendance::Unattended).environment().collect::<Vec<_>>(),
            [("GCM_INTERACTIVE", "never")]
        );
        assert_eq!(Access::default().environment().count(), 0);
    }

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit access {name} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn basic(user: &str, password: &str) -> String {
        format!("Basic {}", STANDARD.encode(format!("{user}:{password}")))
    }

    fn ls_remote(folder: &Path, access: &Access, url: &str) {
        let mut command = Command::new("git");
        command
            .current_dir(folder)
            .args(access.config_args())
            .args([OsString::from("ls-remote"), OsString::from(url)])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", folder.join("global.gitconfig"))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env_remove("GIT_ASKPASS")
            .env_remove("SSH_ASKPASS");
        crate::git::without_console_window(&mut command);
        let _ = command.output().unwrap();
    }

    #[test]
    fn git_asks_sternegit_inside_its_scope_and_the_users_own_helper_outside_it() {
        let server = TestServer::start(|seen| match seen.header("authorization") {
            None => Reply::unauthorized(),
            Some(_) => Reply::status(404),
        });
        let folder = scratch("helper");
        let helper = folder.join("fake sternegit.sh");
        std::fs::write(
            &helper,
            "#!/bin/sh\necho username=sternegit-user\necho password=sternegit-token\n",
        )
        .unwrap();
        std::fs::write(
            folder.join("global.gitconfig"),
            "[credential]\n\thelper = \"!f() { echo username=decoy; echo password=decoy-token; }; f\"\n",
        )
        .unwrap();
        let access = Access {
            helper: Some(helper),
            scopes: vec![scoped(&server.url("/work-org"), "github:localhost:2")],
            attendance: Attendance::Attended,
        };
        ls_remote(&folder, &access, &server.url("/work-org/app.git"));
        ls_remote(&folder, &access, &server.url("/someone/fork.git"));
        let _ = std::fs::remove_dir_all(&folder);
        let sent_to = |path: &str| {
            server
                .seen()
                .into_iter()
                .filter(|seen| seen.target.starts_with(path))
                .find_map(|seen| seen.header("authorization").map(str::to_owned))
        };
        assert_eq!(sent_to("/work-org/"), Some(basic("sternegit-user", "sternegit-token")));
        assert_eq!(sent_to("/someone/"), Some(basic("decoy", "decoy-token")));
    }
}
