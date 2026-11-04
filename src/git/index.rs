use anyhow::Result;

use super::{FileChange, Repo, command::run_utf8};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    Everything,
    Files(Vec<FileChange>),
}

impl Repo {
    pub(crate) fn stage(&self, scope: &Scope) -> Result<()> {
        let mut args = vec!["--literal-pathspecs", "add", "--all", "--"];
        if let Scope::Files(files) = scope {
            args.extend(files.iter().map(|file| file.path.as_str()));
        }
        run_utf8(&self.location, &args).map(drop)
    }

    pub(crate) fn unstage(&self, scope: &Scope) -> Result<()> {
        let has_head_commit = self.local().head_id().is_ok();
        let mut args = if has_head_commit {
            vec!["--literal-pathspecs", "reset", "--quiet", "--"]
        } else {
            vec![
                "--literal-pathspecs",
                "rm",
                "--cached",
                "-r",
                "--quiet",
                "--",
            ]
        };
        match scope {
            Scope::Everything if !has_head_commit => args.push("."),
            Scope::Everything => {}
            Scope::Files(files) => args.extend(files.iter().flat_map(FileChange::paths)),
        }
        run_utf8(&self.location, &args).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::Scope;
    use crate::git::{ChangeKind, FilePatch, Side, scratch::ScratchRepo};

    fn added_and_removed(patch: &FilePatch) -> (usize, usize) {
        let FilePatch::Diff(text) = patch else {
            panic!("expected a diff");
        };
        let lines = text
            .lines()
            .filter(|line| !line.starts_with("+++") && !line.starts_with("---"));
        lines.fold((0, 0), |(added, removed), line| {
            match line.as_bytes().first() {
                Some(b'+') => (added + 1, removed),
                Some(b'-') => (added, removed + 1),
                _ => (added, removed),
            }
        })
    }

    #[test]
    fn files_stage_unstage_and_diff() -> Result<()> {
        let scratch = ScratchRepo::new("stage")?;
        scratch.write("a.txt", "one\n")?;
        let (repo, _) = scratch.open()?;
        let untracked = repo.status()?.unstaged[0].clone();
        assert_eq!(untracked.kind, ChangeKind::Untracked);
        assert_eq!(
            added_and_removed(&repo.file_patch(&untracked, Side::Unstaged)?),
            (1, 0)
        );
        repo.stage(&Scope::Files(vec![untracked.clone()]))?;
        assert_eq!(repo.status()?.staged[0].kind, ChangeKind::Added);
        repo.unstage(&Scope::Everything)?;
        assert!(repo.status()?.staged.is_empty(), "before the first commit");

        repo.stage(&Scope::Everything)?;
        scratch.commit("first")?;
        scratch.write("a.txt", "one\ntwo\n")?;
        let before = repo.status()?;
        let modified = before.unstaged[0].clone();
        assert_eq!(modified.kind, ChangeKind::Modified);
        repo.stage(&Scope::Files(vec![modified.clone()]))?;
        let staged = repo.status()?;
        assert_ne!(staged.digest, before.digest);
        assert_eq!(
            added_and_removed(&repo.file_patch(&staged.staged[0], Side::Staged)?),
            (1, 0)
        );
        repo.unstage(&Scope::Files(vec![modified]))?;
        assert!(repo.status()?.staged.is_empty(), "after the first commit");
        Ok(())
    }
}
