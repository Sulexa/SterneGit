use anyhow::Result;
use gix::ObjectId;

use super::{Repo, command::run_with_input, patch::message_body, text::decode};

const COMMIT: [&str; 4] = ["commit", "--quiet", "--cleanup=whitespace", "--file=-"];
const AMEND: &str = "--amend";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommitMode {
    New,
    AmendLast,
}

pub(crate) fn commit_message(summary: &str, description: &str) -> Option<String> {
    let summary = summary.trim();
    if summary.is_empty() {
        return None;
    }
    let description = description.trim_end();
    Some(if description.trim_start().is_empty() {
        format!("{summary}\n")
    } else {
        format!("{summary}\n\n{description}\n")
    })
}

impl Repo {
    pub(crate) fn commit(&self, message: &str, mode: CommitMode) -> Result<()> {
        let mut args = COMMIT.to_vec();
        if mode == CommitMode::AmendLast {
            args.insert(1, AMEND);
        }
        run_with_input(&self.location, &args, message.as_bytes()).map(drop)
    }

    pub(crate) fn summary_and_description(&self, id: ObjectId) -> Result<(String, String)> {
        let local = self.local();
        let commit = local.find_commit(id)?;
        let summary = decode(&commit.message()?.summary());
        Ok((summary, message_body(&commit).to_string()))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::{CommitMode, commit_message};
    use crate::git::{Scope, scratch::ScratchRepo};

    fn committed(scratch: &ScratchRepo, file: &str, message: &str) -> Result<()> {
        scratch.write(file, file)?;
        scratch.git(&["add", file])?;
        scratch.git(&[
            "commit",
            "--quiet",
            "--author=Ada <ada@example.com>",
            "--date=2026-01-02T03:04:05",
            "-m",
            message,
        ])?;
        Ok(())
    }

    #[test]
    fn a_blank_summary_makes_no_message() {
        assert_eq!(commit_message("", "Body"), None);
        assert_eq!(commit_message(" \t ", ""), None);
    }

    #[test]
    fn the_description_follows_a_blank_line() {
        assert_eq!(
            commit_message("  Fix the parser ", "").as_deref(),
            Some("Fix the parser\n")
        );
        assert_eq!(
            commit_message("Fix the parser", "\n  \n").as_deref(),
            Some("Fix the parser\n")
        );
        assert_eq!(
            commit_message("Fix the parser", "It read past the end.\n  - and this\n\n").as_deref(),
            Some("Fix the parser\n\nIt read past the end.\n  - and this\n")
        );
    }

    #[test]
    fn only_staged_files_are_committed() -> Result<()> {
        let scratch = ScratchRepo::new("commit")?;
        scratch.write("staged.txt", "one\n")?;
        scratch.write("left.txt", "two\n")?;
        let (repo, _) = scratch.open()?;
        let staged = repo
            .status()?
            .unstaged
            .into_iter()
            .filter(|change| &*change.path == "staged.txt")
            .collect();
        repo.stage(&Scope::Files(staged))?;
        let message = commit_message("First commit", "#1 keeps its hash.").unwrap_or_default();
        repo.commit(&message, CommitMode::New)?;
        assert_eq!(
            scratch.git(&["log", "-1", "--format=%B"])?,
            "First commit\n\n#1 keeps its hash.\n\n"
        );
        let after = repo.status()?;
        assert!(after.staged.is_empty());
        assert_eq!(
            after
                .unstaged
                .iter()
                .map(|change| &*change.path)
                .collect::<Vec<_>>(),
            ["left.txt"]
        );
        Ok(())
    }

    #[test]
    fn a_commit_with_nothing_staged_fails() -> Result<()> {
        let scratch = ScratchRepo::new("commit-nothing")?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        assert!(repo.commit("Empty\n", CommitMode::New).is_err());
        Ok(())
    }

    #[test]
    fn amending_puts_the_staged_files_and_the_new_message_in_the_last_commit() -> Result<()> {
        let scratch = ScratchRepo::new("amend")?;
        committed(&scratch, "first.txt", "First")?;
        let author = scratch.git(&["log", "-1", "--format=%an <%ae> %at"])?;
        scratch.write("second.txt", "two\n")?;
        scratch.git(&["add", "second.txt"])?;
        let (repo, _) = scratch.open()?;
        repo.commit("First two files\n", CommitMode::AmendLast)?;
        assert_eq!(scratch.git(&["rev-list", "--count", "HEAD"])?.trim(), "1");
        assert_eq!(scratch.git(&["log", "-1", "--format=%B"])?, "First two files\n\n");
        assert_eq!(scratch.git(&["log", "-1", "--format=%an <%ae> %at"])?, author);
        assert_eq!(
            scratch.git(&["show", "--name-only", "--format=", "HEAD"])?,
            "first.txt\nsecond.txt\n"
        );
        assert!(repo.status()?.staged.is_empty());
        Ok(())
    }

    #[test]
    fn amending_with_nothing_staged_changes_only_the_message() -> Result<()> {
        let scratch = ScratchRepo::new("amend-message")?;
        committed(&scratch, "first.txt", "First")?;
        committed(&scratch, "second.txt", "Secnod")?;
        let tree = scratch.git(&["rev-parse", "HEAD^{tree}"])?;
        let parent = scratch.git(&["rev-parse", "HEAD~1"])?;
        let (repo, _) = scratch.open()?;
        repo.commit("Second\n\nWith its file.\n", CommitMode::AmendLast)?;
        assert_eq!(scratch.git(&["log", "-1", "--format=%B"])?, "Second\n\nWith its file.\n\n");
        assert_eq!(scratch.git(&["rev-parse", "HEAD^{tree}"])?, tree);
        assert_eq!(scratch.git(&["rev-parse", "HEAD~1"])?, parent);
        Ok(())
    }

    #[test]
    fn the_last_message_splits_into_summary_and_description() -> Result<()> {
        let scratch = ScratchRepo::new("amend-read")?;
        scratch.commit("Fix the parser\n\nIt read past the end.\n  - and this\n")?;
        let (repo, _) = scratch.open()?;
        let head = ObjectId::from_hex(scratch.head()?.as_bytes())?;
        let (summary, description) = repo.summary_and_description(head)?;
        assert_eq!(
            (summary.as_str(), description.as_str()),
            ("Fix the parser", "It read past the end.\n  - and this")
        );
        assert_eq!(
            commit_message(&summary, &description).as_deref(),
            Some("Fix the parser\n\nIt read past the end.\n  - and this\n")
        );
        Ok(())
    }
}
