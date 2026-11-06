use anyhow::Result;
use gix::{ObjectId, remote::Direction};

use super::{
    NameProblem, RefKind, RefLabel, Repo,
    branch::new_ref_name,
    command::{Exit, run},
};

const TAGS: &str = "refs/tags/";

pub(crate) fn new_tag_name<'a>(typed: &'a str, refs: &[RefLabel]) -> Result<&'a str, NameProblem> {
    new_ref_name(typed, refs, RefKind::Tag, TAGS)
}

impl Repo {
    pub(crate) fn create_tag(&self, name: &str, id: ObjectId, message: Option<&str>) -> Result<()> {
        let id = id.to_string();
        let mut args = vec!["tag"];
        if let Some(message) = message {
            args.extend(["--annotate", "--message", message]);
        }
        args.extend([name, &id]);
        run(&self.location, &args, Exit::Success).map(drop)
    }

    pub(crate) fn push_tag(&self, remote: &str, name: &str) -> Result<()> {
        let tag = format!("{TAGS}{name}");
        self.reach(&["push", "--quiet", remote, &tag]).map(drop)
    }

    pub(crate) fn push_remote(&self) -> Option<String> {
        self.local()
            .remote_default_name(Direction::Push)
            .map(|name| name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::new_tag_name;
    use crate::git::{NameProblem, scratch::ScratchRepo};

    fn with_two_commits(label: &str) -> Result<(ScratchRepo, ObjectId)> {
        let scratch = ScratchRepo::new(label)?;
        scratch.commit("first")?;
        let first = ObjectId::from_hex(scratch.head()?.as_bytes())?;
        scratch.commit("second")?;
        Ok((scratch, first))
    }

    #[test]
    fn a_tag_without_a_message_is_lightweight() -> Result<()> {
        let (scratch, first) = with_two_commits("tag-light")?;
        scratch.open()?.0.create_tag("v1", first, None)?;
        assert_eq!(scratch.git(&["cat-file", "-t", "v1"])?.trim(), "commit");
        assert_eq!(scratch.git(&["rev-parse", "v1"])?.trim(), first.to_string());
        Ok(())
    }

    #[test]
    fn a_tag_with_a_message_is_annotated() -> Result<()> {
        let (scratch, first) = with_two_commits("tag-annotated")?;
        scratch.open()?.0.create_tag("v1", first, Some("First release"))?;
        assert_eq!(scratch.git(&["cat-file", "-t", "v1"])?.trim(), "tag");
        assert_eq!(scratch.git(&["tag", "-l", "--format=%(contents)", "v1"])?.trim(), "First release");
        assert_eq!(scratch.git(&["rev-parse", "v1^{commit}"])?.trim(), first.to_string());
        Ok(())
    }

    #[test]
    fn a_pushed_tag_reaches_the_remote() -> Result<()> {
        let origin = ScratchRepo::bare("tag-origin")?;
        let (scratch, first) = with_two_commits("tag-push")?;
        scratch.git(&["remote", "add", "origin", &origin.location()])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.push_remote().as_deref(), Some("origin"));
        repo.create_tag("v1", first, None)?;
        repo.push_tag("origin", "v1")?;
        assert_eq!(origin.git(&["rev-parse", "v1"])?.trim(), first.to_string());
        Ok(())
    }

    #[test]
    fn a_tag_name_must_be_new_and_one_git_accepts() -> Result<()> {
        let (scratch, first) = with_two_commits("tag-names")?;
        scratch.open()?.0.create_tag("v1", first, None)?;
        let (_, history) = scratch.open()?;
        assert_eq!(new_tag_name(" v2 ", &history.refs), Ok("v2"));
        assert_eq!(new_tag_name("v1", &history.refs), Err(NameProblem::Taken));
        assert_eq!(new_tag_name("main", &history.refs), Ok("main"));
        assert_eq!(new_tag_name("a b", &history.refs), Err(NameProblem::Refused));
        assert_eq!(new_tag_name("", &history.refs), Err(NameProblem::Empty));
        Ok(())
    }
}
