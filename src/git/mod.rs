mod command;
mod commit;
mod graph;
mod history;
mod hunks;
mod index;
mod patch;
mod refs;
mod remote;
mod status;
mod text;
mod time;

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context as _, Result};
use gix::ThreadSafeRepository;
use gpui_kit::SharedString;

pub(crate) use commit::commit_message;
pub(crate) use graph::{Edge, Row};
#[cfg(test)]
pub(crate) use history::fixtures;
pub(crate) use history::{Commit, HISTORY_LIMIT, History, short_id};
pub(crate) use hunks::{Hunks, LineAt, LineSide, LineSpan, Part, PartAction};
pub(crate) use index::Scope;
pub(crate) use patch::FilePatch;
pub(crate) use refs::{RefDigest, RefKind, RefLabel};
pub(crate) use status::{ChangeKind, FileChange, Side, WorkingChanges};

const OBJECT_CACHE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct Repo {
    repo: ThreadSafeRepository,
    location: PathBuf,
    name: SharedString,
    has_work_tree: bool,
}

impl Repo {
    pub(crate) fn open(path: &Path) -> Result<(Self, History)> {
        let started = Instant::now();
        let mut repo = gix::discover(path)
            .with_context(|| format!("{} isn't inside a Git repository", path.display()))?;
        decode_each_commit_once(&mut repo);
        let history = History::read(&repo, started)?;
        Ok((Self::new(repo), history))
    }

    fn new(repo: gix::Repository) -> Self {
        let has_work_tree = repo.workdir().is_some();
        let location = absolute(repo.workdir().unwrap_or_else(|| repo.git_dir()));
        let name = folder_name(&location);
        Self {
            repo: repo.into_sync(),
            location,
            name,
            has_work_tree,
        }
    }

    pub(crate) fn name(&self) -> SharedString {
        self.name.clone()
    }

    pub(crate) fn location(&self) -> &Path {
        &self.location
    }

    pub(crate) fn has_work_tree(&self) -> bool {
        self.has_work_tree
    }

    pub(crate) fn ref_digest(&self) -> Result<RefDigest> {
        RefDigest::of(&self.local())
    }

    fn local(&self) -> gix::Repository {
        self.repo.to_thread_local()
    }
}

pub(crate) fn is_inside_repository(path: &Path) -> bool {
    gix::discover(path).is_ok()
}

pub(crate) fn folder_name(path: &Path) -> SharedString {
    path.file_name()
        .map_or_else(
            || path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
        .into()
}

fn absolute(location: &Path) -> PathBuf {
    std::path::absolute(location).unwrap_or_else(|_| location.to_owned())
}

fn decode_each_commit_once(repo: &mut gix::Repository) {
    repo.object_cache_size_if_unset(OBJECT_CACHE_BYTES);
}

#[cfg(test)]
pub(crate) mod scratch {
    use std::{io::Write as _, path::PathBuf};

    use anyhow::Result;

    use super::{
        History, Repo,
        command::{Exit, run_text, run_with_input},
    };

    pub(crate) const DEFAULT_BRANCH: &str = "main";

    const CONFIG_THAT_IGNORES_THE_USERS: &str = "[user]
	name = Test
	email = test@example.com
[commit]
	gpgsign = false
[core]
	autocrlf = false
	safecrlf = false
	hooksPath = no-hooks
	logAllRefUpdates = true
[merge]
	ff = true
	verifySignatures = false
";

    pub(crate) struct ScratchRepo {
        pub(crate) dir: PathBuf,
    }

    impl ScratchRepo {
        pub(crate) fn new(label: &str) -> Result<Self> {
            let dir = std::env::temp_dir().join(format!("fastgit-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)?;
            let scratch = Self { dir };
            scratch.git(&["init", "--quiet", "--template="])?;
            let default_branch = format!("refs/heads/{DEFAULT_BRANCH}");
            scratch.git(&["symbolic-ref", "HEAD", &default_branch])?;
            std::fs::OpenOptions::new()
                .append(true)
                .open(scratch.dir.join(".git").join("config"))?
                .write_all(CONFIG_THAT_IGNORES_THE_USERS.as_bytes())?;
            Ok(scratch)
        }

        pub(crate) fn git(&self, args: &[&str]) -> Result<String> {
            run_text(&self.dir, args, Exit::Success)
        }

        pub(crate) fn commit(&self, message: &str) -> Result<String> {
            self.git(&[
                "commit",
                "--allow-empty",
                "--no-verify",
                "--quiet",
                "-m",
                message,
            ])
        }

        pub(crate) fn commit_file_named(&self, name: &[u8], contents: &str) -> Result<()> {
            self.write("contents.txt", contents)?;
            let blob = self.git(&["hash-object", "-w", "contents.txt"])?;
            std::fs::remove_file(self.dir.join("contents.txt"))?;
            let mut entry = format!("100644 {}\t", blob.trim()).into_bytes();
            entry.extend_from_slice(name);
            entry.push(0);
            run_with_input(&self.dir, &["update-index", "-z", "--index-info"], &entry)?;
            self.commit("Add a file")?;
            Ok(())
        }

        pub(crate) fn write(&self, path: &str, contents: &str) -> Result<()> {
            Ok(std::fs::write(self.dir.join(path), contents)?)
        }

        pub(crate) fn open(&self) -> Result<(Repo, History)> {
            Repo::open(&self.dir)
        }
    }

    impl Drop for ScratchRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}
