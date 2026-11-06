use std::{
    ffi::OsString,
    io::Read as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::{Context as _, Result, bail};

use super::{
    access::Access,
    command::{Exit, run, without_console_window},
    ssh_key::ssh_command_setting,
};

const LOOK_EVERY: Duration = Duration::from_millis(50);
const REMOTE_PREFIX: &str = "remote: ";
const LINES_KEPT: usize = 6;
const CANCELLED: &str = "Cancelled";
const GIT_MISSING: &str = "Couldn't run git. Is it installed and on PATH?";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CloneRequest {
    pub(crate) url: String,
    pub(crate) folder: PathBuf,
    pub(crate) access: Access,
    pub(crate) ssh_command: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Progress {
    pub(crate) phase: String,
    pub(crate) percent: Option<u8>,
    pub(crate) done: Option<u64>,
    pub(crate) total: Option<u64>,
}

#[derive(Clone, Default)]
pub(crate) struct Stop(Arc<AtomicBool>);

impl Stop {
    pub(crate) fn now(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn asked(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub(crate) fn clone(request: &CloneRequest, mut progress: impl FnMut(Progress) + Send + 'static, stop: &Stop) -> Result<()> {
    let existed = request.folder.exists();
    let mut args = request.access.config_args();
    args.push("clone".into());
    args.push("--progress".into());
    if let Some(command) = &request.ssh_command {
        args.push("-c".into());
        args.push(ssh_command_setting(command).into());
    }
    args.push("--".into());
    args.push(OsString::from(&request.url));
    args.push(request.folder.as_os_str().to_owned());
    let mut command = Command::new("git");
    command
        .args(args)
        .envs(request.access.environment())
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    without_console_window(&mut command);
    let mut child = command.spawn().context(GIT_MISSING)?;
    let mut stderr = child.stderr.take().context(GIT_MISSING)?;
    let reader = thread::spawn(move || {
        let mut kept: Vec<String> = Vec::new();
        let mut pending = Vec::new();
        let mut chunk = [0; 4096];
        while let Ok(read) = stderr.read(&mut chunk) {
            if read == 0 {
                break;
            }
            pending.extend_from_slice(&chunk[..read]);
            while let Some(end) = pending.iter().position(|byte| matches!(byte, b'\r' | b'\n')) {
                let line = String::from_utf8_lossy(&pending[..end]).trim().to_owned();
                pending.drain(..=end);
                if line.is_empty() {
                    continue;
                }
                match read_progress(&line) {
                    Some(step) => progress(step),
                    None => {
                        kept.push(line);
                        if kept.len() > LINES_KEPT {
                            kept.remove(0);
                        }
                    }
                }
            }
        }
        kept
    });
    let status = loop {
        if stop.asked() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            if !existed {
                let _ = std::fs::remove_dir_all(&request.folder);
            }
            bail!(CANCELLED);
        }
        match child.try_wait()? {
            Some(status) => break status,
            None => thread::sleep(LOOK_EVERY),
        }
    };
    let kept = reader.join().unwrap_or_default();
    if !status.success() {
        bail!("{}", kept.join("\n"));
    }
    Ok(())
}

pub(crate) fn read_progress(line: &str) -> Option<Progress> {
    let line = line.strip_prefix(REMOTE_PREFIX).unwrap_or(line);
    let (phase, rest) = line.split_once(':')?;
    let rest = rest.trim_start();
    let (percent, rest) = rest.split_once('%')?;
    let percent: u8 = percent.trim().parse().ok()?;
    let counts = rest
        .split_once('(')
        .and_then(|(_, after)| after.split_once(')'))
        .and_then(|(inside, _)| inside.split_once('/'));
    let (done, total) = match counts {
        Some((done, total)) => (done.trim().parse().ok(), total.trim().parse().ok()),
        None => (None, None),
    };
    Some(Progress {
        phase: phase.trim().to_owned(),
        percent: Some(percent.min(100)),
        done,
        total,
    })
}

pub(crate) fn init(folder: &Path) -> Result<()> {
    std::fs::create_dir_all(folder)?;
    run(folder, &["init", "--quiet"], Exit::Success).map(drop)
}

pub(crate) fn add_remote(repository: &Path, name: &str, url: &str) -> Result<()> {
    run(repository, &["remote", "add", name, url], Exit::Success).map(drop)
}

pub(crate) fn is_empty_or_missing(folder: &Path) -> bool {
    std::fs::read_dir(folder).map_or(true, |mut entries| entries.next().is_none())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{CloneRequest, Progress, Stop, add_remote, clone, init, is_empty_or_missing, read_progress};
    use crate::git::{
        Access,
        scratch::ScratchRepo,
        ssh_key::ssh_command_of,
    };

    #[test]
    fn progress_lines_give_their_phase_percent_and_counts() {
        assert_eq!(
            read_progress("Receiving objects:  45% (1234/2742), 1.20 MiB | 2.00 MiB/s"),
            Some(Progress { phase: "Receiving objects".into(), percent: Some(45), done: Some(1234), total: Some(2742) })
        );
        assert_eq!(
            read_progress("remote: Counting objects: 100% (4/4), done."),
            Some(Progress { phase: "Counting objects".into(), percent: Some(100), done: Some(4), total: Some(4) })
        );
        assert_eq!(read_progress("Cloning into 'sternegit'..."), None);
        assert_eq!(read_progress("remote: Enumerating objects: 5, done."), None);
    }

    #[test]
    fn a_clone_copies_the_history_and_keeps_the_ssh_command_it_was_given() -> anyhow::Result<()> {
        let origin = ScratchRepo::new("clone-origin")?;
        origin.commit("First")?;
        let target = std::env::temp_dir().join(format!("sternegit-clone-target-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&target);
        let request = CloneRequest {
            url: origin.location(),
            folder: target.clone(),
            access: Access::default(),
            ssh_command: Some("ssh -o IdentitiesOnly=yes".into()),
        };
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = seen.clone();
        clone(&request, move |step| recorded.lock().unwrap().push(step), &Stop::default())?;
        let command = ssh_command_of(&target);
        let head = std::fs::read_to_string(target.join(".git").join("HEAD"))?;
        let _ = std::fs::remove_dir_all(&target);
        assert_eq!(command.as_deref(), Some("ssh -o IdentitiesOnly=yes"));
        assert!(head.starts_with("ref: refs/heads/"));
        Ok(())
    }

    #[test]
    fn a_failed_clone_reports_gits_reason() {
        let target = std::env::temp_dir().join(format!("sternegit-clone-missing-{}", std::process::id()));
        let request = CloneRequest {
            url: target.join("nowhere").display().to_string(),
            folder: target.clone(),
            access: Access::default(),
            ssh_command: None,
        };
        let failed = clone(&request, |_| {}, &Stop::default()).unwrap_err();
        let _ = std::fs::remove_dir_all(&target);
        assert!(failed.to_string().contains("nowhere"), "{failed}");
    }

    #[test]
    fn a_stopped_clone_leaves_no_folder_behind() {
        let target = std::env::temp_dir().join(format!("sternegit-clone-stopped-{}", std::process::id()));
        let stop = Stop::default();
        stop.now();
        let request = CloneRequest {
            url: "https://127.0.0.1:9/never.git".into(),
            folder: target.clone(),
            access: Access::default(),
            ssh_command: None,
        };
        let stopped = clone(&request, |_| {}, &stop).unwrap_err();
        assert_eq!(stopped.to_string(), "Cancelled");
        assert!(!target.exists());
    }

    #[test]
    fn a_new_repository_gets_its_folder_and_an_origin() -> anyhow::Result<()> {
        let folder = std::env::temp_dir().join(format!("sternegit-init-{}", std::process::id())).join("starfall");
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
        assert!(is_empty_or_missing(&folder));
        init(&folder)?;
        add_remote(&folder, "origin", "https://github.com/octocat/starfall.git")?;
        let config = std::fs::read_to_string(folder.join(".git").join("config"))?;
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
        assert!(config.contains("url = https://github.com/octocat/starfall.git"), "{config}");
        Ok(())
    }
}
