use anyhow::Result;
use gix::bstr::BStr;

use super::{
    RefKind, RefLabel, Repo, Tracking,
    command::{Exit, run},
    refusal::why_git_stopped,
};

const BRANCHES: &str = "refs/heads/";
const REMOTE_BRANCHES: &str = "refs/remotes/";
const NAMES_GIT_RESERVES: [&str; 1] = ["@"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NameProblem {
    Empty,
    Taken,
    Refused,
}

pub(crate) fn new_branch_name<'a>(typed: &'a str, refs: &[RefLabel]) -> Result<&'a str, NameProblem> {
    let name = typed.trim();
    if name.is_empty() {
        return Err(NameProblem::Empty);
    }
    let full = format!("{BRANCHES}{name}");
    let refused = name.starts_with('-')
        || NAMES_GIT_RESERVES.contains(&name)
        || gix::validate::reference::branch_name(BStr::new(&full)).is_err();
    if refused {
        return Err(NameProblem::Refused);
    }
    let taken = refs
        .iter()
        .any(|label| label.kind == RefKind::LocalBranch && label.name.as_str() == name);
    if taken {
        return Err(NameProblem::Taken);
    }
    Ok(name)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CheckedOut {
    Existing(String),
    Tracking(Tracking),
}

impl Repo {
    pub(crate) fn create_branch(&self, name: &str, start: Option<&str>) -> Result<()> {
        let create = format!("--create={name}");
        let mut args = vec!["switch", "--quiet", create.as_str()];
        args.extend(start);
        self.switch(&args)
    }

    pub(crate) fn check_out(&self, branch: &str) -> Result<()> {
        self.switch(&["switch", "--quiet", branch])
    }

    pub(crate) fn check_out_remote(&self, remote_branch: &str, local: &str) -> Result<CheckedOut> {
        if self.has_branch(local) {
            self.check_out(local)?;
            return Ok(CheckedOut::Existing(local.to_owned()));
        }
        let create = format!("--create={local}");
        let start = format!("{REMOTE_BRANCHES}{remote_branch}");
        self.switch(&["switch", "--quiet", &create, "--track", &start])?;
        Ok(CheckedOut::Tracking(Tracking {
            branch: local.to_owned(),
            upstream: remote_branch.to_owned(),
        }))
    }

    pub(crate) fn delete_branch(&self, branch: &str) -> Result<()> {
        run(
            &self.location,
            &["branch", "--quiet", "--delete", "--force", branch],
            Exit::Success,
        )
        .map(drop)
    }

    pub(crate) fn delete_remote_branch(&self, remote: &str, branch: &str) -> Result<()> {
        let there = format!("{BRANCHES}{branch}");
        run(
            &self.location,
            &["push", "--quiet", remote, "--delete", &there],
            Exit::Success,
        )
        .map(drop)
    }

    fn has_branch(&self, name: &str) -> bool {
        self.local()
            .try_find_reference(format!("{BRANCHES}{name}").as_str())
            .is_ok_and(|found| found.is_some())
    }

    fn switch(&self, args: &[&str]) -> Result<()> {
        run(&self.location, args, Exit::Success)
            .map(drop)
            .map_err(|error| why_git_stopped(&self.local(), error))
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{CheckedOut, NameProblem, new_branch_name};
    use crate::git::{Refusal, Tracking, scratch::ScratchRepo};

    fn current_branch(scratch: &ScratchRepo) -> Result<String> {
        Ok(scratch
            .git(&["symbolic-ref", "--short", "HEAD"])?
            .trim()
            .to_owned())
    }

    fn with_remote_branch(label: &str) -> Result<(ScratchRepo, ScratchRepo)> {
        let origin = ScratchRepo::bare(&format!("{label}-origin"))?;
        let scratch = ScratchRepo::new(label)?;
        scratch.commit("first")?;
        scratch.git(&["remote", "add", "origin", &origin.location()])?;
        scratch.git(&["push", "--quiet", "origin", "main", "main:feature/x"])?;
        scratch.git(&["fetch", "--quiet", "origin"])?;
        Ok((origin, scratch))
    }

    #[test]
    fn checking_out_switches_branches_unless_changes_are_in_the_way() -> Result<()> {
        let scratch = ScratchRepo::new("check-out")?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.write("shared.txt", "topic\n")?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "topic"])?;
        scratch.git(&["switch", "--quiet", "main"])?;
        let (repo, _) = scratch.open()?;
        scratch.write("shared.txt", "draft\n")?;
        let refused = repo.check_out("topic").expect_err("the draft is in the way");
        assert_eq!(
            refused.downcast_ref::<Refusal>(),
            Some(&Refusal::ChangesInTheWay)
        );
        assert_eq!(current_branch(&scratch)?, "main");
        scratch.git(&["checkout", "--quiet", "--", "shared.txt"])?;
        repo.check_out("topic")?;
        assert_eq!(current_branch(&scratch)?, "topic");
        Ok(())
    }

    #[test]
    fn checking_out_a_remote_branch_tracks_it_or_reuses_the_local_one() -> Result<()> {
        let (_origin, scratch) = with_remote_branch("check-out-remote")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.check_out_remote("origin/feature/x", "feature/x")?,
            CheckedOut::Tracking(Tracking {
                branch: "feature/x".into(),
                upstream: "origin/feature/x".into()
            })
        );
        assert_eq!(current_branch(&scratch)?, "feature/x");
        assert_eq!(
            scratch
                .git(&["rev-parse", "--abbrev-ref", "feature/x@{upstream}"])?
                .trim(),
            "origin/feature/x"
        );
        scratch.git(&["switch", "--quiet", "main"])?;
        assert_eq!(
            repo.check_out_remote("origin/feature/x", "feature/x")?,
            CheckedOut::Existing("feature/x".into())
        );
        assert_eq!(current_branch(&scratch)?, "feature/x");
        Ok(())
    }

    #[test]
    fn a_new_branch_can_start_at_another_ref() -> Result<()> {
        let scratch = ScratchRepo::new("branch-from")?;
        scratch.commit("first")?;
        scratch.git(&["branch", "topic"])?;
        scratch.commit("second")?;
        let (repo, _) = scratch.open()?;
        repo.create_branch("fix", Some("topic"))?;
        assert_eq!(current_branch(&scratch)?, "fix");
        assert_eq!(scratch.head()?, scratch.git(&["rev-parse", "topic"])?.trim());
        Ok(())
    }

    #[test]
    fn deleting_a_branch_removes_it_even_with_commits_only_it_has() -> Result<()> {
        let scratch = ScratchRepo::new("delete-branch")?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.commit("only here")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        let (repo, _) = scratch.open()?;
        repo.delete_branch("topic")?;
        assert_eq!(scratch.git(&["branch", "--list", "topic"])?, "");
        Ok(())
    }

    #[test]
    fn deleting_a_remote_branch_removes_it_from_the_remote() -> Result<()> {
        let (origin, scratch) = with_remote_branch("delete-remote-branch")?;
        let (repo, _) = scratch.open()?;
        repo.delete_remote_branch("origin", "feature/x")?;
        assert_eq!(origin.git(&["branch", "--list", "feature/x"])?, "");
        assert_eq!(
            scratch.git(&["branch", "--remotes", "--list", "origin/feature/x"])?,
            ""
        );
        Ok(())
    }

    #[test]
    fn a_new_branch_starts_where_head_is_and_is_checked_out() -> Result<()> {
        let scratch = ScratchRepo::new("create-branch")?;
        scratch.commit("first")?;
        scratch.write("draft.txt", "kept\n")?;
        let head = scratch.head()?;
        let (repo, history) = scratch.open()?;
        let name = new_branch_name(" feature/toolbar ", &history.refs).expect("a fine name");
        repo.create_branch(name, None)?;
        assert_eq!(
            scratch.git(&["symbolic-ref", "--short", "HEAD"])?.trim(),
            "feature/toolbar"
        );
        assert_eq!(scratch.head()?, head);
        assert!(scratch.dir.join("draft.txt").exists());
        Ok(())
    }

    #[test]
    fn names_git_refuses_or_that_exist_cannot_make_a_branch() -> Result<()> {
        let scratch = ScratchRepo::new("branch-names")?;
        scratch.commit("first")?;
        scratch.git(&["branch", "topic/tabs"])?;
        scratch.git(&["tag", "v1"])?;
        let (_, history) = scratch.open()?;
        let check = |name: &str| new_branch_name(name, &history.refs).map(str::to_owned);
        assert_eq!(check("  "), Err(NameProblem::Empty));
        assert_eq!(check("topic/tabs"), Err(NameProblem::Taken));
        assert_eq!(check("main"), Err(NameProblem::Taken));
        for refused in [
            "my branch",
            "a..b",
            "-f",
            "HEAD",
            "@",
            "x.lock",
            "end/",
            "what?",
            "star*",
            "tilde~1",
            "caret^",
            "colon:",
            "back\\slash",
            "[bracket",
            "at@{1}",
        ] {
            assert_eq!(check(refused), Err(NameProblem::Refused), "{refused}");
        }
        assert_eq!(check("v1").as_deref(), Ok("v1"));
        assert_eq!(check("fix/merge-2").as_deref(), Ok("fix/merge-2"));
        Ok(())
    }
}
