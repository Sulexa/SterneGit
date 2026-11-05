use anyhow::Result;
use gix::bstr::BStr;

use super::{
    Carried, RefKind, RefLabel, Refusal, Repo, Tracking,
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
    pub(crate) fn create_branch(&self, name: &str, start: Option<&str>) -> Result<Carried> {
        let create = format!("--create={name}");
        let mut args = vec!["switch", "--quiet", create.as_str()];
        args.extend(start);
        self.switch(&args)
    }

    pub(crate) fn check_out(&self, branch: &str) -> Result<Carried> {
        self.switch(&["switch", "--quiet", branch])
    }

    pub(crate) fn check_out_remote(
        &self,
        remote_branch: &str,
        local: &str,
    ) -> Result<(CheckedOut, Carried)> {
        if self.has_branch(local) {
            let carried = self.check_out(local)?;
            return Ok((CheckedOut::Existing(local.to_owned()), carried));
        }
        let create = format!("--create={local}");
        let start = format!("{REMOTE_BRANCHES}{remote_branch}");
        let carried = self.switch(&["switch", "--quiet", &create, "--track", &start])?;
        let tracking = Tracking {
            branch: local.to_owned(),
            upstream: remote_branch.to_owned(),
        };
        Ok((CheckedOut::Tracking(tracking), carried))
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

    fn switch(&self, args: &[&str]) -> Result<Carried> {
        match self.try_switch(args) {
            Ok(()) => Ok(Carried::Along),
            Err(error) if is_changes_in_the_way(&error) => {
                self.switch_with_changes_stashed(args, error)
            }
            Err(error) => Err(error),
        }
    }

    fn switch_with_changes_stashed(
        &self,
        args: &[&str],
        in_the_way: anyhow::Error,
    ) -> Result<Carried> {
        if !self.stash_aside()? {
            return Err(in_the_way);
        }
        if let Err(refused) = self.try_switch(args) {
            self.reapply_stash()?;
            return Err(refused);
        }
        Ok(self.reapply_stash().unwrap_or(Carried::OnlyInStash))
    }

    fn try_switch(&self, args: &[&str]) -> Result<()> {
        run(&self.location, args, Exit::Success)
            .map(drop)
            .map_err(why_git_stopped)
    }
}

fn is_changes_in_the_way(error: &anyhow::Error) -> bool {
    error.downcast_ref::<Refusal>() == Some(&Refusal::ChangesInTheWay)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{CheckedOut, NameProblem, new_branch_name};
    use crate::git::{Carried, ChangeKind, FileChange, Tracking, scratch::ScratchRepo};

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

    const EIGHT_LINES: &str = "1\n2\n3\n4\n5\n6\n7\n8\n";

    fn with_topic_changing(label: &str, on_main: &str, on_topic: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("shared.txt", on_main)?;
        scratch.write("other.txt", "a\n")?;
        scratch.git(&["add", "shared.txt", "other.txt"])?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.write("shared.txt", on_topic)?;
        scratch.git(&["commit", "--quiet", "--all", "-m", "topic"])?;
        scratch.git(&["switch", "--quiet", "main"])?;
        Ok(scratch)
    }

    fn staged_and_unstaged(scratch: &ScratchRepo) -> Result<(Vec<String>, Vec<String>)> {
        let status = scratch.open()?.0.status()?;
        let paths = |changes: &[FileChange]| {
            changes
                .iter()
                .map(|change| change.path.to_string())
                .collect()
        };
        Ok((paths(&status.staged), paths(&status.unstaged)))
    }

    fn stashes(scratch: &ScratchRepo) -> Result<usize> {
        Ok(scratch.git(&["stash", "list"])?.lines().count())
    }

    #[test]
    fn changes_the_branch_leaves_alone_come_along_without_a_stash() -> Result<()> {
        let scratch = with_topic_changing("check-out-along", "one\n", "topic\n")?;
        scratch.write("other.txt", "b\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.check_out("topic")?, Carried::Along);
        assert_eq!(current_branch(&scratch)?, "topic");
        assert_eq!(
            staged_and_unstaged(&scratch)?,
            (vec![], vec!["other.txt".into()])
        );
        assert_eq!(stashes(&scratch)?, 0);
        Ok(())
    }

    #[test]
    fn changes_in_the_way_are_stashed_and_come_back_staged_or_not_on_the_branch() -> Result<()> {
        let on_topic = EIGHT_LINES.replace("8\n", "topic\n");
        let scratch = with_topic_changing("check-out-stashed", EIGHT_LINES, &on_topic)?;
        scratch.write("older.txt", "older\n")?;
        scratch.git(&["stash", "push", "--include-untracked", "--quiet"])?;
        scratch.write("other.txt", "b\n")?;
        scratch.git(&["add", "other.txt"])?;
        scratch.write("shared.txt", &EIGHT_LINES.replace("1\n", "draft\n"))?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.check_out("topic")?, Carried::Along);
        assert_eq!(current_branch(&scratch)?, "topic");
        assert_eq!(
            staged_and_unstaged(&scratch)?,
            (vec!["other.txt".into()], vec!["shared.txt".into()])
        );
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("shared.txt"))?,
            on_topic.replace("1\n", "draft\n")
        );
        assert_eq!(stashes(&scratch)?, 1);
        assert_eq!(scratch.git(&["show", "stash@{0}^3:older.txt"])?, "older\n");
        Ok(())
    }

    #[test]
    fn changes_that_conflict_with_the_branch_are_left_to_resolve_and_kept_in_the_stash()
    -> Result<()> {
        let scratch = with_topic_changing("check-out-conflict", "one\n", "topic\n")?;
        scratch.write("shared.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.check_out("topic")?, Carried::WithConflicts);
        assert_eq!(current_branch(&scratch)?, "topic");
        let status = repo.status()?;
        assert!(
            status
                .conflicted
                .iter()
                .any(|change| change.kind == ChangeKind::Conflicted)
        );
        assert_eq!(stashes(&scratch)?, 1);
        Ok(())
    }

    #[test]
    fn staged_changes_the_branch_contradicts_stay_only_in_the_stash() -> Result<()> {
        let scratch = with_topic_changing("check-out-staged-conflict", "one\n", "topic\n")?;
        scratch.write("shared.txt", "draft\n")?;
        scratch.git(&["add", "shared.txt"])?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.check_out("topic")?, Carried::OnlyInStash);
        assert_eq!(current_branch(&scratch)?, "topic");
        assert_eq!(staged_and_unstaged(&scratch)?, (vec![], vec![]));
        assert_eq!(scratch.git(&["show", "stash@{0}:shared.txt"])?, "draft\n");
        Ok(())
    }

    #[test]
    fn an_untracked_file_the_branch_tracks_stays_only_in_the_stash() -> Result<()> {
        let scratch = with_topic_changing("check-out-untracked", "one\n", "topic\n")?;
        scratch.git(&["switch", "--quiet", "topic"])?;
        scratch.write("new.txt", "topic\n")?;
        scratch.git(&["add", "new.txt"])?;
        scratch.commit("add new.txt")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.write("new.txt", "mine\n")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.check_out("topic")?, Carried::OnlyInStash);
        assert_eq!(current_branch(&scratch)?, "topic");
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("new.txt"))?,
            "topic\n"
        );
        assert_eq!(scratch.git(&["show", "stash@{0}^3:new.txt"])?, "mine\n");
        Ok(())
    }

    #[test]
    fn a_check_out_git_refuses_for_another_reason_stashes_nothing() -> Result<()> {
        let scratch = with_topic_changing("check-out-missing", "one\n", "topic\n")?;
        scratch.write("shared.txt", "draft\n")?;
        let (repo, _) = scratch.open()?;
        repo.check_out("no-such-branch")
            .expect_err("there is no such branch");
        assert_eq!(current_branch(&scratch)?, "main");
        assert_eq!(
            staged_and_unstaged(&scratch)?,
            (vec![], vec!["shared.txt".into()])
        );
        assert_eq!(stashes(&scratch)?, 0);
        Ok(())
    }

    #[test]
    fn checking_out_a_remote_branch_tracks_it_or_reuses_the_local_one() -> Result<()> {
        let (_origin, scratch) = with_remote_branch("check-out-remote")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.check_out_remote("origin/feature/x", "feature/x")?,
            (
                CheckedOut::Tracking(Tracking {
                    branch: "feature/x".into(),
                    upstream: "origin/feature/x".into()
                }),
                Carried::Along
            )
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
            (CheckedOut::Existing("feature/x".into()), Carried::Along)
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
        assert_eq!(
            scratch.head()?,
            scratch.git(&["rev-parse", "topic"])?.trim()
        );
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
