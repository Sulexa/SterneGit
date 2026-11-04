use anyhow::{Context as _, Result};
use gix::{ObjectId, bstr::ByteSlice as _};
use gpui_kit::SharedString;

use super::{
    ChangeKind, FileChange, Repo, Side,
    command::{Exit, run, run_utf8},
};

const COMMIT_PATCH: &[&str] = &[
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
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

const NOTHING: &str = "/dev/null";

pub(crate) struct CommitDetails {
    pub(crate) body: SharedString,
    pub(crate) patch: String,
}

pub(crate) enum FilePatch {
    Diff(String),
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
        let patch = run_utf8(&self.location, &args)?;
        Ok(CommitDetails { body, patch })
    }

    pub(crate) fn file_patch(&self, change: &FileChange, side: Side) -> Result<FilePatch> {
        let paths: Vec<&str> = change.paths().collect();
        match (change.kind, side) {
            (ChangeKind::Conflicted, _) => self.conflicted_file(change),
            (ChangeKind::Untracked, _) => self.diff(
                &["--no-index"],
                &[NOTHING, change.path.as_str()],
                Exit::SuccessOrDifference,
            ),
            (_, Side::Staged) => self.diff(&["--cached", "--find-renames"], &paths, Exit::Success),
            (_, Side::Unstaged) => self.diff(&[], &paths, Exit::Success),
        }
    }

    fn diff(&self, options: &[&str], paths: &[&str], exit: Exit) -> Result<FilePatch> {
        let mut args = FILE_PATCH.to_vec();
        args.extend(options);
        args.push("--");
        args.extend(paths);
        let patch = run(&self.location, &args, exit)?;
        Ok(FilePatch::Diff(
            String::from_utf8_lossy(&patch).into_owned(),
        ))
    }

    fn conflicted_file(&self, change: &FileChange) -> Result<FilePatch> {
        let path = change.path.as_str();
        let bytes = std::fs::read(self.location.join(path))
            .with_context(|| format!("Couldn't read {path}"))?;
        Ok(FilePatch::Conflicted {
            path: change.path.clone(),
            contents: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }
}

fn message_body(commit: &gix::Commit<'_>) -> SharedString {
    let message = commit.message_raw_sloppy().to_str_lossy();
    message
        .split_once("\n\n")
        .map_or("", |(_, body)| body.trim())
        .to_owned()
        .into()
}
