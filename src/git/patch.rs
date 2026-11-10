use std::ffi::OsStr;

use anyhow::{Context as _, Result};
use gix::ObjectId;
use gpui_kit::SharedString;

use super::{
    ChangeKind, FileChange, Repo, Side,
    command::{Exit, READ_ONLY, UNIQUE_LINES_FIRST, run, run_text},
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
    UNIQUE_LINES_FIRST,
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
    UNIQUE_LINES_FIRST,
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

const FIND_RENAMES: &str = "--find-renames";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Baseline {
    FirstParent(Option<ObjectId>),
    Commit(ObjectId),
}

impl Baseline {
    pub(crate) fn commit(self) -> Option<ObjectId> {
        match self {
            Baseline::FirstParent(parent) => parent,
            Baseline::Commit(base) => Some(base),
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

    pub(crate) fn message(&self, id: ObjectId) -> Result<String> {
        let local = self.local();
        let commit = local.find_commit(id)?;
        Ok(decode(commit.message_raw_sloppy()).trim_end().to_owned())
    }

    #[cfg(test)]
    pub(crate) fn file_patch(&self, change: &FileChange, side: Side) -> Result<FilePatch> {
        self.file_patch_with(change, side, Whitespace::Compared)
    }

    pub(crate) fn texts_patch(&self, before: &str, after: &str, name: &str) -> Result<String> {
        static TURNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let turn = TURNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let folder = std::env::temp_dir()
            .join("sternegit")
            .join(format!("texts-{}-{turn}", std::process::id()));
        for (side, text) in [("a", before), ("b", after)] {
            std::fs::create_dir_all(folder.join(side))?;
            std::fs::write(folder.join(side).join(name), text)?;
        }
        let output = run(
            &folder,
            &[
                READ_ONLY,
                "diff",
                "--no-index",
                "--no-color",
                UNIQUE_LINES_FIRST,
                "--",
                &format!("a/{name}"),
                &format!("b/{name}"),
            ],
            Exit::SuccessOrDifference,
        );
        let _ = std::fs::remove_dir_all(&folder);
        let _ = self;
        Ok(String::from_utf8_lossy(&output?).into_owned())
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
        baseline: Baseline,
        paths: &[&OsStr],
        whitespace: Whitespace,
    ) -> Result<String> {
        let id = id.to_string();
        let base = baseline.commit().map(|base| base.to_string());
        let mut args: Vec<&OsStr> = match (baseline, &base) {
            (Baseline::Commit(_), Some(base)) => FILE_PATCH
                .iter()
                .copied()
                .chain([FIND_RENAMES, base.as_str()])
                .map(OsStr::new)
                .collect(),
            _ => COMMIT_PATCH.iter().map(OsStr::new).collect(),
        };
        args.extend(whitespace.option().map(OsStr::new));
        args.extend([OsStr::new(&id), OsStr::new("--")]);
        args.extend(paths);
        run_text(&self.location, &args, Exit::Success)
    }

    pub(crate) fn range_patch(&self, base: ObjectId, head: ObjectId) -> Result<String> {
        let (base, head) = (base.to_string(), head.to_string());
        let mut args = FILE_PATCH.to_vec();
        args.extend([FIND_RENAMES, base.as_str(), head.as_str(), "--"]);
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

    use super::FilePatch;
    use crate::git::{
        Side,
        scratch::{
            INSERTED_LINES, ScratchRepo, added_and_removed_lines, lookalike_blocks,
            lookalike_blocks_with_insertions,
        },
    };

    const LONG_AGO: Duration = Duration::from_secs(1_000_000_000);

    #[test]
    fn blocks_inserted_between_lookalike_blocks_show_as_added_lines_only() -> Result<()> {
        let scratch = ScratchRepo::new("lookalike-blocks")?;
        scratch.write("shop.prefab", &lookalike_blocks())?;
        scratch.git(&["add", "shop.prefab"])?;
        scratch.commit("Add the shop")?;
        scratch.write("shop.prefab", &lookalike_blocks_with_insertions())?;
        let (repo, _) = scratch.open()?;
        let edited = repo.status()?.unstaged[0].clone();
        let FilePatch::Diff { text: unstaged, .. } = repo.file_patch(&edited, Side::Unstaged)? else {
            panic!("the edit isn't a conflict");
        };
        scratch.git(&["commit", "--quiet", "-am", "Insert offers"])?;
        let (repo, history) = scratch.open()?;
        let committed = repo.commit_details(history.commits[0].id)?.patch;
        let texts = repo.texts_patch(
            &lookalike_blocks(),
            &lookalike_blocks_with_insertions(),
            "shop.prefab",
        )?;
        for patch in [unstaged, committed, texts] {
            assert_eq!(added_and_removed_lines(&patch), (INSERTED_LINES, 0));
        }
        Ok(())
    }

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
