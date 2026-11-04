use gpui_kit::{
    AnyElement, App, IntoElement, SharedString, Window,
    component::diff::{DiffFile, DiffFileStatus},
    div,
};

use crate::{
    git::{ChangeKind, FilePatch},
    ui::{self, Load},
};

#[derive(Clone, Copy)]
pub(crate) struct PatchStats {
    pub(crate) files: usize,
    pub(crate) additions: usize,
    pub(crate) deletions: usize,
}

impl PatchStats {
    pub(crate) fn of(files: &[DiffFile]) -> Self {
        let (additions, deletions) = files.iter().fold((0, 0), |(added, deleted), file| {
            (added + file.additions(), deleted + file.deletions())
        });
        Self {
            files: files.len(),
            additions,
            deletions,
        }
    }
}

#[derive(Default)]
pub(crate) struct RedrawAfterMeasuring {
    due: bool,
}

impl RedrawAfterMeasuring {
    pub(crate) fn files_changed(&mut self) {
        self.due = true;
    }

    pub(crate) fn request(&mut self, window: &Window) {
        if std::mem::take(&mut self.due) {
            window.request_animation_frame();
        }
    }
}

pub(crate) struct PatchTexts {
    pub(crate) loading: &'static str,
    pub(crate) failed: &'static str,
    pub(crate) empty: &'static str,
}

pub(crate) fn patch_content<T>(
    load: &Load<T>,
    stats: impl Fn(&T) -> PatchStats,
    texts: &PatchTexts,
    ready: impl FnOnce(&T) -> AnyElement,
    cx: &App,
) -> AnyElement {
    match load {
        Load::Idle => div().into_any_element(),
        Load::Loading { .. } => ui::loading(texts.loading),
        Load::Failed(failure) => ui::failure(texts.failed, failure.message(), cx),
        Load::Refreshing { value, .. } | Load::Ready(value) if stats(value).files == 0 => {
            ui::hint(texts.empty, cx)
        }
        Load::Refreshing { value, .. } | Load::Ready(value) => ready(value),
    }
}

pub(crate) fn change_kind(file: &DiffFile) -> ChangeKind {
    match file.status() {
        DiffFileStatus::Added => ChangeKind::Added,
        DiffFileStatus::Deleted => ChangeKind::Deleted,
        DiffFileStatus::Renamed => ChangeKind::Renamed,
        DiffFileStatus::Copied => ChangeKind::Copied,
        DiffFileStatus::Conflicted => ChangeKind::Conflicted,
        _ => ChangeKind::Modified,
    }
}

pub(crate) fn renamed_from(file: &DiffFile) -> Option<&SharedString> {
    file.original_path()
        .filter(|original| *original != file.path())
}

pub(crate) fn parse_patch(patch: &str) -> anyhow::Result<Vec<DiffFile>> {
    DiffFile::parse(patch).map_err(|error| anyhow::anyhow!("{error}"))
}

pub(crate) fn file_patch_files(patch: FilePatch) -> anyhow::Result<Vec<DiffFile>> {
    match patch {
        FilePatch::Diff(text) => parse_patch(&text),
        FilePatch::Conflicted { path, contents } => Ok(vec![
            DiffFile::parse_conflicts(&path, &contents)
                .unwrap_or_else(|_| DiffFile::unchanged(&path, &contents)),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{PatchStats, change_kind, file_patch_files, parse_patch, renamed_from};
    use crate::git::{
        ChangeKind, Side,
        scratch::{DEFAULT_BRANCH, ScratchRepo},
    };

    fn counts(stats: PatchStats) -> (usize, usize, usize) {
        (stats.files, stats.additions, stats.deletions)
    }

    #[test]
    fn what_git_prints_parses_into_the_files_shown() -> Result<()> {
        let scratch = ScratchRepo::new("parse")?;
        scratch.write(
            "a.txt", "one
",
        )?;
        let (repo, _) = scratch.open()?;
        let untracked = repo.status()?.unstaged[0].clone();
        let files = file_patch_files(repo.file_patch(&untracked, Side::Unstaged)?)?;
        assert_eq!(counts(PatchStats::of(&files)), (1, 1, 0));

        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write(
            "a.txt", "one
two
",
        )?;
        scratch.git(&["add", "a.txt"])?;
        let (repo, history) = scratch.open()?;
        let staged = repo.status()?.staged[0].clone();
        let files = file_patch_files(repo.file_patch(&staged, Side::Staged)?)?;
        assert_eq!(counts(PatchStats::of(&files)), (1, 1, 0));

        let first = repo.commit_details(history.commits[0].id)?;
        assert_eq!(
            counts(PatchStats::of(&parse_patch(&first.patch)?)),
            (1, 1, 0)
        );
        Ok(())
    }

    #[test]
    fn a_file_named_in_latin_1_reads_in_its_commit() -> Result<()> {
        let scratch = ScratchRepo::new("latin-1-name-commit")?;
        scratch.commit_file_named(b"caf\xE9.txt", "un\n")?;
        let (repo, history) = scratch.open()?;
        let patch = repo.commit_details(history.commits[0].id)?.patch;
        let files = parse_patch(&patch)?;
        assert_eq!(files[0].path().as_str(), "café.txt");
        assert_eq!(counts(PatchStats::of(&files)), (1, 1, 0));
        Ok(())
    }

    #[test]
    fn each_file_of_a_commit_says_how_it_changed() -> Result<()> {
        let scratch = ScratchRepo::new("commit-files")?;
        scratch.write(
            "kept.txt",
            "one
two
three
four
five
",
        )?;
        scratch.write(
            "moved.txt",
            "a
b
c
d
e
f
",
        )?;
        scratch.write(
            "gone.txt", "bye
",
        )?;
        scratch.git(&["add", "."])?;
        scratch.commit("first")?;
        scratch.write(
            "kept.txt",
            "one
two
3
four
five
",
        )?;
        scratch.git(&["mv", "moved.txt", "renamed.txt"])?;
        scratch.git(&["rm", "--quiet", "gone.txt"])?;
        scratch.write(
            "new.txt", "hello
",
        )?;
        scratch.git(&["add", "."])?;
        scratch.commit("second")?;
        let (repo, history) = scratch.open()?;
        let files = parse_patch(&repo.commit_details(history.commits[0].id)?.patch)?;
        let mut shown: Vec<_> = files
            .iter()
            .map(|file| {
                (
                    file.path().to_string(),
                    change_kind(file),
                    renamed_from(file).map(ToString::to_string),
                )
            })
            .collect();
        shown.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            shown,
            [
                ("gone.txt".to_owned(), ChangeKind::Deleted, None),
                ("kept.txt".to_owned(), ChangeKind::Modified, None),
                ("new.txt".to_owned(), ChangeKind::Added, None),
                (
                    "renamed.txt".to_owned(),
                    ChangeKind::Renamed,
                    Some("moved.txt".to_owned())
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_conflicted_file_parses_into_its_conflicts() -> Result<()> {
        let scratch = ScratchRepo::new("conflict")?;
        scratch.write(
            "a.txt", "base
",
        )?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("base")?;
        scratch.git(&["checkout", "--quiet", "-b", "theirs"])?;
        scratch.write(
            "a.txt", "theirs
",
        )?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("theirs")?;
        scratch.git(&["checkout", "--quiet", DEFAULT_BRANCH])?;
        scratch.write(
            "a.txt", "ours
",
        )?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("ours")?;
        let stopped = scratch
            .git(&[
                "merge",
                "--quiet",
                "--no-ff",
                "--no-verify-signatures",
                "theirs",
            ])
            .expect_err("the merge stops at the conflict");
        assert!(
            scratch.dir.join(".git").join("MERGE_HEAD").exists(),
            "the merge failed for another reason: {stopped:#}"
        );

        let (repo, _) = scratch.open()?;
        let conflicted = repo
            .status()?
            .unstaged
            .into_iter()
            .find(|change| change.kind == ChangeKind::Conflicted)
            .expect("a.txt is conflicted");
        let files = file_patch_files(repo.file_patch(&conflicted, Side::Unstaged)?)?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].conflicts().len(), 1);
        Ok(())
    }
}
