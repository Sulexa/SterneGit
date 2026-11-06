use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context as _, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD, engine::general_purpose::STANDARD_NO_PAD};
use sha2::{Digest as _, Sha256};

use super::command::{Exit, run, run_text, without_console_window};
use crate::settings::Folder;

const KEYS_FOLDER: &str = "ssh";
const KNOWN_HOSTS: &str = "known_hosts";
const PUBLIC_SUFFIX: &str = "pub";
const FINGERPRINT_PREFIX: &str = "SHA256:";
const SSH_COMMAND_KEY: &str = "core.sshCommand";
const KEYGEN: &str = "ssh-keygen";
const USERS_KNOWN_HOSTS: &str = "~/.ssh/known_hosts";
const NO_KEYGEN: &str = "Couldn't find ssh-keygen beside Git or on PATH";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyPair {
    pub(crate) private: PathBuf,
    pub(crate) public: String,
}

impl KeyPair {
    pub(crate) fn read(private: &Path) -> Option<Self> {
        let public = fs::read_to_string(public_half(private)).ok()?;
        let public = public.trim().to_owned();
        (private.is_file() && !public.is_empty()).then(|| Self {
            private: private.to_owned(),
            public,
        })
    }

    pub(crate) fn fingerprint(&self) -> Option<String> {
        fingerprint(&self.public)
    }

    pub(crate) fn kind(&self) -> &str {
        self.public.split_whitespace().next().unwrap_or_default()
    }

    pub(crate) fn delete(&self) -> io::Result<()> {
        for path in [self.private.clone(), public_half(&self.private)] {
            match fs::remove_file(&path) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(())
    }
}

pub(crate) fn keys_folder() -> Option<PathBuf> {
    Folder::Local.path().map(|folder| folder.join(KEYS_FOLDER))
}

pub(crate) fn known_hosts_file() -> Option<PathBuf> {
    keys_folder().map(|folder| folder.join(KNOWN_HOSTS))
}

pub(crate) fn key_file_name(account: &str, kind: &str) -> String {
    format!("{}_{kind}", account.replace(':', "-"))
}

pub(crate) fn generate(private: &Path, kind: &str, comment: &str) -> Result<KeyPair> {
    let keygen = find_keygen().context(NO_KEYGEN)?;
    if let Some(folder) = private.parent() {
        make_private_folder(folder)?;
    }
    if let Some(stale) = KeyPair::read(private) {
        stale.delete()?;
    }
    let _ = fs::remove_file(private);
    let mut command = Command::new(keygen);
    command
        .args(["-q", "-t", kind, "-N", "", "-C", comment, "-f"])
        .arg(private)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    without_console_window(&mut command);
    let output = command.output().context(NO_KEYGEN)?;
    if !output.status.success() {
        bail!("ssh-keygen failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    KeyPair::read(private).context("ssh-keygen made no key")
}

pub(crate) fn fingerprint(public: &str) -> Option<String> {
    let blob = public.split_whitespace().nth(1)?;
    let decoded = STANDARD.decode(blob).ok()?;
    Some(format!("{FINGERPRINT_PREFIX}{}", STANDARD_NO_PAD.encode(Sha256::digest(&decoded))))
}

pub(crate) fn same_key(first: &str, second: &str) -> bool {
    fn blob(line: &str) -> Vec<&str> {
        line.split_whitespace().take(2).collect()
    }
    let first = blob(first);
    first.len() == 2 && first == blob(second)
}

pub(crate) fn remember_host_keys(file: &Path, hosts: &[String], keys: &[String]) -> io::Result<()> {
    let kept = fs::read_to_string(file).unwrap_or_default();
    let mut lines: Vec<String> = kept
        .lines()
        .filter(|line| {
            let host = line.split_whitespace().next().unwrap_or_default();
            !line.trim().is_empty() && !hosts.iter().any(|known| known.eq_ignore_ascii_case(host))
        })
        .map(str::to_owned)
        .collect();
    for host in hosts {
        for key in keys {
            let mut parts = key.split_whitespace();
            if let (Some(kind), Some(blob)) = (parts.next(), parts.next()) {
                lines.push(format!("{host} {kind} {blob}"));
            }
        }
    }
    if let Some(folder) = file.parent() {
        make_private_folder(folder)?;
    }
    let mut text = lines.join("\n");
    text.push('\n');
    fs::write(file, text)
}

pub(crate) fn knows_host(file: &Path, host: &str) -> bool {
    fs::read_to_string(file).is_ok_and(|text| {
        text.lines()
            .any(|line| line.split_whitespace().next().is_some_and(|known| known.eq_ignore_ascii_case(host)))
    })
}

pub(crate) fn ssh_command(private: &Path, known_hosts: &Path, strict: bool) -> String {
    let known = format!("UserKnownHostsFile=\"{}\" {USERS_KNOWN_HOSTS}", slashed(known_hosts));
    let checking = if strict { "yes" } else { "accept-new" };
    format!(
        "ssh -i {} -o IdentitiesOnly=yes -o {} -o StrictHostKeyChecking={checking}",
        quoted_for_sh(&slashed(private)),
        quoted_for_sh(&known)
    )
}

pub(crate) fn names_key(command: &str, private: &Path) -> bool {
    command.contains(&quoted_for_sh(&slashed(private)))
}

pub(crate) fn ssh_command_of(repository: &Path) -> Option<String> {
    run_text(repository, &["config", "--local", "--get", SSH_COMMAND_KEY], Exit::Success)
        .ok()
        .map(|command| command.trim().to_owned())
        .filter(|command| !command.is_empty())
}

pub(crate) fn set_ssh_command(repository: &Path, command: &str) -> Result<()> {
    run(repository, &["config", "--local", SSH_COMMAND_KEY, command], Exit::Success).map(drop)
}

pub(crate) fn unset_ssh_command(repository: &Path) -> Result<()> {
    run(repository, &["config", "--local", "--unset", SSH_COMMAND_KEY], Exit::SuccessOrDifference).map(drop)
}

pub(crate) fn ssh_command_setting(command: &str) -> String {
    format!("{SSH_COMMAND_KEY}={command}")
}

fn public_half(private: &Path) -> PathBuf {
    let mut name = private.as_os_str().to_owned();
    name.push(".");
    name.push(PUBLIC_SUFFIX);
    PathBuf::from(name)
}

fn slashed(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn quoted_for_sh(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

fn make_private_folder(folder: &Path) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(folder, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn find_keygen() -> Option<PathBuf> {
    beside_git().or_else(|| on_path(KEYGEN))
}

fn beside_git() -> Option<PathBuf> {
    let exec_path = run_text(Path::new("."), &["--exec-path"], Exit::Success).ok()?;
    let exec_path = PathBuf::from(exec_path.trim());
    exec_path
        .ancestors()
        .map(|folder| folder.join("usr").join("bin").join(program(KEYGEN)))
        .find(|candidate| candidate.is_file())
}

fn on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|folder| folder.join(program(name)))
        .find(|candidate| candidate.is_file())
}

fn program(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        KeyPair, fingerprint, generate, key_file_name, knows_host, names_key, remember_host_keys,
        same_key, ssh_command,
    };

    const GITHUB_ED25519: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("fastgit-ssh-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_key_file_is_named_after_its_account_and_type() {
        assert_eq!(
            key_file_name("github:ghe.example.com:8443:583231", "ed25519"),
            "github-ghe.example.com-8443-583231_ed25519"
        );
    }

    #[test]
    fn the_fingerprint_matches_the_one_github_publishes_for_its_host_key() {
        assert_eq!(
            fingerprint(GITHUB_ED25519).as_deref(),
            Some("SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU")
        );
        assert_eq!(fingerprint("ssh-ed25519 not-base64!"), None);
        assert!(same_key(&format!("{GITHUB_ED25519} FastGit octocat@github.com"), GITHUB_ED25519));
        assert!(!same_key("ssh-ed25519", "ssh-ed25519"));
    }

    #[test]
    fn host_keys_replace_the_earlier_lines_of_the_same_hosts_only() {
        let folder = scratch("known-hosts");
        let file = folder.join("ssh").join("known_hosts");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "github.com ssh-rsa OLD\nother.example ssh-ed25519 KEEP\n").unwrap();
        remember_host_keys(&file, &["github.com".into(), "ssh.github.com".into()], &[GITHUB_ED25519.into()]).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        let blob = GITHUB_ED25519.split(' ').nth(1).unwrap();
        assert_eq!(
            text,
            format!("other.example ssh-ed25519 KEEP\ngithub.com ssh-ed25519 {blob}\nssh.github.com ssh-ed25519 {blob}\n")
        );
    }

    #[test]
    fn the_ssh_command_quotes_paths_for_sh_and_checks_host_keys() {
        let key = Path::new(r"C:\Users\O'Neil\AppData\Local\FastGit\ssh\github-github.com-1_ed25519");
        let known = Path::new(r"C:\Users\O'Neil\AppData\Local\FastGit\ssh\known_hosts");
        let command = ssh_command(key, known, true);
        assert_eq!(
            command,
            r#"ssh -i 'C:/Users/O'\''Neil/AppData/Local/FastGit/ssh/github-github.com-1_ed25519' -o IdentitiesOnly=yes -o 'UserKnownHostsFile="C:/Users/O'\''Neil/AppData/Local/FastGit/ssh/known_hosts" ~/.ssh/known_hosts' -o StrictHostKeyChecking=yes"#
        );
        assert!(names_key(&command, key));
        assert!(!names_key(&command, Path::new(r"C:\other_ed25519")));
        assert!(ssh_command(key, known, false).ends_with("StrictHostKeyChecking=accept-new"));
    }

    #[test]
    fn ssh_keygen_makes_a_key_pair_that_reads_back_and_is_deleted_whole() {
        let folder = scratch("keygen");
        let private = folder.join("ssh").join(key_file_name("github:github.com:1", "ed25519"));
        let made = generate(&private, "ed25519", "FastGit octocat@github.com").unwrap();
        assert_eq!(made.kind(), "ssh-ed25519");
        assert!(made.public.ends_with("FastGit octocat@github.com"));
        assert!(made.fingerprint().unwrap().starts_with("SHA256:"));
        assert_eq!(KeyPair::read(&private), Some(made.clone()));
        let again = generate(&private, "ed25519", "FastGit octocat@github.com").unwrap();
        assert_ne!(again.public, made.public);
        again.delete().unwrap();
        assert_eq!(KeyPair::read(&private), None);
        assert!(!knows_host(&folder.join("missing"), "github.com"));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
