use anyhow::Result;
use gix::ObjectId;

use super::{
    ChangeKind, Refusal, Repo,
    command::{Exit, run},
};

const STASH: &str = "refs/stash";
const STASH_LEFT_UNAPPLIED: [&str; 3] = [
    "conflicts in index",
    "could not restore untracked files",
    "would be overwritten by",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Carried {
    Along,
    WithConflicts,
    OnlyInStash,
}

impl Repo {
    pub(crate) fn has_stash(&self) -> bool {
        self.latest_stash().is_some()
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
        match self.reapply_stash()? {
            Carried::Along => Ok(()),
            Carried::WithConflicts => Err(Refusal::StashKept.into()),
            Carried::OnlyInStash => Err(Refusal::StashNotApplied.into()),
        }
    }

    pub(super) fn stash_aside(&self) -> Result<bool> {
        let before = self.latest_stash();
        self.stash()?;
        Ok(self.latest_stash() != before)
    }

    pub(super) fn reapply_stash(&self) -> Result<Carried> {
        match run(
            &self.location,
            &["stash", "pop", "--index", "--quiet"],
            Exit::Success,
        ) {
            Ok(_) => Ok(Carried::Along),
            Err(_) if self.has_conflicts() => Ok(Carried::WithConflicts),
            Err(error) if left_unapplied(&error) => Ok(Carried::OnlyInStash),
            Err(error) => Err(error),
        }
    }

    fn latest_stash(&self) -> Option<ObjectId> {
        let local = self.local();
        let stash = local.try_find_reference(STASH).ok()??;
        stash.try_id().map(|id| id.detach())
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

fn left_unapplied(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_lowercase();
    STASH_LEFT_UNAPPLIED.iter().any(|why| message.contains(why))
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
    fn a_popped_stash_keeps_what_was_staged_staged() -> Result<()> {
        let scratch = ScratchRepo::new("stash-staged")?;
        scratch.write("staged.txt", "one\n")?;
        scratch.write("unstaged.txt", "one\n")?;
        scratch.git(&["add", "staged.txt", "unstaged.txt"])?;
        scratch.commit("first")?;
        scratch.write("staged.txt", "two\n")?;
        scratch.git(&["add", "staged.txt"])?;
        scratch.write("unstaged.txt", "two\n")?;
        let (repo, _) = scratch.open()?;
        repo.stash()?;
        repo.pop_stash()?;
        assert_eq!(
            scratch.git(&["diff", "--cached", "--name-only"])?,
            "staged.txt\n"
        );
        assert_eq!(scratch.git(&["diff", "--name-only"])?, "unstaged.txt\n");
        Ok(())
    }

    #[test]
    fn a_stash_that_cannot_apply_is_kept_and_changes_nothing() -> Result<()> {
        let scratch = ScratchRepo::new("stash-not-applied")?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.write("shared.txt", "stashed\n")?;
        scratch.git(&["add", "shared.txt"])?;
        let (repo, _) = scratch.open()?;
        repo.stash()?;
        scratch.write("shared.txt", "committed\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("second")?;
        let error = repo
            .pop_stash()
            .expect_err("the staged change contradicts the commit");
        assert_eq!(
            error.downcast_ref::<Refusal>(),
            Some(&Refusal::StashNotApplied)
        );
        assert!(changed_paths(&scratch)?.is_empty());
        assert!(repo.has_stash());
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
