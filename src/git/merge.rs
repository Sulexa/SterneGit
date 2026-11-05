use anyhow::Result;
use gix::ObjectId;

use super::{
    Carried, Repo,
    command::{Exit, run_with_access},
    refusal::why_git_stopped,
    remote::{checked_out_branch, head_commit},
};

const MERGE_COMMIT_PARENTS: usize = 2;
pub(super) const AUTOSTASH: &str = "--autostash";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Merged {
    UpToDate,
    FastForwarded,
    WithMergeCommit,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rebased {
    UpToDate,
    Replayed,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ran {
    Through,
    Stopped,
}

impl Repo {
    pub(crate) fn merge(&self, revision: &str) -> Result<(Merged, Carried)> {
        let repo = self.local();
        checked_out_branch(&repo)?;
        let before = head_commit(&repo);
        let stash = self.latest_stash();
        let args = ["merge", "--quiet", "--no-edit", AUTOSTASH, revision];
        if self.run_until_stopped(&args)? == Ran::Stopped {
            return Ok((Merged::Stopped, Carried::Along));
        }
        let after = head_commit(&repo);
        let merged = if after == before {
            Merged::UpToDate
        } else if parent_count(&repo, after) >= MERGE_COMMIT_PARENTS {
            Merged::WithMergeCommit
        } else {
            Merged::FastForwarded
        };
        Ok((merged, self.carried_since(stash)))
    }

    pub(crate) fn rebase_onto(&self, revision: &str) -> Result<(Rebased, Carried)> {
        let repo = self.local();
        checked_out_branch(&repo)?;
        let before = head_commit(&repo);
        let stash = self.latest_stash();
        let args = ["rebase", "--quiet", AUTOSTASH, revision];
        if self.run_until_stopped(&args)? == Ran::Stopped {
            return Ok((Rebased::Stopped, Carried::Along));
        }
        let rebased = if head_commit(&repo) == before {
            Rebased::UpToDate
        } else {
            Rebased::Replayed
        };
        Ok((rebased, self.carried_since(stash)))
    }

    pub(super) fn run_until_stopped(&self, args: &[&str]) -> Result<Ran> {
        match run_with_access(&self.location, &self.access, args, Exit::Success) {
            Ok(_) => Ok(Ran::Through),
            Err(_) if self.underway().is_some() => Ok(Ran::Stopped),
            Err(error) => Err(why_git_stopped(error)),
        }
    }
}

fn parent_count(repo: &gix::Repository, commit: Option<ObjectId>) -> usize {
    commit
        .and_then(|id| repo.find_commit(id).ok())
        .map_or(0, |commit| commit.parent_ids().count())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Merged, Rebased};
    use crate::git::{Carried, Refusal, scratch::ScratchRepo};

    fn forked(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.git(&["branch", "topic"])?;
        Ok(scratch)
    }

    fn commit_file(scratch: &ScratchRepo, file: &str, contents: &str) -> Result<()> {
        scratch.write(file, contents)?;
        scratch.git(&["add", file])?;
        scratch.commit(contents.trim())?;
        Ok(())
    }

    fn refusal(result: Result<impl std::fmt::Debug>) -> Refusal {
        let error = result.expect_err("git should have refused");
        error
            .downcast_ref::<Refusal>()
            .cloned()
            .unwrap_or_else(|| panic!("not a refusal: {error:#}"))
    }

    #[test]
    fn merging_a_branch_ahead_fast_forwards_and_twice_changes_nothing() -> Result<()> {
        let scratch = forked("merge-ahead")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "topic.txt", "topic\n")?;
        let topic = scratch.head()?;
        scratch.git(&["switch", "--quiet", "main"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic")?, (Merged::FastForwarded, Carried::Along));
        assert_eq!(scratch.head()?, topic);
        assert_eq!(repo.merge("topic")?.0, Merged::UpToDate);
        Ok(())
    }

    #[test]
    fn merging_a_branch_that_has_diverged_makes_a_merge_commit() -> Result<()> {
        let scratch = forked("merge-diverged")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "topic.txt", "topic\n")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        commit_file(&scratch, "main.txt", "main\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic")?.0, Merged::WithMergeCommit);
        assert_eq!(
            scratch.git(&["log", "-1", "--format=%s"])?.trim(),
            "Merge branch 'topic'"
        );
        Ok(())
    }

    #[test]
    fn a_merge_that_conflicts_stops_for_the_conflicts_to_be_resolved() -> Result<()> {
        let scratch = forked("merge-conflict")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "shared.txt", "topic\n")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        commit_file(&scratch, "shared.txt", "main\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic")?, (Merged::Stopped, Carried::Along));
        assert!(repo.underway().is_some());
        Ok(())
    }

    #[test]
    fn rebasing_replays_the_branch_on_top_of_the_other_one() -> Result<()> {
        let scratch = forked("rebase")?;
        commit_file(&scratch, "main.txt", "main\n")?;
        let main = scratch.head()?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "topic.txt", "topic\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.rebase_onto("main")?, (Rebased::Replayed, Carried::Along));
        assert_eq!(scratch.git(&["rev-parse", "HEAD~1"])?.trim(), main);
        assert_eq!(repo.rebase_onto("main")?.0, Rebased::UpToDate);
        Ok(())
    }

    #[test]
    fn a_rebase_puts_uncommitted_changes_aside_and_brings_them_back() -> Result<()> {
        let scratch = forked("rebase-autostash")?;
        commit_file(&scratch, "main.txt", "main\n")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "topic.txt", "topic\n")?;
        scratch.write("topic.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.rebase_onto("main")?, (Rebased::Replayed, Carried::Along));
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("topic.txt"))?,
            "draft\n"
        );
        assert_eq!(scratch.git(&["stash", "list"])?, "");
        Ok(())
    }

    #[test]
    fn changes_that_conflict_with_the_rebased_branch_stay_in_the_stash() -> Result<()> {
        let scratch = forked("rebase-autostash-conflict")?;
        commit_file(&scratch, "shared.txt", "main\n")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        scratch.write("shared.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.rebase_onto("main")?,
            (Rebased::Replayed, Carried::WithConflicts)
        );
        assert!(!repo.status()?.conflicted.is_empty());
        Ok(())
    }

    #[test]
    fn a_rebase_stops_on_conflicts() -> Result<()> {
        let scratch = forked("rebase-stops")?;
        commit_file(&scratch, "shared.txt", "main\n")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "shared.txt", "topic\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.rebase_onto("main")?, (Rebased::Stopped, Carried::Along));
        assert_eq!(repo.status()?.conflicted.len(), 1);
        Ok(())
    }

    #[test]
    fn merge_and_rebase_need_a_checked_out_branch() -> Result<()> {
        let scratch = forked("combine-detached")?;
        scratch.git(&["checkout", "--quiet", "--detach"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(refusal(repo.merge("topic")), Refusal::Detached);
        assert_eq!(refusal(repo.rebase_onto("topic")), Refusal::Detached);
        Ok(())
    }
}
