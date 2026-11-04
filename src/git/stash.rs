use anyhow::Result;

use super::{
    ChangeKind, Refusal, Repo,
    command::{Exit, run},
};

const STASH: &str = "refs/stash";

impl Repo {
    pub(crate) fn has_stash(&self) -> bool {
        self.local()
            .try_find_reference(STASH)
            .is_ok_and(|found| found.is_some())
    }

    pub(crate) fn stash(&self) -> Result<()> {
        run(
            &self.location,
            &["stash", "push", "--include-untracked", "--quiet"],
            Exit::Success,
        )
        .map(drop)
    }

    pub(crate) fn pop_stash(&self) -> Result<()> {
        run(&self.location, &["stash", "pop", "--quiet"], Exit::Success)
            .map(drop)
            .map_err(|error| {
                if self.has_conflicts() {
                    Refusal::StashKept.into()
                } else {
                    error
                }
            })
    }

    fn has_conflicts(&self) -> bool {
        self.status().is_ok_and(|changes| {
            changes
                .unstaged
                .iter()
                .chain(&changes.staged)
                .any(|change| change.kind == ChangeKind::Conflicted)
        })
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::git::{Refusal, scratch::ScratchRepo};

    fn changed_paths(scratch: &ScratchRepo) -> Result<Vec<String>> {
        let status = scratch.open()?.0.status()?;
        let mut paths: Vec<String> = status
            .unstaged
            .iter()
            .chain(&status.staged)
            .map(|change| change.path.to_string())
            .collect();
        paths.sort();
        Ok(paths)
    }

    #[test]
    fn a_stash_puts_every_change_aside_until_it_is_popped() -> Result<()> {
        let scratch = ScratchRepo::new("stash")?;
        scratch.write("tracked.txt", "one\n")?;
        scratch.git(&["add", "tracked.txt"])?;
        scratch.commit("first")?;
        scratch.write("tracked.txt", "two\n")?;
        scratch.write("untracked.txt", "new\n")?;
        let (repo, _) = scratch.open()?;
        assert!(!repo.has_stash());
        repo.stash()?;
        assert!(changed_paths(&scratch)?.is_empty());
        assert!(repo.has_stash());
        repo.pop_stash()?;
        assert_eq!(changed_paths(&scratch)?, ["tracked.txt", "untracked.txt"]);
        assert!(!repo.has_stash());
        Ok(())
    }

    #[test]
    fn a_pop_that_conflicts_keeps_the_stash() -> Result<()> {
        let scratch = ScratchRepo::new("stash-conflict")?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.write("shared.txt", "stashed\n")?;
        let (repo, _) = scratch.open()?;
        repo.stash()?;
        scratch.write("shared.txt", "committed\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("second")?;
        let error = repo.pop_stash().expect_err("the stash conflicts");
        assert_eq!(error.downcast_ref::<Refusal>(), Some(&Refusal::StashKept));
        assert!(repo.has_stash());
        Ok(())
    }
}
