use std::{
    collections::HashSet,
    ffi::OsStr,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    path::Path,
    sync::Arc,
};

use anyhow::Result;
use gix::ObjectId;
use gpui_kit::SharedString;

use super::{
    Repo,
    command::{Exit, READ_ONLY, run},
    text::{decode, os_name},
    underway::{Underway, underway_in},
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

    pub(super) fn from_status_code(code: u8) -> Option<Self> {
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
    path_as_git_wrote_it: Arc<[u8]>,
    original_path_as_git_wrote_it: Option<Arc<[u8]>>,
}

impl FileChange {
    pub(crate) fn new(path: &[u8], original_path: Option<&[u8]>, kind: ChangeKind) -> Self {
        Self {
            path: decode(path).into(),
            original_path: original_path.map(|path| decode(path).into()),
            kind,
            path_as_git_wrote_it: path.into(),
            original_path_as_git_wrote_it: original_path.map(Into::into),
        }
    }

    pub(crate) fn name_as_git_wrote_it(&self) -> &[u8] {
        &self.path_as_git_wrote_it
    }

    pub(crate) fn names_as_git_wrote_them(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(self.name_as_git_wrote_it())
            .chain(self.original_path_as_git_wrote_it.as_deref())
    }

    pub(crate) fn exact_path(&self) -> &OsStr {
        os_name(&self.path_as_git_wrote_it, &self.path)
    }

    pub(crate) fn exact_paths(&self) -> impl Iterator<Item = &OsStr> {
        let original = self
            .original_path_as_git_wrote_it
            .as_deref()
            .zip(self.original_path.as_deref())
            .map(|(bytes, shown)| os_name(bytes, shown));
        std::iter::once(self.exact_path()).chain(original)
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
        index: &Path,
        work_tree: &Path,
        changes: &WorkingChanges,
    ) -> Self {
        let mut hasher = DefaultHasher::new();
        status.hash(&mut hasher);
        head.hash(&mut hasher);
        changes.underway.hash(&mut hasher);
        hash_size_and_modified_time(index, &mut hasher);
        for change in changes.conflicted.iter().chain(&changes.unstaged) {
            hash_size_and_modified_time(&work_tree.join(change.exact_path()), &mut hasher);
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
    pub(crate) conflicted: Vec<FileChange>,
    pub(crate) unstaged: Vec<FileChange>,
    pub(crate) staged: Vec<FileChange>,
    pub(crate) head: Option<ObjectId>,
    pub(crate) underway: Option<Underway>,
    pub(crate) digest: StatusDigest,
}

impl WorkingChanges {
    pub(crate) fn is_empty(&self) -> bool {
        self.conflicted.is_empty() && self.unstaged.is_empty() && self.staged.is_empty()
    }

    pub(crate) fn files(&self, side: Side) -> &[FileChange] {
        match side {
            Side::Unstaged => &self.unstaged,
            Side::Staged => &self.staged,
        }
    }

    pub(crate) fn file_count(&self) -> usize {
        self.conflicted
            .iter()
            .chain(&self.unstaged)
            .chain(&self.staged)
            .map(|change| change.path.as_ref())
            .collect::<HashSet<&str>>()
            .len()
    }

    pub(crate) fn find(&self, side: Side, path: &str) -> Option<&FileChange> {
        let conflicted = match side {
            Side::Unstaged => self.conflicted.as_slice(),
            Side::Staged => &[],
        };
        conflicted
            .iter()
            .chain(self.files(side))
            .find(|change| change.path == path)
    }

    pub(crate) fn is_conflicted(&self, path: &str) -> bool {
        self.conflicted.iter().any(|change| change.path == path)
    }
}

impl Repo {
    pub(crate) fn status(&self) -> Result<WorkingChanges> {
        let output = run(
            &self.location,
            &[
                READ_ONLY,
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--find-renames",
            ],
            Exit::Success,
        )?;
        let local = self.local();
        let head_after_status = local.head_id().ok().map(|id| id.detach());
        let mut changes = parse_status(&output);
        changes.head = head_after_status;
        changes.underway = underway_in(&local);
        changes.digest = StatusDigest::of(
            &output,
            head_after_status,
            &local.index_path(),
            &self.location,
            &changes,
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
        let path = &entry[3..];
        let original_path = matches!(index, b'R' | b'C')
            .then(|| entries.next())
            .flatten();
        let conflicted = matches!(
            (index, worktree),
            (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D')
        );
        if conflicted {
            changes
                .conflicted
                .push(FileChange::new(path, None, ChangeKind::Conflicted));
            continue;
        }
        if index == b'?' {
            changes
                .unstaged
                .push(FileChange::new(path, None, ChangeKind::Untracked));
            continue;
        }
        if let Some(kind) = ChangeKind::from_status_code(index) {
            changes
                .staged
                .push(FileChange::new(path, original_path, kind));
        }
        if let Some(kind) = ChangeKind::from_status_code(worktree) {
            changes.unstaged.push(FileChange::new(path, None, kind));
        }
    }
    let by_path = |a: &FileChange, b: &FileChange| a.path.as_str().cmp(b.path.as_str());
    changes.conflicted.sort_by(by_path);
    changes.unstaged.sort_by(by_path);
    changes.staged.sort_by(by_path);
    changes
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{ChangeKind, FileChange, parse_status};
    use crate::git::{FilePatch, PartAction, Side, scratch::ScratchRepo};

    #[test]
    fn a_file_in_both_lists_counts_once() {
        let status = parse_status(b"MM both.rs\0A  new.rs\0?? fresh.txt\0");
        assert_eq!(status.file_count(), 3);
    }

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
            paths(&status.conflicted),
            [("clash.rs".into(), ChangeKind::Conflicted)]
        );
        assert_eq!(
            paths(&status.unstaged),
            [
                ("both.rs".into(), ChangeKind::Modified),
                ("fresh.txt".into(), ChangeKind::Untracked),
                ("gone.rs".into(), ChangeKind::Deleted),
            ]
        );
    }

    #[test]
    fn a_name_that_is_not_utf8_keeps_the_bytes_git_wrote() {
        let status = parse_status(b" M caf\xE9.txt\0R  n\xE9w.txt\0\xF6ld.txt\0");
        let modified = &status.unstaged[0];
        assert_eq!(&*modified.path, "café.txt");
        assert_eq!(&*modified.path_as_git_wrote_it, b"caf\xE9.txt");
        let renamed = &status.staged[0];
        assert_eq!(renamed.original_path.as_deref(), Some("öld.txt"));
        assert_eq!(
            renamed.original_path_as_git_wrote_it.as_deref(),
            Some(&b"\xF6ld.txt"[..])
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
    fn staging_more_of_a_file_already_in_both_lists_changes_the_digest() -> Result<()> {
        let scratch = ScratchRepo::new("stage-more")?;
        let lines = |edited: &[usize]| -> String {
            (1..=20)
                .map(|n| match edited.contains(&n) {
                    true => format!("edited {n}\n"),
                    false => format!("line {n}\n"),
                })
                .collect()
        };
        scratch.write("a.txt", &lines(&[]))?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", &lines(&[2, 10, 18]))?;
        let (repo, _) = scratch.open()?;
        let stage_first_hunk = || -> Result<()> {
            let change = repo.status()?.unstaged[0].clone();
            let FilePatch::Diff {
                hunks: Some(hunks), ..
            } = repo.file_patch(&change, Side::Unstaged)?
            else {
                anyhow::bail!("expected hunks");
            };
            let patch = hunks.patch(&hunks.hunk(0), PartAction::Stage);
            repo.apply_part(PartAction::Stage, &patch.expect("a hunk"))
        };
        stage_first_hunk()?;
        let first = repo.status()?;
        std::thread::sleep(std::time::Duration::from_millis(20));
        stage_first_hunk()?;
        let second = repo.status()?;
        assert_eq!(first.unstaged, second.unstaged);
        assert_eq!(first.staged, second.staged);
        assert_ne!(first.digest, second.digest);
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
