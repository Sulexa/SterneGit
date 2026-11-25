use anyhow::{Result, anyhow};
use gix::{ObjectId, remote::Direction};

use super::{
    NameProblem, RefKind, RefLabel, Refusal, Repo,
    branch::new_ref_name,
    command::{Exit, run, run_with_access},
    text::decode,
};

const TAGS: &str = "refs/tags/";
const REPORT_FIELDS: char = '\t';
const UP_TO_DATE: char = '=';
const REJECTED: char = '!';
const TAKEN_THERE: &str = "(already exists)";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TagPushed {
    Sent,
    AlreadyThere,
}

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

    pub(crate) fn push_tag(&self, remote: &str, name: &str) -> Result<TagPushed> {
        let pushed = self.push_tags(remote, &[name])?;
        Ok(pushed.into_iter().next().unwrap_or(TagPushed::Sent))
    }

    pub(crate) fn push_tags(&self, remote: &str, names: &[&str]) -> Result<Vec<TagPushed>> {
        let tags: Vec<String> = names.iter().map(|name| format!("{TAGS}{name}")).collect();
        let mut args = vec!["push", "--porcelain", remote];
        args.extend(tags.iter().map(String::as_str));
        let report = run_with_access(&self.location, &self.access, &args, Exit::SuccessOrDifference)?;
        let report = decode(&report);
        names
            .iter()
            .zip(&tags)
            .map(|(name, tag)| {
                read_push_report(&report, tag).map_err(|rejection| match rejection {
                    Rejection::TakenThere => Refusal::TagTakenThere {
                        tag: (*name).to_owned(),
                        remote: remote.to_owned(),
                    }
                    .into(),
                    Rejection::Other(reason) => anyhow!(reason),
                })
            })
            .collect()
    }

    pub(crate) fn delete_tag(&self, name: &str) -> Result<()> {
        self.delete_tags(&[name])
    }

    pub(crate) fn delete_tags(&self, names: &[&str]) -> Result<()> {
        let mut args = vec!["tag", "--delete"];
        args.extend_from_slice(names);
        run(&self.location, &args, Exit::Success).map(drop)
    }

    pub(crate) fn delete_remote_tag(&self, remote: &str, name: &str) -> Result<()> {
        self.delete_remote_tags(remote, &[name])
    }

    pub(crate) fn delete_remote_tags(&self, remote: &str, names: &[&str]) -> Result<()> {
        let there: Vec<String> = names.iter().map(|name| format!("{TAGS}{name}")).collect();
        let mut args = vec!["push", "--quiet", remote, "--delete"];
        args.extend(there.iter().map(String::as_str));
        self.reach(&args).map(drop)
    }

    pub(crate) fn push_remote(&self) -> Option<String> {
        self.local()
            .remote_default_name(Direction::Push)
            .map(|name| name.to_string())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Rejection {
    TakenThere,
    Other(String),
}

fn read_push_report(report: &str, tag: &str) -> Result<TagPushed, Rejection> {
    let pushed = format!("{tag}:{tag}");
    let line = report.lines().find_map(|line| {
        let mut fields = line.split(REPORT_FIELDS);
        let flag = fields.next()?.chars().next()?;
        (fields.next()? == pushed).then(|| (flag, fields.next().unwrap_or_default()))
    });
    match line {
        Some((UP_TO_DATE, _)) => Ok(TagPushed::AlreadyThere),
        Some((REJECTED, summary)) if summary.contains(TAKEN_THERE) => Err(Rejection::TakenThere),
        Some((REJECTED, summary)) => Err(Rejection::Other(summary.to_owned())),
        Some(_) => Ok(TagPushed::Sent),
        None => Err(Rejection::Other(report.trim().to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use super::{Rejection, TagPushed, new_tag_name, read_push_report};
    use crate::git::{NameProblem, Refusal, scratch::ScratchRepo};

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
        assert_eq!(repo.push_tag("origin", "v1")?, TagPushed::Sent);
        assert_eq!(origin.git(&["rev-parse", "v1"])?.trim(), first.to_string());
        assert_eq!(repo.push_tag("origin", "v1")?, TagPushed::AlreadyThere);
        Ok(())
    }

    #[test]
    fn a_tag_the_remote_has_elsewhere_is_refused_by_name() -> Result<()> {
        let origin = ScratchRepo::bare("tag-taken-origin")?;
        let (scratch, first) = with_two_commits("tag-taken")?;
        scratch.git(&["remote", "add", "origin", &origin.location()])?;
        let (repo, _) = scratch.open()?;
        repo.create_tag("v1", first, None)?;
        repo.push_tag("origin", "v1")?;
        repo.delete_tag("v1")?;
        scratch.git(&["tag", "v1"])?;
        let refused = repo.push_tag("origin", "v1").unwrap_err();
        assert_eq!(
            refused.downcast_ref::<Refusal>(),
            Some(&Refusal::TagTakenThere { tag: "v1".into(), remote: "origin".into() })
        );
        assert_eq!(origin.git(&["rev-parse", "v1"])?.trim(), first.to_string());
        Ok(())
    }

    #[test]
    fn a_tag_is_deleted_here_or_on_the_remote_alone() -> Result<()> {
        let origin = ScratchRepo::bare("tag-delete-origin")?;
        let (scratch, first) = with_two_commits("tag-delete")?;
        scratch.git(&["remote", "add", "origin", &origin.location()])?;
        let (repo, _) = scratch.open()?;
        repo.create_tag("v1", first, Some("First release"))?;
        repo.push_tag("origin", "v1")?;
        repo.delete_remote_tag("origin", "v1")?;
        assert!(origin.git(&["tag", "--list"])?.trim().is_empty());
        assert_eq!(scratch.git(&["tag", "--list"])?.trim(), "v1");
        repo.delete_tag("v1")?;
        assert!(scratch.git(&["tag", "--list"])?.trim().is_empty());
        Ok(())
    }

    #[test]
    fn several_tags_are_pushed_and_deleted_in_one_go() -> Result<()> {
        let origin = ScratchRepo::bare("tags-origin")?;
        let (scratch, first) = with_two_commits("tags-several")?;
        scratch.git(&["remote", "add", "origin", &origin.location()])?;
        let (repo, _) = scratch.open()?;
        repo.create_tag("v1", first, None)?;
        repo.create_tag("v2", first, Some("Second release"))?;
        repo.push_tag("origin", "v1")?;
        assert_eq!(
            repo.push_tags("origin", &["v1", "v2"])?,
            [TagPushed::AlreadyThere, TagPushed::Sent]
        );
        assert_eq!(origin.git(&["tag", "--list"])?.trim(), "v1\nv2");
        repo.delete_remote_tags("origin", &["v1", "v2"])?;
        assert!(origin.git(&["tag", "--list"])?.trim().is_empty());
        repo.delete_tags(&["v1", "v2"])?;
        assert!(scratch.git(&["tag", "--list"])?.trim().is_empty());
        Ok(())
    }

    #[test]
    fn the_push_report_says_whether_the_tag_was_sent_already_there_or_rejected() {
        let tag = "refs/tags/v1";
        let report = |line: &str| format!("To ../origin\n{line}\nDone\n");
        assert_eq!(
            read_push_report(&report("*\trefs/tags/v1:refs/tags/v1\t[new tag]"), tag),
            Ok(TagPushed::Sent)
        );
        assert_eq!(
            read_push_report(&report("=\trefs/tags/v1:refs/tags/v1\t[up to date]"), tag),
            Ok(TagPushed::AlreadyThere)
        );
        assert_eq!(
            read_push_report(&report("!\trefs/tags/v1:refs/tags/v1\t[rejected] (already exists)"), tag),
            Err(Rejection::TakenThere)
        );
        assert_eq!(
            read_push_report(&report("!\trefs/tags/v1:refs/tags/v1\t[remote rejected] (hook declined)"), tag),
            Err(Rejection::Other("[remote rejected] (hook declined)".into()))
        );
        let other_tag = report("*\trefs/tags/v10:refs/tags/v10\t[new tag]");
        assert_eq!(read_push_report(&other_tag, tag), Err(Rejection::Other(other_tag.trim().into())));
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
