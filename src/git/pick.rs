use anyhow::Result;
use gix::ObjectId;

use super::{
    Carried, Repo,
    branch::is_changes_in_the_way,
    command::{Exit, run},
    merge::Ran,
};

const FIRST_PARENT: &str = "1";
const SEQUENCER: &str = "sequencer";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Picked {
    Committed,
    AlreadyThere,
    Stopped,
}

impl Repo {
    pub(crate) fn cherry_pick(&self, id: ObjectId) -> Result<(Picked, Carried)> {
        let id = id.to_string();
        self.pick(&["cherry-pick", &id])
    }

    pub(crate) fn revert(&self, id: ObjectId, merge: bool) -> Result<(Picked, Carried)> {
        let id = id.to_string();
        let mut args = vec!["revert", "--no-edit"];
        if merge {
            args.extend(["--mainline", FIRST_PARENT]);
        }
        args.push(&id);
        self.pick(&args)
    }

    pub(crate) fn cherry_pick_several(&self, oldest_first: &[ObjectId]) -> Result<(Picked, Carried)> {
        let ids: Vec<String> = oldest_first.iter().map(ObjectId::to_string).collect();
        let mut args = vec!["cherry-pick"];
        args.extend(ids.iter().map(String::as_str));
        self.pick_several(&args)
    }

    pub(crate) fn revert_several(&self, newest_first: &[ObjectId], merges: bool) -> Result<(Picked, Carried)> {
        let ids: Vec<String> = newest_first.iter().map(ObjectId::to_string).collect();
        let mut args = vec!["revert", "--no-edit"];
        if merges {
            args.extend(["--mainline", FIRST_PARENT]);
        }
        args.extend(ids.iter().map(String::as_str));
        self.pick_several(&args)
    }

    fn pick_several(&self, args: &[&str]) -> Result<(Picked, Carried)> {
        let set_aside = self.set_aside_until_done()?;
        let bring_back = |this: &Self| if set_aside { this.bring_back_set_aside() } else { Ok(Carried::Along) };
        match self.try_pick(args) {
            Ok(Picked::Stopped) => Ok((Picked::Stopped, Carried::Along)),
            Ok(picked) => Ok((picked, bring_back(self)?)),
            Err(error) => {
                self.forget_unstarted_sequence(args[0]);
                bring_back(self)?;
                Err(error)
            }
        }
    }

    fn forget_unstarted_sequence(&self, command: &str) {
        if self.underway().is_none() && self.local().git_dir().join(SEQUENCER).is_dir() {
            run(&self.location, &[command, "--quit"], Exit::Success).ok();
        }
    }

    fn pick(&self, args: &[&str]) -> Result<(Picked, Carried)> {
        match self.try_pick(args) {
            Err(error) if is_changes_in_the_way(&error) => {
                self.pick_with_changes_set_aside(args, error)
            }
            picked => picked.map(|picked| (picked, Carried::Along)),
        }
    }

    fn pick_with_changes_set_aside(
        &self,
        args: &[&str],
        in_the_way: anyhow::Error,
    ) -> Result<(Picked, Carried)> {
        if !self.set_aside_until_done()? {
            return Err(in_the_way);
        }
        match self.try_pick(args) {
            Ok(Picked::Stopped) => Ok((Picked::Stopped, Carried::Along)),
            Ok(picked) => Ok((picked, self.bring_back_set_aside()?)),
            Err(error) => {
                self.bring_back_set_aside()?;
                Err(error)
            }
        }
    }

    fn try_pick(&self, args: &[&str]) -> Result<Picked> {
        let before = self.head_now();
        let ran = match self.run_until_stopped(args)? {
            Ran::Stopped if !self.has_conflicts() => self.skip_empty_picks()?,
            ran => ran,
        };
        Ok(match ran {
            Ran::Stopped => Picked::Stopped,
            Ran::Through if self.head_now() == before => Picked::AlreadyThere,
            Ran::Through => Picked::Committed,
        })
    }

    fn head_now(&self) -> Option<ObjectId> {
        self.local().head_id().ok().map(|id| id.detach())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::Picked;
    use crate::git::{Carried, Continued, Place, Underway, scratch::ScratchRepo};

    fn with_topic(label: &str) -> Result<(ScratchRepo, ObjectId)> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("shared.txt", "one\n")?;
        scratch.write("other.txt", "a\n")?;
        scratch.git(&["add", "shared.txt", "other.txt"])?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.write("shared.txt", "topic\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "Change it on topic"])?;
        let picked = id(&scratch, "HEAD")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        Ok((scratch, picked))
    }

    fn id(scratch: &ScratchRepo, revision: &str) -> Result<ObjectId> {
        Ok(ObjectId::from_hex(scratch.git(&["rev-parse", revision])?.trim().as_bytes())?)
    }

    fn subjects(scratch: &ScratchRepo, count: &str) -> Result<String> {
        scratch.git(&["log", "--format=%s", count])
    }

    fn read(scratch: &ScratchRepo, file: &str) -> Result<String> {
        Ok(std::fs::read_to_string(scratch.dir.join(file))?)
    }

    #[test]
    fn a_cherry_pick_copies_the_commit_onto_the_branch() -> Result<()> {
        let (scratch, picked) = with_topic("pick-through")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick(picked)?, (Picked::Committed, Carried::Along));
        assert_eq!(subjects(&scratch, "-2")?, "Change it on topic\nfirst\n");
        assert_eq!(read(&scratch, "shared.txt")?, "topic\n");
        Ok(())
    }

    #[test]
    fn staged_changes_are_set_aside_for_the_cherry_pick_and_come_back_staged() -> Result<()> {
        let (scratch, picked) = with_topic("pick-set-aside")?;
        scratch.write("other.txt", "b\n")?;
        scratch.git(&["add", "other.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick(picked)?, (Picked::Committed, Carried::Along));
        assert_eq!(scratch.git(&["diff", "--cached", "--name-only"])?, "other.txt\n");
        assert_eq!(scratch.git(&["stash", "list"])?, "");
        Ok(())
    }

    #[test]
    fn a_cherry_pick_already_on_the_branch_is_skipped() -> Result<()> {
        let (scratch, picked) = with_topic("pick-twice")?;
        let (repo, _) = scratch.open()?;
        repo.cherry_pick(picked)?;
        assert_eq!(repo.cherry_pick(picked)?.0, Picked::AlreadyThere);
        assert_eq!(repo.underway(), None);
        assert_eq!(subjects(&scratch, "-2")?, "Change it on topic\nfirst\n");
        Ok(())
    }

    #[test]
    fn a_conflicting_cherry_pick_stops_and_brings_the_set_aside_changes_back_when_done()
    -> Result<()> {
        let (scratch, picked) = with_topic("pick-conflict")?;
        scratch.write("shared.txt", "main\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "Change it on main"])?;
        scratch.write("other.txt", "draft\n")?;
        scratch.git(&["add", "other.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick(picked)?, (Picked::Stopped, Carried::Along));
        let Some(Underway::CherryPick(picking)) = repo.underway() else {
            panic!("a cherry-pick should be underway");
        };
        assert_eq!(picking.branch, "main");
        assert_eq!(picking.commit.subject, "Change it on topic");
        assert_eq!(scratch.git(&["stash", "list"])?.lines().count(), 1);
        scratch.write("shared.txt", "both\n")?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(repo.continue_underway()?, Continued::Finished(Carried::Along));
        assert_eq!(repo.underway(), None);
        assert_eq!(subjects(&scratch, "-1")?, "Change it on topic\n");
        assert_eq!(read(&scratch, "other.txt")?, "draft\n");
        assert_eq!(scratch.git(&["stash", "list"])?, "");
        Ok(())
    }

    #[test]
    fn a_resolution_that_keeps_the_branch_as_it_was_skips_the_cherry_pick() -> Result<()> {
        let (scratch, picked) = with_topic("pick-emptied")?;
        scratch.write("shared.txt", "main
")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "Change it on main"])?;
        let before = scratch.head()?;
        scratch.write("other.txt", "draft
")?;
        scratch.git(&["add", "other.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick(picked)?.0, Picked::Stopped);
        scratch.git(&["checkout", "--ours", "shared.txt"])?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(repo.continue_underway()?, Continued::Emptied(Carried::Along));
        assert_eq!(repo.underway(), None);
        assert_eq!(scratch.head()?, before);
        assert_eq!(scratch.git(&["diff", "--cached", "--name-only"])?, "other.txt
");
        Ok(())
    }

    #[test]
    fn aborting_a_cherry_pick_brings_the_set_aside_changes_back() -> Result<()> {
        let (scratch, picked) = with_topic("pick-abort")?;
        scratch.write("shared.txt", "main\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "Change it on main"])?;
        let before = scratch.head()?;
        scratch.write("other.txt", "draft\n")?;
        scratch.git(&["add", "other.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick(picked)?.0, Picked::Stopped);
        assert_eq!(repo.abort_underway()?, Carried::Along);
        assert_eq!(scratch.head()?, before);
        assert_eq!(scratch.git(&["diff", "--cached", "--name-only"])?, "other.txt\n");
        Ok(())
    }

    #[test]
    fn a_revert_adds_a_commit_that_undoes_it() -> Result<()> {
        let (scratch, picked) = with_topic("revert")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.revert(picked, false)?.0, Picked::Committed);
        assert_eq!(read(&scratch, "shared.txt")?, "one\n");
        assert_eq!(subjects(&scratch, "-1")?, "Revert \"Change it on topic\"\n");
        Ok(())
    }

    #[test]
    fn a_merge_is_reverted_against_its_first_parent() -> Result<()> {
        let (scratch, _) = with_topic("revert-merge")?;
        scratch.write("other.txt", "main\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "Change other on main"])?;
        scratch.git(&["merge", "--quiet", "--no-edit", "topic"])?;
        let merge = id(&scratch, "HEAD")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.revert(merge, true)?.0, Picked::Committed);
        assert_eq!(read(&scratch, "shared.txt")?, "one\n");
        assert_eq!(read(&scratch, "other.txt")?, "main\n");
        Ok(())
    }

    fn three_on_topic(label: &str) -> Result<(ScratchRepo, Vec<ObjectId>)> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("shared.txt", "one\n")?;
        scratch.write("other.txt", "a\n")?;
        scratch.git(&["add", "shared.txt", "other.txt"])?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        for (file, text, subject) in [("b.txt", "b\n", "Add b"), ("shared.txt", "topic\n", "Change shared"), ("c.txt", "c\n", "Add c")] {
            scratch.write(file, text)?;
            scratch.git(&["add", file])?;
            scratch.git(&["commit", "--quiet", "-m", subject])?;
        }
        let picked = ["topic~2", "topic~1", "topic"].iter().map(|revision| id(&scratch, revision)).collect::<Result<_>>()?;
        scratch.git(&["switch", "--quiet", "main"])?;
        Ok((scratch, picked))
    }

    #[test]
    fn several_commits_are_cherry_picked_oldest_first_around_staged_changes() -> Result<()> {
        let (scratch, oldest_first) = three_on_topic("pick-several")?;
        scratch.write("other.txt", "staged\n")?;
        scratch.git(&["add", "other.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick_several(&oldest_first)?, (Picked::Committed, Carried::Along));
        assert_eq!(subjects(&scratch, "-4")?, "Add c\nChange shared\nAdd b\nfirst\n");
        assert_eq!(scratch.git(&["diff", "--cached", "--name-only"])?, "other.txt\n");
        assert_eq!(scratch.git(&["stash", "list"])?, "");
        assert!(!scratch.dir.join(".git/sequencer").exists());
        Ok(())
    }

    #[test]
    fn a_conflict_in_the_middle_stops_at_its_place_and_continuing_picks_the_rest() -> Result<()> {
        let (scratch, oldest_first) = three_on_topic("pick-several-conflict")?;
        scratch.write("shared.txt", "main\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "Change it on main"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.cherry_pick_several(&oldest_first)?.0, Picked::Stopped);
        let Some(Underway::CherryPick(picking)) = repo.underway() else {
            panic!("a cherry-pick should be underway");
        };
        assert_eq!(picking.commit.subject, "Change shared");
        assert_eq!(picking.place, Some(Place { at: 2, of: 3 }));
        scratch.write("shared.txt", "both\n")?;
        scratch.git(&["add", "shared.txt"])?;
        assert_eq!(repo.continue_underway()?, Continued::Finished(Carried::Along));
        assert_eq!(repo.underway(), None);
        assert_eq!(subjects(&scratch, "-3")?, "Add c\nChange shared\nAdd b\n");
        Ok(())
    }

    #[test]
    fn a_commit_the_branch_already_has_is_skipped_and_the_others_picked() -> Result<()> {
        let (scratch, oldest_first) = three_on_topic("pick-several-skip")?;
        let (repo, _) = scratch.open()?;
        repo.cherry_pick(oldest_first[1])?;
        assert_eq!(repo.cherry_pick_several(&oldest_first)?.0, Picked::Committed);
        assert_eq!(repo.underway(), None);
        assert_eq!(subjects(&scratch, "-4")?, "Add c\nAdd b\nChange shared\nfirst\n");
        assert_eq!(repo.cherry_pick_several(&oldest_first)?.0, Picked::AlreadyThere);
        Ok(())
    }

    #[test]
    fn several_commits_are_reverted_newest_first_merges_against_their_first_parent() -> Result<()> {
        let (scratch, oldest_first) = three_on_topic("revert-several")?;
        let (repo, _) = scratch.open()?;
        repo.cherry_pick_several(&oldest_first[..2])?;
        scratch.git(&["merge", "--quiet", "--no-ff", "--no-edit", "topic"])?;
        let newest_first = [id(&scratch, "main")?, id(&scratch, "main~1")?];
        assert_eq!(repo.revert_several(&newest_first, true)?.0, Picked::Committed);
        let reverts = subjects(&scratch, "-2")?;
        assert!(reverts.starts_with("Revert \"Change shared\"\nRevert \"Merge branch"), "{reverts}");
        assert_eq!(read(&scratch, "shared.txt")?, "one\n");
        assert!(!scratch.dir.join("c.txt").exists());
        Ok(())
    }
}
