use std::{
    hash::{DefaultHasher, Hash as _, Hasher as _},
    path::Path,
};

use anyhow::Result;
use gix::{ObjectId, bstr::ByteSlice as _};
use gpui_kit::SharedString;

use super::{
    Repo,
    command::{Exit, run},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
}

impl ChangeKind {
    pub(crate) fn status_letter(self) -> &'static str {
        match self {
            ChangeKind::Added => "A",
            ChangeKind::Modified => "M",
            ChangeKind::Deleted => "D",
            ChangeKind::Renamed => "R",
            ChangeKind::Copied => "C",
            ChangeKind::Untracked => "?",
            ChangeKind::Conflicted => "U",
        }
    }

    fn from_status_code(code: u8) -> Option<Self> {
        Some(match code {
            b'A' => ChangeKind::Added,
            b'M' | b'T' => ChangeKind::Modified,
            b'D' => ChangeKind::Deleted,
            b'R' => ChangeKind::Renamed,
            b'C' => ChangeKind::Copied,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileChange {
    pub(crate) path: SharedString,
    pub(crate) original_path: Option<SharedString>,
    pub(crate) kind: ChangeKind,
}

impl FileChange {
    pub(crate) fn paths(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.path.as_str()).chain(self.original_path.as_deref())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Unstaged,
    Staged,
}

impl Side {
    pub(crate) fn other(self) -> Self {
        match self {
            Side::Unstaged => Side::Staged,
            Side::Staged => Side::Unstaged,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusDigest(u64);

impl StatusDigest {
    fn of(
        status: &[u8],
        head: Option<ObjectId>,
        work_tree: &Path,
        unstaged: &[FileChange],
    ) -> Self {
        let mut hasher = DefaultHasher::new();
        status.hash(&mut hasher);
        head.hash(&mut hasher);
        for change in unstaged {
            hash_size_and_modified_time(&work_tree.join(change.path.as_str()), &mut hasher);
        }
        Self(hasher.finish())
    }
}

fn hash_size_and_modified_time(path: &Path, hasher: &mut DefaultHasher) {
    if let Ok(metadata) = std::fs::metadata(path) {
        metadata.len().hash(hasher);
        metadata.modified().ok().hash(hasher);
    }
}

#[derive(Default, Debug)]
pub(crate) struct WorkingChanges {
    pub(crate) unstaged: Vec<FileChange>,
    pub(crate) staged: Vec<FileChange>,
    pub(crate) head: Option<ObjectId>,
    pub(crate) digest: StatusDigest,
}

impl WorkingChanges {
    pub(crate) fn is_empty(&self) -> bool {
        self.unstaged.is_empty() && self.staged.is_empty()
    }

    pub(crate) fn files(&self, side: Side) -> &[FileChange] {
        match side {
            Side::Unstaged => &self.unstaged,
            Side::Staged => &self.staged,
        }
    }

    pub(crate) fn find(&self, side: Side, path: &str) -> Option<&FileChange> {
        self.files(side).iter().find(|change| change.path == path)
    }
}

impl Repo {
    pub(crate) fn status(&self) -> Result<WorkingChanges> {
        let output = run(
            &self.location,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--find-renames",
            ],
            Exit::Success,
        )?;
        let head_after_status = self.local().head_id().ok().map(|id| id.detach());
        let mut changes = parse_status(&output);
        changes.head = head_after_status;
        changes.digest = StatusDigest::of(
            &output,
            head_after_status,
            &self.location,
            &changes.unstaged,
        );
        Ok(changes)
    }
}

fn parse_status(output: &[u8]) -> WorkingChanges {
    let mut changes = WorkingChanges::default();
    let mut entries = output
        .split(|&byte| byte == 0)
        .filter(|entry| !entry.is_empty());
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let (index, worktree) = (entry[0], entry[1]);
        let path: SharedString = entry[3..].to_str_lossy().into_owned().into();
        let original_path = matches!(index, b'R' | b'C')
            .then(|| entries.next())
            .flatten()
            .map(|path| SharedString::from(path.to_str_lossy().into_owned()));
        let conflicted = matches!(
            (index, worktree),
            (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D')
        );
        if conflicted {
            changes.unstaged.push(FileChange {
                path,
                original_path: None,
                kind: ChangeKind::Conflicted,
            });
            continue;
        }
        if index == b'?' {
            changes.unstaged.push(FileChange {
                path,
                original_path: None,
                kind: ChangeKind::Untracked,
            });
            continue;
        }
        if let Some(kind) = ChangeKind::from_status_code(index) {
            changes.staged.push(FileChange {
                path: path.clone(),
                original_path,
                kind,
            });
        }
        if let Some(kind) = ChangeKind::from_status_code(worktree) {
            changes.unstaged.push(FileChange {
                path,
                original_path: None,
                kind,
            });
        }
    }
    let by_path = |a: &FileChange, b: &FileChange| a.path.as_str().cmp(b.path.as_str());
    changes.unstaged.sort_by(by_path);
    changes.staged.sort_by(by_path);
    changes
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{ChangeKind, FileChange, parse_status};
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn status_splits_staged_and_unstaged_changes() {
        let status =
            parse_status(b"MM both.rs\0R  new.rs\0old.rs\0?? fresh.txt\0UU clash.rs\0 D gone.rs\0");
        let paths = |files: &[FileChange]| {
            files
                .iter()
                .map(|file| (file.path.to_string(), file.kind))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            paths(&status.staged),
            [
                ("both.rs".into(), ChangeKind::Modified),
                ("new.rs".into(), ChangeKind::Renamed)
            ]
        );
        assert_eq!(status.staged[1].original_path.as_deref(), Some("old.rs"));
        assert_eq!(
            paths(&status.unstaged),
            [
                ("both.rs".into(), ChangeKind::Modified),
                ("clash.rs".into(), ChangeKind::Conflicted),
                ("fresh.txt".into(), ChangeKind::Untracked),
                ("gone.rs".into(), ChangeKind::Deleted),
            ]
        );
    }

    #[test]
    fn a_commit_that_leaves_the_tree_clean_still_changes_the_digest() -> Result<()> {
        let scratch = ScratchRepo::new("clean-commit")?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        let before = repo.status()?;
        scratch.commit("second")?;
        let after = repo.status()?;
        let (_, history) = scratch.open()?;
        assert!(before.is_empty() && after.is_empty());
        assert_eq!(
            after.head, history.head_id,
            "the head the status was read at"
        );
        assert_ne!(before.head, after.head);
        assert_ne!(before.digest, after.digest);
        Ok(())
    }

    #[test]
    fn a_second_edit_to_a_modified_file_changes_the_digest() -> Result<()> {
        let scratch = ScratchRepo::new("edit-twice")?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", "one\ntwo\n")?;
        let (repo, _) = scratch.open()?;
        let first = repo.status()?;
        scratch.write("a.txt", "one\ntwo\nthree\n")?;
        let second = repo.status()?;
        assert_eq!(first.unstaged, second.unstaged);
        assert_ne!(first.digest, second.digest);
        Ok(())
    }
}
