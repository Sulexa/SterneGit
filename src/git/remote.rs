use anyhow::Result;
use gix::{ObjectId, refs::FullName, remote::Direction, state::InProgress};

use super::{
    PullMode, Refusal, Repo, Tracking,
    command::{Exit, run, run_text},
    text::decode,
};

const UPSTREAM_OF_HEAD: [&str; 3] = ["rev-parse", "--symbolic-full-name", "@{upstream}"];
const FAST_FORWARD_IMPOSSIBLE: &str = "Not possible to fast-forward";
const CHANGES_IN_THE_WAY: [&str; 2] = [
    "cannot pull with rebase",
    "would be overwritten by merge",
];
const PUSH_REJECTED: &str = "[rejected]";
const REJECTED_FOR_MISSING_COMMITS: [&str; 2] = ["(fetch first)", "(non-fast-forward)"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fetched {
    NoRemotes,
    AllRemotes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pulled {
    Fetched,
    UpToDate(Tracking),
    BroughtIn(Tracking),
    Rebased(Tracking),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pushed {
    Sent(Tracking),
    AlreadyThere(Tracking),
}

struct Upstream {
    tracking: Tracking,
    reference: FullName,
}

impl Repo {
    pub(crate) fn fetch(&self) -> Result<Fetched> {
        if self.local().remote_names().is_empty() {
            return Ok(Fetched::NoRemotes);
        }
        run(
            &self.location,
            &["fetch", "--all", "--prune", "--quiet"],
            Exit::Success,
        )?;
        Ok(Fetched::AllRemotes)
    }

    pub(crate) fn pull(&self, mode: PullMode) -> Result<Pulled> {
        let Some(how) = mode.pull_args() else {
            return match self.fetch()? {
                Fetched::AllRemotes => Ok(Pulled::Fetched),
                Fetched::NoRemotes => Err(Refusal::NoRemote.into()),
            };
        };
        let repo = self.local();
        let branch = checked_out_branch(&repo)?;
        let Some(upstream) = self.upstream_of(&branch) else {
            return Err(Refusal::NoUpstream {
                branch: short_name(&branch),
            }
            .into());
        };
        let before = head_commit(&repo);
        let mut args = vec!["pull", "--quiet", "--no-edit"];
        args.extend(how);
        if let Err(error) = run(&self.location, &args, Exit::Success) {
            return Err(self.why_the_pull_stopped(error, upstream.tracking));
        }
        let tracking = upstream.tracking;
        Ok(match (head_commit(&repo) == before, mode) {
            (true, _) => Pulled::UpToDate(tracking),
            (false, PullMode::Rebase) => Pulled::Rebased(tracking),
            (false, _) => Pulled::BroughtIn(tracking),
        })
    }

    pub(crate) fn push(&self) -> Result<Pushed> {
        let repo = self.local();
        let branch = checked_out_branch(&repo)?;
        let Some(upstream) = self.upstream_of(&branch) else {
            return self.push_without_upstream(&repo, &branch);
        };
        let already_there = commit_of(&repo, &upstream.reference) == head_commit(&repo);
        run(&self.location, &["push", "--quiet"], Exit::Success)
            .map_err(|error| refused_for_missing_commits(error, &upstream.tracking))?;
        Ok(if already_there {
            Pushed::AlreadyThere(upstream.tracking)
        } else {
            Pushed::Sent(upstream.tracking)
        })
    }

    fn push_without_upstream(&self, repo: &gix::Repository, branch: &FullName) -> Result<Pushed> {
        let name = short_name(branch);
        let remote = match repo.remote_default_name(Direction::Push) {
            Some(remote) => remote.to_string(),
            None if repo.remote_names().is_empty() => return Err(Refusal::NoRemote.into()),
            None => return Err(Refusal::NoRemoteToPushTo { branch: name }.into()),
        };
        let tracking = Tracking {
            upstream: format!("{remote}/{name}"),
            branch: name,
        };
        run(
            &self.location,
            &["push", "--quiet", "--set-upstream", &remote, "HEAD"],
            Exit::Success,
        )
        .map_err(|error| refused_for_missing_commits(error, &tracking))?;
        Ok(Pushed::Sent(tracking))
    }

    fn upstream_of(&self, branch: &FullName) -> Option<Upstream> {
        let full = run_text(&self.location, &UPSTREAM_OF_HEAD, Exit::Success).ok()?;
        let reference = FullName::try_from(full.trim()).ok()?;
        Some(Upstream {
            tracking: Tracking {
                branch: short_name(branch),
                upstream: short_name(&reference),
            },
            reference,
        })
    }

    fn why_the_pull_stopped(&self, error: anyhow::Error, tracking: Tracking) -> anyhow::Error {
        let message = error.to_string();
        match self.local().state() {
            Some(InProgress::Merge) => Refusal::MergeStopped.into(),
            Some(InProgress::Rebase | InProgress::RebaseInteractive) => {
                Refusal::RebaseStopped.into()
            }
            _ if message.contains(FAST_FORWARD_IMPOSSIBLE) => Refusal::Diverged(tracking).into(),
            _ if CHANGES_IN_THE_WAY.iter().any(|why| message.contains(why)) => {
                Refusal::ChangesInTheWay.into()
            }
            _ => error,
        }
    }
}

fn checked_out_branch(repo: &gix::Repository) -> Result<FullName> {
    repo.head_name()?.ok_or_else(|| Refusal::Detached.into())
}


fn short_name(name: &FullName) -> String {
    decode(name.as_ref().shorten())
}

fn head_commit(repo: &gix::Repository) -> Option<ObjectId> {
    repo.head_id().ok().map(|id| id.detach())
}

fn commit_of(repo: &gix::Repository, name: &FullName) -> Option<ObjectId> {
    let mut reference = repo.find_reference(name.as_ref()).ok()?;
    reference.peel_to_commit().ok().map(|commit| commit.id)
}

fn refused_for_missing_commits(error: anyhow::Error, tracking: &Tracking) -> anyhow::Error {
    let message = error.to_string();
    let missing_commits = message.contains(PUSH_REJECTED)
        && REJECTED_FOR_MISSING_COMMITS
            .iter()
            .any(|reason| message.contains(reason));
    if missing_commits {
        Refusal::RemoteAhead(tracking.clone()).into()
    } else {
        error
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Fetched, Pulled, Pushed};
    use crate::git::{PullMode, Refusal, Tracking, scratch::ScratchRepo};

    struct Shared {
        origin: ScratchRepo,
        theirs: ScratchRepo,
        ours: ScratchRepo,
    }

    impl Shared {
        fn new(label: &str) -> Result<Self> {
            let origin = ScratchRepo::bare(&format!("{label}-origin"))?;
            let theirs = ScratchRepo::new(&format!("{label}-theirs"))?;
            theirs.commit("first")?;
            theirs.git(&["remote", "add", "origin", &origin.location()])?;
            theirs.git(&["push", "--quiet", "--set-upstream", "origin", "main"])?;
            let ours = ScratchRepo::tracking(&format!("{label}-ours"), &origin)?;
            Ok(Self {
                origin,
                theirs,
                ours,
            })
        }

        fn they_push(&self, message: &str) -> Result<String> {
            self.theirs.commit(message)?;
            self.theirs.git(&["push", "--quiet"])?;
            self.theirs.head()
        }

        fn both_change(&self, file: &str) -> Result<()> {
            self.theirs.write(file, "theirs\n")?;
            self.theirs.git(&["add", file])?;
            self.they_push("theirs")?;
            self.ours.write(file, "ours\n")?;
            self.ours.git(&["add", file])?;
            self.ours.commit("ours")?;
            Ok(())
        }

        fn pull(&self, mode: PullMode) -> Result<Pulled> {
            self.ours.open()?.0.pull(mode)
        }

        fn push(&self) -> Result<Pushed> {
            self.ours.open()?.0.push()
        }
    }

    fn main_tracking() -> Tracking {
        Tracking {
            branch: "main".into(),
            upstream: "origin/main".into(),
        }
    }

    fn refusal(result: Result<impl std::fmt::Debug>) -> Refusal {
        let error = result.expect_err("git should have refused");
        error
            .downcast_ref::<Refusal>()
            .cloned()
            .unwrap_or_else(|| panic!("not a refusal: {error:#}"))
    }

    fn parent_count(scratch: &ScratchRepo) -> Result<usize> {
        let line = scratch.git(&["rev-list", "--parents", "-1", "HEAD"])?;
        Ok(line.split_whitespace().count() - 1)
    }

    #[test]
    fn a_repository_without_remotes_fetches_nothing() -> Result<()> {
        let scratch = ScratchRepo::new("no-remotes")?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        assert_eq!(repo.fetch()?, Fetched::NoRemotes);
        assert_eq!(refusal(repo.pull(PullMode::FetchOnly)), Refusal::NoRemote);
        assert_eq!(refusal(repo.push()), Refusal::NoRemote);
        Ok(())
    }

    #[test]
    fn pulling_a_branch_that_is_behind_brings_the_new_commits_in() -> Result<()> {
        let shared = Shared::new("pull-behind")?;
        let theirs = shared.they_push("second")?;
        assert_eq!(
            shared.pull(PullMode::FastForwardOnly)?,
            Pulled::BroughtIn(main_tracking())
        );
        assert_eq!(shared.ours.head()?, theirs);
        assert_eq!(
            shared.pull(PullMode::FastForwardIfPossible)?,
            Pulled::UpToDate(main_tracking())
        );
        Ok(())
    }

    #[test]
    fn fast_forward_only_refuses_branches_that_have_diverged() -> Result<()> {
        let shared = Shared::new("pull-ff-only")?;
        shared.they_push("second")?;
        shared.ours.commit("mine")?;
        let before = shared.ours.head()?;
        assert_eq!(
            refusal(shared.pull(PullMode::FastForwardOnly)),
            Refusal::Diverged(main_tracking())
        );
        assert_eq!(shared.ours.head()?, before);
        Ok(())
    }

    #[test]
    fn fast_forward_if_possible_merges_branches_that_have_diverged() -> Result<()> {
        let shared = Shared::new("pull-merge")?;
        shared.they_push("second")?;
        shared.ours.commit("mine")?;
        shared.ours.git(&["config", "pull.rebase", "true"])?;
        assert_eq!(
            shared.pull(PullMode::FastForwardIfPossible)?,
            Pulled::BroughtIn(main_tracking())
        );
        assert_eq!(parent_count(&shared.ours)?, 2);
        Ok(())
    }

    #[test]
    fn rebase_replays_local_commits_on_top_of_the_upstream() -> Result<()> {
        let shared = Shared::new("pull-rebase")?;
        let theirs = shared.they_push("second")?;
        shared.ours.commit("mine")?;
        assert_eq!(
            shared.pull(PullMode::Rebase)?,
            Pulled::Rebased(main_tracking())
        );
        assert_eq!(parent_count(&shared.ours)?, 1);
        assert_eq!(shared.ours.git(&["rev-parse", "HEAD~1"])?.trim(), theirs);
        Ok(())
    }

    #[test]
    fn fetch_only_moves_the_remote_branches_and_leaves_the_branch_alone() -> Result<()> {
        let shared = Shared::new("pull-fetch")?;
        let before = shared.ours.head()?;
        let theirs = shared.they_push("second")?;
        assert_eq!(shared.pull(PullMode::FetchOnly)?, Pulled::Fetched);
        assert_eq!(shared.ours.head()?, before);
        assert_eq!(
            shared.ours.git(&["rev-parse", "origin/main"])?.trim(),
            theirs
        );
        Ok(())
    }

    #[test]
    fn a_pull_stopped_by_conflicts_says_how_to_finish_it() -> Result<()> {
        let merging = Shared::new("pull-merge-conflict")?;
        merging.both_change("shared.txt")?;
        assert_eq!(
            refusal(merging.pull(PullMode::FastForwardIfPossible)),
            Refusal::MergeStopped
        );
        let rebasing = Shared::new("pull-rebase-conflict")?;
        rebasing.both_change("shared.txt")?;
        assert_eq!(
            refusal(rebasing.pull(PullMode::Rebase)),
            Refusal::RebaseStopped
        );
        Ok(())
    }

    #[test]
    fn uncommitted_changes_in_the_way_of_a_pull_ask_for_a_commit_or_a_stash() -> Result<()> {
        let shared = Shared::new("pull-changes-in-the-way")?;
        shared.theirs.write("shared.txt", "one\n")?;
        shared.theirs.git(&["add", "shared.txt"])?;
        shared.they_push("add shared.txt")?;
        shared.pull(PullMode::FastForwardOnly)?;
        shared.theirs.write("shared.txt", "theirs\n")?;
        shared.theirs.git(&["add", "shared.txt"])?;
        shared.they_push("theirs")?;
        shared.ours.write("shared.txt", "draft\n")?;
        for mode in [PullMode::FastForwardIfPossible, PullMode::Rebase] {
            assert_eq!(
                refusal(shared.pull(mode)),
                Refusal::ChangesInTheWay,
                "{mode:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn pull_and_push_need_a_checked_out_branch() -> Result<()> {
        let shared = Shared::new("detached")?;
        shared.ours.git(&["checkout", "--quiet", "--detach"])?;
        assert_eq!(
            refusal(shared.pull(PullMode::FastForwardIfPossible)),
            Refusal::Detached
        );
        assert_eq!(refusal(shared.push()), Refusal::Detached);
        assert_eq!(shared.pull(PullMode::FetchOnly)?, Pulled::Fetched);
        Ok(())
    }

    #[test]
    fn a_branch_without_upstream_cannot_pull() -> Result<()> {
        let shared = Shared::new("pull-no-upstream")?;
        shared.ours.git(&["switch", "--quiet", "--create=topic"])?;
        assert_eq!(
            refusal(shared.pull(PullMode::FastForwardIfPossible)),
            Refusal::NoUpstream {
                branch: "topic".into()
            }
        );
        Ok(())
    }

    #[test]
    fn pushing_sends_the_new_commits_to_the_upstream() -> Result<()> {
        let shared = Shared::new("push")?;
        assert_eq!(shared.push()?, Pushed::AlreadyThere(main_tracking()));
        shared.ours.commit("mine")?;
        assert_eq!(shared.push()?, Pushed::Sent(main_tracking()));
        assert_eq!(
            shared.origin.git(&["rev-parse", "main"])?.trim(),
            shared.ours.head()?
        );
        Ok(())
    }

    #[test]
    fn the_first_push_of_a_branch_makes_its_upstream() -> Result<()> {
        let shared = Shared::new("push-new-branch")?;
        shared.ours.git(&["switch", "--quiet", "--create=topic"])?;
        shared.ours.commit("on topic")?;
        assert_eq!(
            shared.push()?,
            Pushed::Sent(Tracking {
                branch: "topic".into(),
                upstream: "origin/topic".into()
            })
        );
        assert_eq!(
            shared
                .ours
                .git(&["rev-parse", "--abbrev-ref", "topic@{upstream}"])?
                .trim(),
            "origin/topic"
        );
        assert_eq!(
            shared.origin.git(&["rev-parse", "topic"])?.trim(),
            shared.ours.head()?
        );
        Ok(())
    }

    #[test]
    fn a_push_the_remote_rejects_for_missing_commits_asks_for_a_pull() -> Result<()> {
        let shared = Shared::new("push-rejected")?;
        shared.they_push("second")?;
        shared.ours.commit("mine")?;
        assert_eq!(
            refusal(shared.push()),
            Refusal::RemoteAhead(main_tracking())
        );
        Ok(())
    }
}
