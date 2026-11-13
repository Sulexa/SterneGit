use anyhow::{Result, anyhow};
use gix::ObjectId;

use super::{
    Carried, Picked, Refusal, Repo, Tracking,
    command::{Exit, READ_ONLY, run, run_text, run_with_access},
    refusal::why_git_stopped,
    remote::{checked_out_branch, head_commit, short_name},
    underway::{forget_replay, remember_replay},
};

const MERGE_COMMIT_PARENTS: usize = 2;
pub(super) const AUTOSTASH: &str = "--autostash";
pub(super) const SQUASH_MESSAGE: &str = "SQUASH_MSG";
pub(super) const COMMIT_SQUASH: [&str; 4] = ["commit", "--quiet", "--no-edit", "--cleanup=strip"];
pub(super) const DROP_SQUASH: [&str; 3] = ["reset", "--merge", "--quiet"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MergeMode {
    FastForwardIfPossible,
    MergeCommit,
    FastForwardOnly,
    Squash,
    RebaseThenFastForward,
}

impl MergeMode {
    pub(crate) const KEEPING_COMMITS: [MergeMode; 3] = [
        MergeMode::FastForwardIfPossible,
        MergeMode::MergeCommit,
        MergeMode::FastForwardOnly,
    ];
    pub(crate) const MAKING_COMMITS: [MergeMode; 2] = [MergeMode::Squash, MergeMode::RebaseThenFastForward];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Merged {
    UpToDate,
    NothingNew,
    FastForwarded,
    WithMergeCommit,
    Squashed,
    Replayed(usize),
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

struct Apart {
    ours: usize,
    theirs: usize,
}

impl Repo {
    pub(crate) fn merge(&self, revision: &str, mode: MergeMode) -> Result<(Merged, Carried)> {
        let branch = checked_out_branch(&self.local())?;
        let apart = self.apart_from(revision)?;
        if apart.theirs == 0 {
            return Ok((Merged::UpToDate, Carried::Along));
        }
        let diverged = apart.ours > 0;
        match mode {
            MergeMode::FastForwardIfPossible => self.merge_with("--ff", revision),
            MergeMode::MergeCommit => self.merge_with("--no-ff", revision),
            MergeMode::Squash => self.squash_in(revision),
            MergeMode::RebaseThenFastForward if diverged => self.replay(revision),
            MergeMode::FastForwardOnly if diverged => Err(Refusal::MergeCantFastForward(Tracking {
                branch: short_name(&branch),
                upstream: revision.to_owned(),
            })
            .into()),
            MergeMode::FastForwardOnly | MergeMode::RebaseThenFastForward => {
                self.merge_with("--ff-only", revision)
            }
        }
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

    fn apart_from(&self, revision: &str) -> Result<Apart> {
        let range = format!("HEAD...{revision}");
        let counts = run_text(
            &self.location,
            &[READ_ONLY, "rev-list", "--left-right", "--count", &range, "--"],
            Exit::Success,
        )?;
        let mut numbers = counts.split_whitespace().map(str::parse::<usize>);
        match (numbers.next(), numbers.next()) {
            (Some(Ok(ours)), Some(Ok(theirs))) => Ok(Apart { ours, theirs }),
            _ => Err(anyhow!("git rev-list didn't count the commits apart: {counts}")),
        }
    }

    fn merge_with(&self, fast_forward: &str, revision: &str) -> Result<(Merged, Carried)> {
        let repo = self.local();
        let before = head_commit(&repo);
        let stash = self.latest_stash();
        let args = ["merge", "--quiet", "--no-edit", AUTOSTASH, fast_forward, revision];
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

    fn squash_in(&self, revision: &str) -> Result<(Merged, Carried)> {
        let message = squash_message(revision, &self.subjects_only_on(revision)?);
        let set_aside = self.set_aside_until_done()?;
        let bring_back = |this: &Self| if set_aside { this.bring_back_set_aside() } else { Ok(Carried::Along) };
        match self.try_squash(revision, &message) {
            Ok(Merged::Stopped) => Ok((Merged::Stopped, Carried::Along)),
            Ok(merged) => Ok((merged, bring_back(self)?)),
            Err(error) => {
                bring_back(self)?;
                Err(error)
            }
        }
    }

    fn try_squash(&self, revision: &str, message: &str) -> Result<Merged> {
        let ran = self.run_until_stopped(&["merge", "--squash", "--quiet", revision])?;
        std::fs::write(self.local().git_dir().join(SQUASH_MESSAGE), message)?;
        if ran == Ran::Stopped {
            return Ok(Merged::Stopped);
        }
        if self.status()?.staged.is_empty() {
            run(&self.location, &DROP_SQUASH, Exit::Success)?;
            return Ok(Merged::NothingNew);
        }
        run(&self.location, &COMMIT_SQUASH, Exit::Success)?;
        Ok(Merged::Squashed)
    }

    fn subjects_only_on(&self, revision: &str) -> Result<Vec<String>> {
        let range = format!("HEAD..{revision}");
        let subjects = run_text(
            &self.location,
            &[READ_ONLY, "log", "--reverse", "--no-merges", "--format=%s", &range, "--"],
            Exit::Success,
        )?;
        Ok(subjects.lines().map(str::to_owned).collect())
    }

    fn replay(&self, revision: &str) -> Result<(Merged, Carried)> {
        let oldest_first = self.commits_to_replay(revision)?;
        if oldest_first.is_empty() {
            return Ok((Merged::NothingNew, Carried::Along));
        }
        remember_replay(&self.local(), revision, &oldest_first)?;
        let replayed = self.cherry_pick_several(&oldest_first);
        if !matches!(replayed, Ok((Picked::Stopped, _))) {
            forget_replay(&self.local());
        }
        let (picked, carried) = replayed?;
        let merged = match picked {
            Picked::Stopped => Merged::Stopped,
            Picked::Committed => Merged::Replayed(oldest_first.len()),
            Picked::AlreadyThere => Merged::NothingNew,
        };
        Ok((merged, carried))
    }

    fn commits_to_replay(&self, revision: &str) -> Result<Vec<ObjectId>> {
        let range = format!("HEAD...{revision}");
        let ids = run_text(
            &self.location,
            &[READ_ONLY, "rev-list", "--reverse", "--no-merges", "--right-only", "--cherry-pick", &range, "--"],
            Exit::Success,
        )?;
        ids.lines()
            .map(|id| Ok(ObjectId::from_hex(id.trim().as_bytes())?))
            .collect()
    }
}

pub(crate) fn squash_message(from: &str, subjects: &[String]) -> String {
    let listed: String = subjects.iter().map(|subject| format!("\n* {subject}")).collect();
    match listed.is_empty() {
        true => format!("Squashed branch '{from}'\n"),
        false => format!("Squashed branch '{from}'\n{listed}\n"),
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

    use super::{MergeMode, Merged, Rebased, squash_message};
    use crate::git::{
        Carried, Continued, Merging, Refusal, Tracking, Underway, scratch::ScratchRepo,
    };

    const EVERY_MODE: [MergeMode; 5] = [
        MergeMode::FastForwardIfPossible,
        MergeMode::MergeCommit,
        MergeMode::FastForwardOnly,
        MergeMode::Squash,
        MergeMode::RebaseThenFastForward,
    ];

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

    fn topic_ahead(label: &str) -> Result<ScratchRepo> {
        let scratch = forked(label)?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "a.txt", "a\n")?;
        commit_file(&scratch, "b.txt", "b\n")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        Ok(scratch)
    }

    fn diverged(label: &str) -> Result<ScratchRepo> {
        let scratch = topic_ahead(label)?;
        commit_file(&scratch, "main.txt", "main\n")?;
        Ok(scratch)
    }

    fn conflicting(label: &str) -> Result<ScratchRepo> {
        let scratch = forked(label)?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "shared.txt", "topic\n")?;
        commit_file(&scratch, "b.txt", "b\n")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        commit_file(&scratch, "shared.txt", "main\n")?;
        Ok(scratch)
    }

    fn subjects(scratch: &ScratchRepo, count: &str) -> Result<String> {
        scratch.git(&["log", "--format=%s", count])
    }

    fn read(scratch: &ScratchRepo, file: &str) -> Result<String> {
        Ok(std::fs::read_to_string(scratch.dir.join(file))?)
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
        let scratch = topic_ahead("merge-ahead")?;
        let topic = scratch.git(&["rev-parse", "topic"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.merge("topic", MergeMode::FastForwardIfPossible)?,
            (Merged::FastForwarded, Carried::Along)
        );
        assert_eq!(scratch.head()?, topic.trim());
        for mode in EVERY_MODE {
            assert_eq!(repo.merge("topic", mode)?.0, Merged::UpToDate, "{mode:?}");
        }
        Ok(())
    }

    #[test]
    fn merging_a_branch_that_has_diverged_makes_a_merge_commit() -> Result<()> {
        let scratch = diverged("merge-diverged")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::FastForwardIfPossible)?.0, Merged::WithMergeCommit);
        assert_eq!(subjects(&scratch, "-1")?, "Merge branch 'topic'\n");
        Ok(())
    }

    #[test]
    fn a_merge_commit_is_made_even_when_main_could_fast_forward() -> Result<()> {
        let scratch = topic_ahead("merge-no-ff")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::MergeCommit)?.0, Merged::WithMergeCommit);
        assert_eq!(subjects(&scratch, "-1")?, "Merge branch 'topic'\n");
        assert_eq!(scratch.git(&["rev-parse", "HEAD^2"])?, scratch.git(&["rev-parse", "topic"])?);
        Ok(())
    }

    #[test]
    fn fast_forward_only_moves_main_or_refuses_and_changes_nothing() -> Result<()> {
        let scratch = topic_ahead("merge-ff-only")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::FastForwardOnly)?.0, Merged::FastForwarded);
        commit_file(&scratch, "main.txt", "main\n")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        commit_file(&scratch, "c.txt", "c\n")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        let before = scratch.head()?;
        assert_eq!(
            refusal(repo.merge("topic", MergeMode::FastForwardOnly)),
            Refusal::MergeCantFastForward(Tracking {
                branch: "main".into(),
                upstream: "topic".into(),
            })
        );
        assert_eq!(scratch.head()?, before);
        Ok(())
    }

    #[test]
    fn a_squash_makes_one_commit_listing_the_branch_subjects_around_staged_changes() -> Result<()> {
        let scratch = diverged("merge-squash")?;
        scratch.write("shared.txt", "staged\n")?;
        scratch.git(&["add", "shared.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::Squash)?, (Merged::Squashed, Carried::Along));
        assert_eq!(
            scratch.git(&["log", "-1", "--format=%B"])?,
            "Squashed branch 'topic'\n\n* a\n* b\n\n"
        );
        assert_eq!(scratch.git(&["log", "-1", "--format=%P"])?.split_whitespace().count(), 1);
        assert_eq!(scratch.git(&["diff", "--name-only", "HEAD~1", "HEAD"])?, "a.txt\nb.txt\n");
        assert_eq!(scratch.git(&["diff", "--cached", "--name-only"])?, "shared.txt\n");
        assert_eq!(scratch.git(&["stash", "list"])?, "");
        assert!(!scratch.dir.join(".git/SQUASH_MSG").exists());
        Ok(())
    }

    #[test]
    fn a_squash_that_conflicts_stops_and_commits_once_resolved() -> Result<()> {
        let scratch = conflicting("merge-squash-conflict")?;
        scratch.write("draft.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::Squash)?, (Merged::Stopped, Carried::Along));
        assert_eq!(
            repo.underway(),
            Some(Underway::Squash(Merging {
                from: "topic".into(),
                into: "main".into(),
            }))
        );
        assert!(!scratch.dir.join("draft.txt").exists());
        assert_eq!(repo.continue_underway()?, Continued::StoppedAgain);
        scratch.write("shared.txt", "both\n")?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(repo.continue_underway()?, Continued::Finished(Carried::Along));
        assert_eq!(repo.underway(), None);
        assert_eq!(
            scratch.git(&["log", "-1", "--format=%B"])?,
            "Squashed branch 'topic'\n\n* topic\n* b\n\n"
        );
        assert_eq!(read(&scratch, "draft.txt")?, "draft\n");
        assert_eq!(scratch.git(&["stash", "list"])?, "");
        Ok(())
    }

    #[test]
    fn aborting_a_squash_puts_main_and_the_set_aside_changes_back() -> Result<()> {
        let scratch = conflicting("merge-squash-abort")?;
        let before = scratch.head()?;
        scratch.write("draft.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::Squash)?.0, Merged::Stopped);
        assert_eq!(repo.abort_underway()?, Carried::Along);
        assert_eq!(repo.underway(), None);
        assert_eq!(scratch.head()?, before);
        assert_eq!(read(&scratch, "shared.txt")?, "main\n");
        assert!(!scratch.dir.join("b.txt").exists());
        assert_eq!(read(&scratch, "draft.txt")?, "draft\n");
        assert_eq!(scratch.git(&["status", "--porcelain"])?, "?? draft.txt\n");
        Ok(())
    }

    #[test]
    fn a_squash_whose_changes_main_already_has_adds_nothing() -> Result<()> {
        let scratch = topic_ahead("merge-squash-nothing")?;
        commit_file(&scratch, "a.txt", "a\n")?;
        commit_file(&scratch, "b.txt", "b\n")?;
        let before = scratch.head()?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::Squash)?, (Merged::NothingNew, Carried::Along));
        assert_eq!(scratch.head()?, before);
        assert_eq!(repo.underway(), None);
        Ok(())
    }

    #[test]
    fn rebase_then_fast_forward_copies_the_commits_on_top_and_leaves_the_branch() -> Result<()> {
        let scratch = diverged("merge-replay")?;
        let topic = scratch.git(&["rev-parse", "topic"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.merge("topic", MergeMode::RebaseThenFastForward)?,
            (Merged::Replayed(2), Carried::Along)
        );
        assert_eq!(subjects(&scratch, "-4")?, "b\na\nmain\nfirst\n");
        assert_eq!(scratch.git(&["rev-list", "--merges", "--count", "HEAD"])?, "0\n");
        assert_eq!(scratch.git(&["rev-parse", "topic"])?, topic);
        assert_eq!(repo.merge("topic", MergeMode::RebaseThenFastForward)?.0, Merged::NothingNew);
        Ok(())
    }

    #[test]
    fn rebase_then_fast_forward_only_fast_forwards_when_main_has_nothing_of_its_own() -> Result<()> {
        let scratch = topic_ahead("merge-replay-ff")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::RebaseThenFastForward)?.0, Merged::FastForwarded);
        assert_eq!(scratch.head()?, scratch.git(&["rev-parse", "topic"])?.trim());
        Ok(())
    }

    #[test]
    fn a_conflicting_rebase_then_fast_forward_stops_as_a_rebase_and_goes_on() -> Result<()> {
        let scratch = conflicting("merge-replay-conflict")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::RebaseThenFastForward)?.0, Merged::Stopped);
        let Some(Underway::Replay(replay)) = repo.underway() else {
            panic!("a rebase then fast-forward should be underway");
        };
        assert_eq!(replay.from, "topic");
        assert_eq!(replay.picking.branch, "main");
        assert_eq!(replay.picking.commit.subject, "topic");
        scratch.write("shared.txt", "both\n")?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(repo.continue_underway()?, Continued::Finished(Carried::Along));
        assert_eq!(repo.underway(), None);
        assert_eq!(subjects(&scratch, "-3")?, "b\ntopic\nmain\n");
        assert!(!scratch.dir.join(".git/sternegit-replay").exists());
        Ok(())
    }

    #[test]
    fn a_later_cherry_pick_is_not_taken_for_a_rebase() -> Result<()> {
        let scratch = conflicting("merge-replay-stale")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.merge("topic", MergeMode::RebaseThenFastForward)?.0, Merged::Stopped);
        scratch.git(&["cherry-pick", "--abort"])?;
        scratch.git(&["switch", "--quiet", "--create=other", "main~1"])?;
        commit_file(&scratch, "shared.txt", "other\n")?;
        let other = scratch.head()?;
        scratch.git(&["switch", "--quiet", "main"])?;
        assert!(scratch.git(&["cherry-pick", &other]).is_err());
        assert!(matches!(repo.underway(), Some(Underway::CherryPick(_))));
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
        assert_eq!(read(&scratch, "topic.txt")?, "draft\n");
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
    fn a_merge_that_conflicts_stops_for_the_conflicts_to_be_resolved() -> Result<()> {
        let scratch = conflicting("merge-conflict")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.merge("topic", MergeMode::FastForwardIfPossible)?,
            (Merged::Stopped, Carried::Along)
        );
        assert!(matches!(repo.underway(), Some(Underway::Merge(_))));
        Ok(())
    }

    #[test]
    fn merge_and_rebase_need_a_checked_out_branch() -> Result<()> {
        let scratch = forked("combine-detached")?;
        scratch.git(&["checkout", "--quiet", "--detach"])?;
        let (repo, _) = scratch.open()?;
        for mode in EVERY_MODE {
            assert_eq!(refusal(repo.merge("topic", mode)), Refusal::Detached);
        }
        assert_eq!(refusal(repo.rebase_onto("topic")), Refusal::Detached);
        Ok(())
    }

    #[test]
    fn the_squash_message_lists_each_subject_under_the_branch_name() {
        assert_eq!(
            squash_message("feature/blame", &["Add blame".into(), "Show it".into()]),
            "Squashed branch 'feature/blame'\n\n* Add blame\n* Show it\n"
        );
        assert_eq!(squash_message("topic", &[]), "Squashed branch 'topic'\n");
    }
}
