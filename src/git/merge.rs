use anyhow::Result;
use gix::ObjectId;

use super::{
    Repo,
    command::{Exit, run},
    refusal::why_git_stopped,
    remote::{checked_out_branch, head_commit},
};

const MERGE_COMMIT_PARENTS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Merged {
    UpToDate,
    FastForwarded,
    WithMergeCommit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rebased {
    UpToDate,
    Replayed,
}

impl Repo {
    pub(crate) fn merge(&self, revision: &str) -> Result<Merged> {
        let repo = self.local();
        checked_out_branch(&repo)?;
        let before = head_commit(&repo);
        self.run_until_stopped(&repo, &["merge", "--quiet", "--no-edit", revision])?;
        let after = head_commit(&repo);
        Ok(if after == before {
            Merged::UpToDate
        } else if parent_count(&repo, after) >= MERGE_COMMIT_PARENTS {
            Merged::WithMergeCommit
        } else {
            Merged::FastForwarded
        })
    }

    pub(crate) fn rebase_onto(&self, revision: &str) -> Result<Rebased> {
        let repo = self.local();
        checked_out_branch(&repo)?;
        let before = head_commit(&repo);
        self.run_until_stopped(&repo, &["rebase", "--quiet", revision])?;
        Ok(if head_commit(&repo) == before {
            Rebased::UpToDate
        } else {
            Rebased::Replayed
        })
    }

    fn run_until_stopped(&self, repo: &gix::Repository, args: &[&str]) -> Result<()> {
        run(&self.location, args, Exit::Success)
            .map(drop)
            .map_err(|error| why_git_stopped(repo, error))
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
    use crate::git::{Refusal, scratch::ScratchRepo};

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
        assert_eq!(repo.merge("topic")?, Merged::FastForwarded);
        assert_eq!(scratch.head()?, topic);
        assert_eq!(repo.merge("topic")?, Merged::UpToDate);
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
        assert_eq!(repo.merge("topic")?, Merged::WithMergeCommit);
        assert_eq!(
            scratch.git(&["log", "-1", "--format=%s"])?.trim(),
            "Merge branch 'topic'"
        );
        Ok(())
    }

    #[test]
    fn a_merge_that_conflicts_stops_and_says_how_to_finish_it() -> Result<()> {
        let scratch = forked("merge-conflict")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "shared.txt", "topic\n")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        commit_file(&scratch, "shared.txt", "main\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(refusal(repo.merge("topic")), Refusal::MergeStopped);
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
        assert_eq!(repo.rebase_onto("main")?, Rebased::Replayed);
        assert_eq!(scratch.git(&["rev-parse", "HEAD~1"])?.trim(), main);
        assert_eq!(repo.rebase_onto("main")?, Rebased::UpToDate);
        Ok(())
    }

    #[test]
    fn a_rebase_waits_for_uncommitted_changes_and_stops_on_conflicts() -> Result<()> {
        let scratch = forked("rebase-stops")?;
        commit_file(&scratch, "shared.txt", "main\n")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "shared.txt", "topic\n")?;
        let (repo, _) = scratch.open()?;
        scratch.write("shared.txt", "draft\n")?;
        assert_eq!(refusal(repo.rebase_onto("main")), Refusal::ChangesInTheWay);
        scratch.git(&["checkout", "--quiet", "--", "shared.txt"])?;
        assert_eq!(refusal(repo.rebase_onto("main")), Refusal::RebaseStopped);
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
