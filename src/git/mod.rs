mod access;
mod branch;
mod clone;
mod command;
mod commit;
mod context;
mod credential;
mod facts;
mod file_history;
mod graph;
mod grep;
mod history;
mod hunks;
mod index;
mod lfs;
mod markers;
mod merge;
mod patch;
mod pick;
mod pull_head;
mod pull_mode;
mod reach;
mod refs;
mod refusal;
mod remote;
mod reset;
mod reword;
pub(crate) mod ssh_key;
mod stash;
mod status;
mod tag;
mod text;
mod time;
mod underway;
pub(crate) mod url_settings;
mod version;
mod worktree;

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context as _, Result};
use gix::ThreadSafeRepository;
use gpui_kit::SharedString;

pub(crate) use access::{Access, Attendance, CredentialScope, GIT_CREDENTIAL_FLAG, renew_tokens_with};
pub(crate) use branch::{CheckedOut, NameProblem, new_branch_name};
pub(crate) use clone::{CloneRequest, Progress, Stop, add_remote, clone, init, is_empty_or_missing};
pub(crate) use command::without_console_window;
pub(crate) use commit::commit_message;
pub(crate) use credential::{CredentialAnswer, CredentialRequest};
pub(crate) use facts::DifftoolTarget;
pub(crate) use file_history::{FileCommit, LineCounts};
pub(crate) use graph::{Edge, Row};
#[cfg(test)]
pub(crate) use history::fixtures;
pub(crate) use history::{Commit, HISTORY_LIMIT, History, RowKind, short_id};
pub(crate) use hunks::{Hunks, LineAt, LineSide, LineSpan, Part, PartAction};
pub(crate) use index::Scope;
pub(crate) use lfs::FileVersion;
pub(crate) use markers::{Marked, Piece, has_markers, split_conflicts};
pub(crate) use merge::{Merged, Rebased};
pub(crate) use patch::{Baseline, FilePatch, Whitespace};
pub(crate) use pick::Picked;
pub(crate) use pull_head::{BranchSpan, NamedRemote, PullSource, fork_branch_name};
pub(crate) use pull_mode::PullMode;
pub(crate) use refs::{AheadBehind, RefDigest, RefKind, RefLabel, RemoteBranch, ShownRef};
pub(crate) use refusal::{Refusal, Tracking};
pub(crate) use remote::{Behind, CaughtUp, Pulled, Pushed};
pub(crate) use reset::ResetMode;
pub(crate) use reword::RewordScope;
pub(crate) use stash::{Carried, Stash};
pub(crate) use status::{ChangeKind, FileChange, Side, WorkingChanges};
pub(crate) use tag::{TagPushed, new_tag_name};
pub(crate) use underway::{Continued, Picking, Rebasing, Underway};
#[cfg(test)]
pub(crate) use underway::{Merging, Replaying};
pub(crate) use version::{Conflicted, Revision, Version};
pub(crate) use worktree::{
    FolderLeft, Worktree, WorktreeChanges, WorktreeStart, changes_elsewhere, folder_is_free,
    folder_of_worktrees, is_linked_worktree, suggested_folder,
};

const OBJECT_CACHE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct Repo {
    repo: ThreadSafeRepository,
    location: PathBuf,
    name: SharedString,
    has_work_tree: bool,
    access: Access,
}

impl Repo {
    pub(crate) fn open(path: &Path) -> Result<(Self, History)> {
        let started = Instant::now();
        let mut repo = gix::discover(path)
            .with_context(|| format!("{} isn't inside a Git repository", path.display()))?;
        decode_each_commit_once(&mut repo);
        let history = History::read(&repo, started, &top_folder_of(&repo))?;
        Ok((Self::new(repo), history))
    }

    fn new(repo: gix::Repository) -> Self {
        let has_work_tree = repo.workdir().is_some();
        let location = top_folder_of(&repo);
        let name = folder_name(&location);
        Self {
            repo: repo.into_sync(),
            location,
            name,
            has_work_tree,
            access: Access::default(),
        }
    }

    pub(crate) fn with_access(self, access: Access) -> Self {
        Self { access, ..self }
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

pub(crate) fn top_folder(path: &Path) -> Option<PathBuf> {
    gix::discover(path).ok().map(|repo| top_folder_of(&repo))
}

fn top_folder_of(repo: &gix::Repository) -> PathBuf {
    absolute(repo.workdir().unwrap_or_else(|| repo.git_dir()))
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
            let dir = std::env::temp_dir().join(format!("sternegit-{label}-{}", std::process::id()));
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

        pub(crate) fn bare(label: &str) -> Result<Self> {
            let dir = std::env::temp_dir().join(format!("sternegit-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)?;
            let scratch = Self { dir };
            scratch.git(&["init", "--bare", "--quiet", "--template="])?;
            Ok(scratch)
        }

        pub(crate) fn tracking(label: &str, origin: &ScratchRepo) -> Result<Self> {
            let scratch = Self::new(label)?;
            scratch.git(&["remote", "add", "origin", &origin.location()])?;
            scratch.git(&["fetch", "--quiet", "origin"])?;
            let upstream = format!("origin/{DEFAULT_BRANCH}");
            scratch.git(&["checkout", "--quiet", "-B", DEFAULT_BRANCH, &upstream])?;
            Ok(scratch)
        }

        pub(crate) fn location(&self) -> String {
            self.dir.display().to_string()
        }

        pub(crate) fn head(&self) -> Result<String> {
            Ok(self.git(&["rev-parse", "HEAD"])?.trim().to_owned())
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

    const LOOKALIKE_BLOCKS: usize = 100;
    const BLOCKS_BETWEEN_INSERTIONS: usize = 10;
    const BLOCKS_PER_INSERTION: usize = 10;
    const LINES_PER_BLOCK: usize = 8;
    const FIRST_INSERTED_BLOCK: usize = 1_000_000;
    pub(crate) const INSERTED_LINES: usize =
        LOOKALIKE_BLOCKS / BLOCKS_BETWEEN_INSERTIONS * BLOCKS_PER_INSERTION * LINES_PER_BLOCK;

    pub(crate) fn lookalike_blocks() -> String {
        (0..LOOKALIKE_BLOCKS).map(lookalike_block).collect()
    }

    pub(crate) fn lookalike_blocks_with_insertions() -> String {
        let mut inserted = FIRST_INSERTED_BLOCK..;
        (0..LOOKALIKE_BLOCKS)
            .flat_map(|id| {
                let insertion = match id % BLOCKS_BETWEEN_INSERTIONS == BLOCKS_BETWEEN_INSERTIONS / 2 {
                    true => inserted.by_ref().take(BLOCKS_PER_INSERTION).collect(),
                    false => Vec::new(),
                };
                insertion.into_iter().chain([id])
            })
            .map(lookalike_block)
            .collect()
    }

    fn lookalike_block(id: usize) -> String {
        let kind = id % 11;
        let value = kind % 5;
        format!(
            "--- !u!114 &{id}\nMonoBehaviour:\n  m_ObjectHideFlags: 0\n  m_GameObject: {{fileID: {kind}}}\n  m_Enabled: 1\n  m_Value: {value}\n  m_Color: {{r: 1, g: 1, b: 1, a: 1}}\n  m_Items: []\n"
        )
    }

    pub(crate) fn added_and_removed_lines(patch: &str) -> (usize, usize) {
        let mut in_hunk = false;
        let (mut added, mut removed) = (0, 0);
        for line in patch.lines() {
            match line.as_bytes().first() {
                _ if line.starts_with("diff ") => in_hunk = false,
                _ if line.starts_with("@@") => in_hunk = true,
                Some(b'+') if in_hunk => added += 1,
                Some(b'-') if in_hunk => removed += 1,
                _ => {}
            }
        }
        (added, removed)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{scratch::ScratchRepo, top_folder};

    #[test]
    fn a_folder_inside_a_repository_leads_to_the_repository_top_folder() -> Result<()> {
        let scratch = ScratchRepo::new("top-folder")?;
        let inside = scratch.dir.join("src").join("deep");
        std::fs::create_dir_all(&inside)?;
        assert_eq!(top_folder(&inside), Some(scratch.dir.clone()));
        Ok(())
    }
}
