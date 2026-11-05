use std::ffi::OsStr;

use anyhow::{Context as _, Result};
use gix::ObjectId;
use gpui_kit::SharedString;

use super::{
    ChangeKind, FileChange, Repo, Side,
    command::{Exit, READ_ONLY, run, run_text},
    context::with_whole_file,
    hunks::Hunks,
    text::decode,
    version::Revision,
};

const NAMES_AS_WRITTEN: &str = "core.quotePath=false";

const COMMIT_PATCH: &[&str] = &[
    READ_ONLY,
    "-c",
    NAMES_AS_WRITTEN,
    "show",
    "--no-show-signature",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--format=",
    "--first-parent",
    "--patch",
    "--find-renames",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

const FILE_PATCH: &[&str] = &[
    READ_ONLY,
    "-c",
    NAMES_AS_WRITTEN,
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

const NOTHING: &str = "/dev/null";

const IGNORE_WHITESPACE: &str = "--ignore-all-space";

const MOST_BYTES_SPREAD: u64 = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Whitespace {
    #[default]
    Compared,
    Ignored,
}

impl Whitespace {
    fn option(self) -> Option<&'static str> {
        match self {
            Whitespace::Compared => None,
            Whitespace::Ignored => Some(IGNORE_WHITESPACE),
        }
    }
}

pub(crate) struct CommitDetails {
    pub(crate) body: SharedString,
    pub(crate) patch: String,
}

pub(crate) enum FilePatch {
    Diff {
        text: String,
        hunks: Option<Hunks>,
    },
    Conflicted {
        path: SharedString,
        contents: String,
    },
}

impl Repo {
    pub(crate) fn commit_details(&self, id: ObjectId) -> Result<CommitDetails> {
        let body = message_body(&self.local().find_commit(id)?);
        let id = id.to_string();
        let mut args = COMMIT_PATCH.to_vec();
        args.extend([id.as_str(), "--"]);
        let patch = run_text(&self.location, &args, Exit::Success)?;
        Ok(CommitDetails { body, patch })
    }

    #[cfg(test)]
    pub(crate) fn file_patch(&self, change: &FileChange, side: Side) -> Result<FilePatch> {
        self.file_patch_with(change, side, Whitespace::Compared)
    }

    pub(crate) fn file_patch_with(
        &self,
        change: &FileChange,
        side: Side,
        whitespace: Whitespace,
    ) -> Result<FilePatch> {
        let paths: Vec<&OsStr> = change.exact_paths().collect();
        let split = change.kind == ChangeKind::Modified && whitespace == Whitespace::Compared;
        let blanks = whitespace.option();
        match (change.kind, side) {
            (ChangeKind::Conflicted, _) => self.conflicted_file(change),
            (ChangeKind::Untracked, _) => self.diff(
                &["--no-index"],
                &[OsStr::new(NOTHING), change.exact_path()],
                Exit::SuccessOrDifference,
                false,
            ),
            (_, Side::Staged) => self.diff(
                &[Some("--cached"), Some("--find-renames"), blanks]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>(),
                &paths,
                Exit::Success,
                split,
            ),
            (_, Side::Unstaged) => self.diff(
                &blanks.into_iter().collect::<Vec<_>>(),
                &paths,
                Exit::Success,
                split,
            ),
        }
    }

    pub(crate) fn commit_file_patch(
        &self,
        id: ObjectId,
        paths: &[&OsStr],
        whitespace: Whitespace,
    ) -> Result<String> {
        let id = id.to_string();
        let mut args: Vec<&OsStr> = COMMIT_PATCH.iter().map(OsStr::new).collect();
        args.extend(whitespace.option().map(OsStr::new));
        args.extend([OsStr::new(&id), OsStr::new("--")]);
        args.extend(paths);
        run_text(&self.location, &args, Exit::Success)
    }

    pub(crate) fn with_whole_file(&self, patch: String, revision: Revision, path: &OsStr) -> String {
        let small_enough = match revision {
            Revision::WorkTree => std::fs::metadata(self.location.join(path))
                .is_ok_and(|metadata| metadata.len() <= MOST_BYTES_SPREAD),
            _ => true,
        };
        let new_text = small_enough
            .then(|| self.file_bytes(revision, path).ok())
            .flatten()
            .filter(|bytes| bytes.len() as u64 <= MOST_BYTES_SPREAD);
        match new_text {
            Some(bytes) => with_whole_file(&patch, &decode(&bytes)).unwrap_or(patch),
            None => patch,
        }
    }

    fn diff(
        &self,
        options: &[&str],
        paths: &[&OsStr],
        exit: Exit,
        split: bool,
    ) -> Result<FilePatch> {
        let mut args: Vec<&OsStr> = FILE_PATCH.iter().chain(options).map(OsStr::new).collect();
        args.push(OsStr::new("--"));
        args.extend(paths);
        let patch = run(&self.location, &args, exit)?;
        Ok(FilePatch::Diff {
            text: decode(&patch),
            hunks: split.then(|| Hunks::parse(&patch)).flatten(),
        })
    }

    fn conflicted_file(&self, change: &FileChange) -> Result<FilePatch> {
        let path = change.path.as_str();
        let bytes = std::fs::read(self.location.join(change.exact_path()))
            .with_context(|| format!("Couldn't read {path}"))?;
        Ok(FilePatch::Conflicted {
            path: change.path.clone(),
            contents: decode(&bytes),
        })
    }
}

fn message_body(commit: &gix::Commit<'_>) -> SharedString {
    let message = decode(commit.message_raw_sloppy());
    message
        .split_once("\n\n")
        .map_or("", |(_, body)| body.trim())
        .to_owned()
        .into()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use anyhow::Result;

    use crate::git::{Side, scratch::ScratchRepo};

    const LONG_AGO: Duration = Duration::from_secs(1_000_000_000);

    #[test]
    fn reading_the_repository_leaves_its_index_alone() -> Result<()> {
        let scratch = ScratchRepo::new("read-only")?;
        scratch.write("same.txt", "same\n")?;
        scratch.write("edited.txt", "before\n")?;
        scratch.git(&["add", "same.txt", "edited.txt"])?;
        scratch.commit("first")?;
        scratch.write("edited.txt", "after\n")?;
        std::fs::File::options()
            .write(true)
            .open(scratch.dir.join("same.txt"))?
            .set_modified(SystemTime::UNIX_EPOCH + LONG_AGO)?;
        let index = scratch.dir.join(".git").join("index");
        let untouched = std::fs::read(&index)?;

        let (repo, history) = scratch.open()?;
        let edited = repo.status()?.unstaged[0].clone();
        repo.file_patch(&edited, Side::Unstaged)?;
        repo.commit_details(history.commits[0].id)?;
        assert_eq!(
            std::fs::read(&index)?,
            untouched,
            "a read rewrote the index"
        );
        Ok(())
    }

    #[test]
    fn a_latin_1_file_shows_its_accented_letters() -> Result<()> {
        let scratch = ScratchRepo::new("latin-1")?;
        std::fs::write(scratch.dir.join("notes.txt"), b"caf\xE9 cr\xE8me, \xE2ge\n")?;
        scratch.git(&["add", "notes.txt"])?;
        scratch.commit("Add notes")?;
        let (repo, history) = scratch.open()?;
        let details = repo.commit_details(history.commits[0].id)?;
        assert!(
            details.patch.contains("+café crème, âge"),
            "{}",
            details.patch
        );
        Ok(())
    }
}
