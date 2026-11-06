use anyhow::Result;
use gix::ObjectId;

use super::{
    Repo,
    command::{Exit, run},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

impl ResetMode {
    pub(crate) const ALL: [ResetMode; 3] = [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard];

    fn flag(self) -> &'static str {
        match self {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        }
    }
}

impl Repo {
    pub(crate) fn reset(&self, id: ObjectId, mode: ResetMode) -> Result<()> {
        let id = id.to_string();
        run(&self.location, &["reset", "--quiet", mode.flag(), &id], Exit::Success).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::ResetMode;
    use crate::git::scratch::ScratchRepo;

    fn two_commits(label: &str) -> Result<(ScratchRepo, ObjectId)> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("file.txt", "one\n")?;
        scratch.git(&["add", "file.txt"])?;
        scratch.commit("first")?;
        let first = ObjectId::from_hex(scratch.head()?.as_bytes())?;
        scratch.write("file.txt", "two\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "second"])?;
        scratch.write("draft.txt", "draft\n")?;
        Ok((scratch, first))
    }

    fn state(scratch: &ScratchRepo) -> Result<(String, String, String)> {
        Ok((
            scratch.git(&["log", "--format=%s"])?,
            scratch.git(&["diff", "--cached", "--name-only"])?,
            std::fs::read_to_string(scratch.dir.join("file.txt"))?,
        ))
    }

    #[test]
    fn soft_keeps_what_the_branch_leaves_behind_staged() -> Result<()> {
        let (scratch, first) = two_commits("reset-soft")?;
        scratch.open()?.0.reset(first, ResetMode::Soft)?;
        assert_eq!(state(&scratch)?, ("first\n".into(), "file.txt\n".into(), "two\n".into()));
        Ok(())
    }

    #[test]
    fn mixed_keeps_it_unstaged() -> Result<()> {
        let (scratch, first) = two_commits("reset-mixed")?;
        scratch.open()?.0.reset(first, ResetMode::Mixed)?;
        assert_eq!(state(&scratch)?, ("first\n".into(), String::new(), "two\n".into()));
        Ok(())
    }

    #[test]
    fn hard_throws_tracked_changes_away_and_leaves_untracked_files() -> Result<()> {
        let (scratch, first) = two_commits("reset-hard")?;
        scratch.open()?.0.reset(first, ResetMode::Hard)?;
        assert_eq!(state(&scratch)?, ("first\n".into(), String::new(), "one\n".into()));
        assert!(scratch.dir.join("draft.txt").exists());
        Ok(())
    }
}
