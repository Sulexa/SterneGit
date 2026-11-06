use std::ffi::OsStr;

use anyhow::Result;
use gix::{ObjectId, refs::FullName, remote::Direction};

use super::{
    Carried, PullMode, Refusal, Repo, Tracking,
    command::{Exit, run_text, run_with_access},
    merge::{AUTOSTASH, Ran},
    text::decode,
};

const UPSTREAM_OF_HEAD: [&str; 3] = ["rev-parse", "--symbolic-full-name", "@{upstream}"];
const UPSTREAM_FIELDS: &str =
    "--format=%(refname)%09%(upstream)%09%(upstream:remotename)%09%(upstream:remoteref)";
const FIELD_SEPARATOR: char = '\t';
const BRANCHES: &str = "refs/heads/";
const HEAD: &str = "HEAD";
const FAST_FORWARD_IMPOSSIBLE: &str = "Not possible to fast-forward";
const PUSH_REJECTED: &str = "[rejected]";
const REJECTED_FOR_MISSING_COMMITS: [&str; 2] = ["(fetch first)", "(non-fast-forward)"];
const REJECTED_FOR_A_MOVED_LEASE: &str = "(stale info)";
const FETCH_REFUSED_TO_MOVE_BACK: &str = "(non-fast-forward)";
const COUNT_EACH_SIDE: [&str; 3] = ["rev-list", "--left-right", "--count"];
const REMOTE_URL_KEYS: &str = r"^remote\..*url$";

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
    Stopped(Tracking),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Pushed {
    Sent(Tracking),
    AlreadyThere(Tracking),
    Forced(Tracking),
    Behind(Behind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Behind {
    pub(crate) tracking: Tracking,
    pub(crate) theirs: usize,
    pub(crate) ours: usize,
    pub(crate) checked_out: bool,
    pub(crate) upstream_at: ObjectId,
}

impl Behind {
    pub(crate) fn can_pull(&self) -> bool {
        self.checked_out || self.ours == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaughtUp {
    pub(crate) pulled: Pulled,
    pub(crate) carried: Carried,
    pub(crate) pushed: Option<Pushed>,
}

struct Upstream {
    tracking: Tracking,
    reference: FullName,
}

struct ConfiguredUpstream {
    upstream: Upstream,
    remote: String,
    merge: String,
}

impl Repo {
    pub(crate) fn fetch(&self) -> Result<Fetched> {
        if self.local().remote_names().is_empty() {
            return Ok(Fetched::NoRemotes);
        }
        self.reach(&["fetch", "--all", "--prune", "--quiet"])?;
        Ok(Fetched::AllRemotes)
    }

    pub(crate) fn pull(&self, mode: PullMode) -> Result<(Pulled, Carried)> {
        let Some(how) = mode.pull_args() else {
            return match self.fetch()? {
                Fetched::AllRemotes => Ok((Pulled::Fetched, Carried::Along)),
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
        let stash = self.latest_stash();
        let mut args = vec!["pull", "--quiet", "--no-edit", AUTOSTASH];
        args.extend(how);
        let tracking = upstream.tracking;
        match self.run_until_stopped(&args) {
            Ok(Ran::Stopped) => return Ok((Pulled::Stopped(tracking), Carried::Along)),
            Ok(Ran::Through) => {}
            Err(error) => return Err(diverged_or(error, tracking)),
        }
        let pulled = match (head_commit(&repo) == before, mode) {
            (true, _) => Pulled::UpToDate(tracking),
            (false, PullMode::Rebase) => Pulled::Rebased(tracking),
            (false, _) => Pulled::BroughtIn(tracking),
        };
        Ok((pulled, self.carried_since(stash)))
    }

    pub(crate) fn push(&self) -> Result<Pushed> {
        let repo = self.local();
        let branch = checked_out_branch(&repo)?;
        let Some(upstream) = self.upstream_of(&branch) else {
            return self.push_without_upstream(&repo, &branch, HEAD);
        };
        let already_there = commit_of(&repo, &upstream.reference) == head_commit(&repo);
        if let Err(error) = self.reach(&["push", "--quiet"]) {
            return self.behind_or(error, &upstream.tracking.branch);
        }
        Ok(if already_there {
            Pushed::AlreadyThere(upstream.tracking)
        } else {
            Pushed::Sent(upstream.tracking)
        })
    }

    pub(crate) fn pull_branch(&self, branch: &str) -> Result<Pulled> {
        let Some(configured) = self.configured_upstream(branch)? else {
            return Err(Refusal::NoUpstream {
                branch: branch.to_owned(),
            }
            .into());
        };
        let repo = self.local();
        let local = branch_reference(branch)?;
        let before = commit_of(&repo, &local);
        let refspec = format!("{}:{}", configured.merge, local.as_bstr());
        let tracking = configured.upstream.tracking;
        if let Err(error) = self.reach(&fetch_that_reports_rejections(&configured.remote, &refspec)) {
            return Err(if error.to_string().contains(FETCH_REFUSED_TO_MOVE_BACK) {
                Refusal::CantFastForward(tracking).into()
            } else {
                error
            });
        }
        Ok(if commit_of(&repo, &local) == before {
            Pulled::UpToDate(tracking)
        } else {
            Pulled::BroughtIn(tracking)
        })
    }

    pub(crate) fn push_branch(&self, branch: &str) -> Result<Pushed> {
        let repo = self.local();
        let local = branch_reference(branch)?;
        let Some(configured) = self.configured_upstream(branch)? else {
            let same_name_there = format!("{0}:{0}", local.as_bstr());
            return self.push_without_upstream(&repo, &local, &same_name_there);
        };
        let upstream = configured.upstream;
        let already_there = commit_of(&repo, &upstream.reference) == commit_of(&repo, &local);
        let refspec = format!("{}:{}", local.as_bstr(), configured.merge);
        if let Err(error) = self.reach(&["push", "--quiet", &configured.remote, &refspec]) {
            return self.behind_or(error, branch);
        }
        Ok(if already_there {
            Pushed::AlreadyThere(upstream.tracking)
        } else {
            Pushed::Sent(upstream.tracking)
        })
    }

    pub(crate) fn force_push(&self, behind: &Behind) -> Result<Pushed> {
        let branch = &behind.tracking.branch;
        let Some(configured) = self.configured_upstream(branch)? else {
            return Err(Refusal::NoUpstream {
                branch: branch.clone(),
            }
            .into());
        };
        let lease = format!("--force-with-lease={}:{}", configured.merge, behind.upstream_at);
        let refspec = format!("{}:{}", branch_reference(branch)?.as_bstr(), configured.merge);
        let tracking = configured.upstream.tracking;
        match self.reach(&["push", "--quiet", &lease, &configured.remote, &refspec]) {
            Ok(_) => Ok(Pushed::Forced(tracking)),
            Err(error) if error.to_string().contains(REJECTED_FOR_A_MOVED_LEASE) => {
                Err(Refusal::UpstreamMoved(tracking).into())
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn pull_then_push(&self, behind: &Behind) -> Result<CaughtUp> {
        let branch = &behind.tracking.branch;
        let on_it = self.local().head_name()? == Some(branch_reference(branch)?);
        if !on_it {
            let pulled = self.pull_branch(branch)?;
            let pushed = self.push_branch(branch)?;
            return Ok(CaughtUp {
                pulled,
                carried: Carried::Along,
                pushed: Some(pushed),
            });
        }
        let (pulled, carried) = self.pull(PullMode::FastForwardIfPossible)?;
        let pushed = match pulled {
            Pulled::Stopped(_) => None,
            _ => Some(self.push()?),
        };
        Ok(CaughtUp {
            pulled,
            carried,
            pushed,
        })
    }

    fn behind_or(&self, error: anyhow::Error, branch: &str) -> Result<Pushed> {
        if !rejected_for_missing_commits(&error) {
            return Err(error);
        }
        let Some(configured) = self.configured_upstream(branch)? else {
            return Err(error);
        };
        self.reach(&["fetch", "--quiet", &configured.remote])?;
        let repo = self.local();
        let local = branch_reference(branch)?;
        let upstream = configured.upstream;
        let Some(upstream_at) = commit_of(&repo, &upstream.reference) else {
            return Err(error);
        };
        let range = format!("{}...{}", local.as_bstr(), upstream.reference.as_bstr());
        let mut args = COUNT_EACH_SIDE.to_vec();
        args.push(&range);
        let counted = run_text(&self.location, &args, Exit::Success)?;
        let Some((ours, theirs)) = each_side(&counted).filter(|&(_, theirs)| theirs > 0) else {
            return Err(error);
        };
        Ok(Pushed::Behind(Behind {
            tracking: upstream.tracking,
            theirs,
            ours,
            checked_out: repo.head_name()? == Some(local),
            upstream_at,
        }))
    }

    fn configured_upstream(&self, branch: &str) -> Result<Option<ConfiguredUpstream>> {
        let local = branch_reference(branch)?;
        let local_name = local.as_bstr().to_string();
        let listed = run_text(
            &self.location,
            &["for-each-ref", UPSTREAM_FIELDS, &local_name],
            Exit::Success,
        )?;
        let fields = listed.lines().find_map(|line| {
            let mut fields = line.split(FIELD_SEPARATOR);
            (fields.next()? == local_name).then_some(())?;
            let reference = FullName::try_from(fields.next()?).ok()?;
            Some((reference, fields.next()?, fields.next()?))
        });
        Ok(fields.map(|(reference, remote, merge)| ConfiguredUpstream {
            upstream: Upstream {
                tracking: Tracking {
                    branch: branch.to_owned(),
                    upstream: short_name(&reference),
                },
                reference,
            },
            remote: remote.to_owned(),
            merge: merge.to_owned(),
        }))
    }

    fn push_without_upstream(
        &self,
        repo: &gix::Repository,
        branch: &FullName,
        refspec: &str,
    ) -> Result<Pushed> {
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
        self.reach(&["push", "--quiet", "--set-upstream", &remote, refspec])
            .map_err(|error| {
                if rejected_for_missing_commits(&error) {
                    Refusal::RemoteAhead(tracking.clone()).into()
                } else {
                    error
                }
            })?;
        Ok(Pushed::Sent(tracking))
    }

    pub(crate) fn has_remotes(&self) -> bool {
        !self.local().remote_names().is_empty()
    }

    pub(crate) fn remote_urls(&self) -> Vec<String> {
        run_text(
            &self.location,
            &["config", "--get-regexp", REMOTE_URL_KEYS],
            Exit::SuccessOrDifference,
        )
        .map(|listed| {
            listed
                .lines()
                .filter_map(|line| line.split_once(' '))
                .map(|(_, url)| url.trim().to_owned())
                .collect()
        })
        .unwrap_or_default()
    }

    pub(super) fn reach<A: AsRef<OsStr>>(&self, args: &[A]) -> Result<Vec<u8>> {
        run_with_access(&self.location, &self.access, args, Exit::Success)
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
}

fn diverged_or(error: anyhow::Error, tracking: Tracking) -> anyhow::Error {
    if error.to_string().contains(FAST_FORWARD_IMPOSSIBLE) {
        Refusal::Diverged(tracking).into()
    } else {
        error
    }
}

fn fetch_that_reports_rejections<'a>(remote: &'a str, refspec: &'a str) -> [&'a str; 3] {
    ["fetch", remote, refspec]
}

pub(super) fn checked_out_branch(repo: &gix::Repository) -> Result<FullName> {
    repo.head_name()?.ok_or_else(|| Refusal::Detached.into())
}

pub(super) fn branch_reference(branch: &str) -> Result<FullName> {
    Ok(FullName::try_from(format!("{BRANCHES}{branch}"))?)
}

pub(super) fn short_name(name: &FullName) -> String {
    decode(name.as_ref().shorten())
}

pub(super) fn head_commit(repo: &gix::Repository) -> Option<ObjectId> {
    repo.head_id().ok().map(|id| id.detach())
}

pub(super) fn commit_of(repo: &gix::Repository, name: &FullName) -> Option<ObjectId> {
    let mut reference = repo.find_reference(name.as_ref()).ok()?;
    reference.peel_to_commit().ok().map(|commit| commit.id)
}

fn rejected_for_missing_commits(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains(PUSH_REJECTED)
        && REJECTED_FOR_MISSING_COMMITS
            .iter()
            .any(|reason| message.contains(reason))
}

fn each_side(counted: &str) -> Option<(usize, usize)> {
    let mut sides = counted.split_whitespace().map(str::parse);
    match (sides.next(), sides.next()) {
        (Some(Ok(ours)), Some(Ok(theirs))) => Some((ours, theirs)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Behind, CaughtUp, Fetched, Pulled, Pushed};
    use crate::git::{Carried, PullMode, Refusal, Tracking, scratch::ScratchRepo};

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
            self.pull_carrying(mode).map(|(pulled, _)| pulled)
        }

        fn pull_carrying(&self, mode: PullMode) -> Result<(Pulled, Carried)> {
            self.ours.open()?.0.pull(mode)
        }

        fn push(&self) -> Result<Pushed> {
            self.ours.open()?.0.push()
        }

        fn behind(&self) -> Result<Behind> {
            match self.push()? {
                Pushed::Behind(behind) => Ok(behind),
                pushed => panic!("the push should have been refused, got {pushed:?}"),
            }
        }

        fn upstream_head(&self, branch: &str) -> Result<String> {
            Ok(self.origin.git(&["rev-parse", branch])?.trim().to_owned())
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
    fn a_pull_that_conflicts_stops_for_the_conflicts_to_be_resolved() -> Result<()> {
        let merging = Shared::new("pull-merge-conflict")?;
        merging.both_change("shared.txt")?;
        assert_eq!(
            merging.pull(PullMode::FastForwardIfPossible)?,
            Pulled::Stopped(main_tracking())
        );
        let rebasing = Shared::new("pull-rebase-conflict")?;
        rebasing.both_change("shared.txt")?;
        assert_eq!(
            rebasing.pull(PullMode::Rebase)?,
            Pulled::Stopped(main_tracking())
        );
        assert!(rebasing.ours.open()?.0.underway().is_some());
        Ok(())
    }

    #[test]
    fn uncommitted_changes_are_put_aside_for_a_pull_and_brought_back() -> Result<()> {
        for mode in [PullMode::FastForwardIfPossible, PullMode::Rebase] {
            let shared = Shared::new(&format!("pull-autostash-{mode:?}"))?;
            shared.theirs.write("shared.txt", "one\n")?;
            shared.theirs.git(&["add", "shared.txt"])?;
            shared.they_push("add shared.txt")?;
            shared.pull(PullMode::FastForwardOnly)?;
            shared.theirs.write("shared.txt", "theirs\n")?;
            shared.theirs.git(&["add", "shared.txt"])?;
            shared.they_push("theirs")?;
            shared.ours.write("other.txt", "draft\n")?;
            shared.ours.git(&["add", "other.txt"])?;
            shared.ours.commit("Add other.txt")?;
            shared.ours.write("other.txt", "more\n")?;
            assert_eq!(shared.pull_carrying(mode)?.1, Carried::Along, "{mode:?}");
            shared.theirs.write("shared.txt", "theirs again\n")?;
            shared.theirs.git(&["add", "shared.txt"])?;
            shared.they_push("again")?;
            shared.ours.write("shared.txt", "draft\n")?;
            assert_eq!(
                shared.pull_carrying(mode)?.1,
                Carried::WithConflicts,
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
    fn a_push_the_remote_rejects_fetches_and_counts_both_sides() -> Result<()> {
        let shared = Shared::new("push-rejected")?;
        let theirs = shared.they_push("second")?;
        shared.they_push("third")?;
        shared.ours.commit("mine")?;
        let behind = shared.behind()?;
        assert_eq!(
            (behind.tracking.clone(), behind.theirs, behind.ours, behind.checked_out),
            (main_tracking(), 2, 1, true)
        );
        assert_eq!(behind.upstream_at.to_string(), shared.theirs.head()?);
        assert_eq!(
            shared.ours.git(&["rev-parse", "origin/main~1"])?.trim(),
            theirs
        );
        Ok(())
    }

    #[test]
    fn pull_and_push_merges_the_upstream_in_then_sends_the_merge() -> Result<()> {
        let shared = Shared::new("push-pull-merge")?;
        shared.they_push("second")?;
        shared.ours.commit("mine")?;
        shared.ours.git(&["config", "pull.rebase", "true"])?;
        let behind = shared.behind()?;
        let caught_up = shared.ours.open()?.0.pull_then_push(&behind)?;
        assert_eq!(
            caught_up,
            CaughtUp {
                pulled: Pulled::BroughtIn(main_tracking()),
                carried: Carried::Along,
                pushed: Some(Pushed::Sent(main_tracking())),
            }
        );
        assert_eq!(parent_count(&shared.ours)?, 2);
        assert_eq!(shared.upstream_head("main")?, shared.ours.head()?);
        Ok(())
    }

    #[test]
    fn with_nothing_of_its_own_pull_and_push_only_fast_forwards() -> Result<()> {
        let shared = Shared::new("push-only-behind")?;
        let theirs = shared.they_push("second")?;
        let behind = shared.behind()?;
        assert_eq!((behind.theirs, behind.ours), (1, 0));
        let caught_up = shared.ours.open()?.0.pull_then_push(&behind)?;
        assert_eq!(caught_up.pulled, Pulled::BroughtIn(main_tracking()));
        assert_eq!(
            caught_up.pushed,
            Some(Pushed::AlreadyThere(main_tracking()))
        );
        assert_eq!(shared.ours.head()?, theirs);
        Ok(())
    }

    #[test]
    fn a_pull_that_stops_on_conflicts_pushes_nothing() -> Result<()> {
        let shared = Shared::new("push-pull-conflict")?;
        shared.both_change("shared.txt")?;
        let theirs = shared.theirs.head()?;
        let behind = shared.behind()?;
        let caught_up = shared.ours.open()?.0.pull_then_push(&behind)?;
        assert_eq!(caught_up.pulled, Pulled::Stopped(main_tracking()));
        assert_eq!(caught_up.pushed, None);
        assert_eq!(shared.upstream_head("main")?, theirs);
        Ok(())
    }

    #[test]
    fn force_push_overwrites_the_upstream_only_where_it_was_counted() -> Result<()> {
        let shared = Shared::new("push-force")?;
        shared.they_push("second")?;
        shared.ours.commit("mine")?;
        let behind = shared.behind()?;
        let moved_again = shared.they_push("third")?;
        let (repo, _) = shared.ours.open()?;
        assert_eq!(
            refusal(repo.force_push(&behind)),
            Refusal::UpstreamMoved(main_tracking())
        );
        assert_eq!(shared.upstream_head("main")?, moved_again);
        let behind = shared.behind()?;
        assert_eq!((behind.theirs, behind.ours), (2, 1));
        assert_eq!(repo.force_push(&behind)?, Pushed::Forced(main_tracking()));
        assert_eq!(shared.upstream_head("main")?, shared.ours.head()?);
        Ok(())
    }

    #[test]
    fn a_branch_that_is_not_checked_out_asks_too_and_can_be_forced() -> Result<()> {
        let shared = with_topic_on_both_sides("push-other-behind")?;
        shared.they_push("theirs on topic")?;
        shared.ours.git(&["switch", "--quiet", "topic"])?;
        shared.ours.commit("mine on topic")?;
        let mine = shared.ours.head()?;
        shared.ours.git(&["switch", "--quiet", "main"])?;
        let (repo, _) = shared.ours.open()?;
        let Pushed::Behind(behind) = repo.push_branch("topic")? else {
            panic!("the push of topic should have been refused");
        };
        assert_eq!(
            (behind.tracking.clone(), behind.theirs, behind.ours, behind.checked_out),
            (topic_tracking(), 1, 1, false)
        );
        assert!(!behind.can_pull());
        assert_eq!(repo.force_push(&behind)?, Pushed::Forced(topic_tracking()));
        assert_eq!(shared.upstream_head("topic")?, mine);
        Ok(())
    }

    #[test]
    fn a_branch_that_is_not_checked_out_and_only_behind_pulls_by_fast_forward() -> Result<()> {
        let shared = with_topic_on_both_sides("push-other-only-behind")?;
        let theirs = shared.they_push("theirs on topic")?;
        let (repo, _) = shared.ours.open()?;
        let Pushed::Behind(behind) = repo.push_branch("topic")? else {
            panic!("the push of topic should have been refused");
        };
        assert!(behind.can_pull());
        let caught_up = repo.pull_then_push(&behind)?;
        assert_eq!(caught_up.pulled, Pulled::BroughtIn(topic_tracking()));
        assert_eq!(
            caught_up.pushed,
            Some(Pushed::AlreadyThere(topic_tracking()))
        );
        assert_eq!(shared.ours.git(&["rev-parse", "topic"])?.trim(), theirs);
        Ok(())
    }

    fn topic_tracking() -> Tracking {
        Tracking {
            branch: "topic".into(),
            upstream: "origin/topic".into(),
        }
    }

    fn with_topic_on_both_sides(label: &str) -> Result<Shared> {
        let shared = Shared::new(label)?;
        shared.theirs.git(&["switch", "--quiet", "--create=topic"])?;
        shared
            .theirs
            .git(&["push", "--quiet", "--set-upstream", "origin", "topic"])?;
        shared.ours.git(&["fetch", "--quiet", "origin"])?;
        shared
            .ours
            .git(&["branch", "--quiet", "--track", "topic", "origin/topic"])?;
        Ok(shared)
    }

    #[test]
    fn pulling_a_branch_that_is_not_checked_out_fast_forwards_it() -> Result<()> {
        let shared = with_topic_on_both_sides("pull-other")?;
        let theirs = shared.they_push("on topic")?;
        let before = shared.ours.head()?;
        let (repo, _) = shared.ours.open()?;
        assert_eq!(
            repo.pull_branch("topic")?,
            Pulled::BroughtIn(topic_tracking())
        );
        assert_eq!(shared.ours.git(&["rev-parse", "topic"])?.trim(), theirs);
        assert_eq!(shared.ours.head()?, before);
        assert_eq!(
            repo.pull_branch("topic")?,
            Pulled::UpToDate(topic_tracking())
        );
        Ok(())
    }

    #[test]
    fn a_branch_that_is_not_checked_out_only_fast_forwards() -> Result<()> {
        let shared = with_topic_on_both_sides("pull-other-diverged")?;
        shared.they_push("theirs")?;
        shared.ours.git(&["switch", "--quiet", "topic"])?;
        shared.ours.commit("mine")?;
        let mine = shared.ours.head()?;
        shared.ours.git(&["switch", "--quiet", "main"])?;
        let (repo, _) = shared.ours.open()?;
        assert_eq!(
            refusal(repo.pull_branch("topic")),
            Refusal::CantFastForward(topic_tracking())
        );
        assert_eq!(shared.ours.git(&["rev-parse", "topic"])?.trim(), mine);
        shared.ours.git(&["branch", "lonely"])?;
        assert_eq!(
            refusal(repo.pull_branch("lonely")),
            Refusal::NoUpstream {
                branch: "lonely".into()
            }
        );
        Ok(())
    }

    #[test]
    fn pushing_a_branch_that_is_not_checked_out_makes_its_upstream_then_updates_it()
    -> Result<()> {
        let shared = Shared::new("push-other")?;
        shared.ours.git(&["switch", "--quiet", "--create=topic"])?;
        shared.ours.commit("on topic")?;
        shared.ours.git(&["switch", "--quiet", "main"])?;
        let (repo, _) = shared.ours.open()?;
        assert_eq!(repo.push_branch("topic")?, Pushed::Sent(topic_tracking()));
        assert_eq!(
            shared
                .ours
                .git(&["rev-parse", "--abbrev-ref", "topic@{upstream}"])?
                .trim(),
            "origin/topic"
        );
        assert_eq!(
            repo.push_branch("topic")?,
            Pushed::AlreadyThere(topic_tracking())
        );
        shared.ours.git(&["switch", "--quiet", "topic"])?;
        shared.ours.commit("newer")?;
        shared.ours.git(&["switch", "--quiet", "main"])?;
        assert_eq!(repo.push_branch("topic")?, Pushed::Sent(topic_tracking()));
        assert_eq!(
            shared.origin.git(&["rev-parse", "topic"])?.trim(),
            shared.ours.git(&["rev-parse", "topic"])?.trim()
        );
        Ok(())
    }
}
