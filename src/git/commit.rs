use anyhow::Result;

use super::{Repo, command::run_with_input};

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
    pub(crate) fn commit(&self, message: &str) -> Result<()> {
        run_with_input(
            &self.location,
            &["commit", "--quiet", "--cleanup=whitespace", "--file=-"],
            message.as_bytes(),
        )
        .map(drop)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::commit_message;
    use crate::git::{Scope, scratch::ScratchRepo};

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
        repo.commit(&message)?;
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
        assert!(repo.commit("Empty\n").is_err());
        Ok(())
    }
}
