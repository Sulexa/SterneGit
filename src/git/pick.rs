use anyhow::Result;
use gix::ObjectId;

use super::{Carried, Repo, branch::is_changes_in_the_way, merge::Ran};

const FIRST_PARENT: &str = "1";

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
        match self.run_until_stopped(args)? {
            Ran::Through => Ok(Picked::Committed),
            Ran::Stopped if self.has_conflicts() => Ok(Picked::Stopped),
            Ran::Stopped => {
                self.skip_underway()?;
                Ok(Picked::AlreadyThere)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::Picked;
    use crate::git::{Carried, Continued, Underway, scratch::ScratchRepo};

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
}
