use gpui_kit::SharedString;

use crate::{
    git::{
        Behind, Carried, CaughtUp, CheckedOut, Continued, Merged, Pulled, Pushed, Rebased,
        RefLabel, Refusal, RemoteBranch, Tracking, Underway,
    },
    ui::Failure,
};

const DIVERGED_ADVICE: &str = "Choose Fast-forward if possible or Rebase in the Pull menu.";
pub(crate) const STASHED: &str = "Stashed the uncommitted changes";
pub(crate) const POPPED: &str = "Brought back the latest stash";
pub(crate) const PULL_AND_PUSH_FAILED: &str = "Couldn't pull and push";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Pull,
    Push,
    Branch,
    Stash,
    Pop,
}

impl Operation {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Operation::Pull => "Pull",
            Operation::Push => "Push",
            Operation::Branch => "Branch",
            Operation::Stash => "Stash",
            Operation::Pop => "Pop",
        }
    }

    pub(crate) fn failed(self) -> &'static str {
        match self {
            Operation::Pull => "Pull failed",
            Operation::Push => "Push failed",
            Operation::Branch => "Couldn't create the branch",
            Operation::Stash => "Stash failed",
            Operation::Pop => "Pop failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Finish {
    Continue,
    Abort,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Done(SharedString),
    Warning {
        title: SharedString,
        message: SharedString,
    },
    Stopped,
    Behind(Behind),
    Quiet,
}

impl From<SharedString> for Outcome {
    fn from(done: SharedString) -> Self {
        Outcome::Done(done)
    }
}

pub(crate) struct Operated {
    pub(crate) failed: SharedString,
    pub(crate) outcome: Result<Outcome, Failure>,
}

pub(crate) fn pulled(pulled: &Pulled) -> SharedString {
    match pulled {
        Pulled::Fetched => "Fetched every remote".into(),
        Pulled::UpToDate(Tracking { branch, upstream }) => {
            format!("{branch} is already up to date with {upstream}").into()
        }
        Pulled::BroughtIn(Tracking { branch, upstream }) => {
            format!("Pulled {upstream} into {branch}").into()
        }
        Pulled::Rebased(Tracking { branch, upstream }) => {
            format!("Rebased {branch} onto {upstream}").into()
        }
        Pulled::Stopped(Tracking { branch, upstream }) => {
            format!("Pulling {upstream} into {branch} stopped on conflicts").into()
        }
    }
}

pub(crate) fn pulled_back((pulled, carried): (Pulled, Carried)) -> Outcome {
    match &pulled {
        Pulled::Stopped(_) => Outcome::Stopped,
        Pulled::Fetched => Outcome::Done(self::pulled(&pulled)),
        Pulled::UpToDate(tracking) | Pulled::BroughtIn(tracking) | Pulled::Rebased(tracking) => {
            let place = format!("the updated {}", tracking.branch);
            put_back(self::pulled(&pulled), &place, carried)
        }
    }
}

pub(crate) fn pushed(pushed: Pushed) -> Outcome {
    let done = match pushed {
        Pushed::Behind(behind) => return Outcome::Behind(behind),
        Pushed::Sent(Tracking { branch, upstream }) => format!("Pushed {branch} to {upstream}"),
        Pushed::AlreadyThere(Tracking { branch, upstream }) => {
            format!("{upstream} already has every commit of {branch}")
        }
        Pushed::Forced(Tracking { branch, upstream }) => {
            format!("Force pushed {branch} to {upstream}")
        }
    };
    Outcome::Done(done.into())
}

pub(crate) fn caught_up(caught_up: CaughtUp) -> Outcome {
    let CaughtUp {
        pulled,
        carried,
        pushed,
    } = caught_up;
    match pushed {
        Some(Pushed::Behind(behind)) => Outcome::Behind(behind),
        Some(Pushed::Sent(Tracking { branch, upstream })) => put_back(
            format!("Pulled {upstream} into {branch}, then pushed {branch} to {upstream}").into(),
            &format!("the updated {branch}"),
            carried,
        ),
        _ => pulled_back((pulled, carried)),
    }
}

pub(crate) fn branch_created(name: &str) -> SharedString {
    format!("Switched to the new branch {name}").into()
}

pub(crate) fn checked_out(checked_out: &CheckedOut) -> SharedString {
    match checked_out {
        CheckedOut::Existing(branch) => format!("Switched to {branch}").into(),
        CheckedOut::Tracking(Tracking { branch, upstream }) => {
            format!("Switched to the new branch {branch}, tracking {upstream}").into()
        }
    }
}

pub(crate) fn switched(done: SharedString, branch: &str, carried: Carried) -> Outcome {
    put_back(done, branch, carried)
}

fn put_back(done: SharedString, place: &str, carried: Carried) -> Outcome {
    let message = match carried {
        Carried::Along => return Outcome::Done(done),
        Carried::WithConflicts => format!(
            "Your uncommitted changes conflict with {place}: resolve the conflicted files. The stash keeps a copy of them."
        ),
        Carried::OnlyInStash => {
            format!("Your uncommitted changes don't apply on {place}, so they stay in the stash.")
        }
    };
    Outcome::Warning {
        title: done,
        message: message.into(),
    }
}

pub(crate) fn merged(from: &str, into: &str, (merged, carried): (Merged, Carried)) -> Outcome {
    let done = match merged {
        Merged::Stopped => return Outcome::Stopped,
        Merged::UpToDate => format!("{into} already has every commit of {from}"),
        Merged::FastForwarded => format!("Fast-forwarded {into} to {from}"),
        Merged::WithMergeCommit => format!("Merged {from} into {into}"),
    };
    put_back(done.into(), &format!("the merged {into}"), carried)
}

pub(crate) fn rebased(branch: &str, onto: &str, (rebased, carried): (Rebased, Carried)) -> Outcome {
    let done = match rebased {
        Rebased::Stopped => return Outcome::Stopped,
        Rebased::UpToDate => format!("{branch} is already on top of {onto}"),
        Rebased::Replayed => format!("Rebased {branch} onto {onto}"),
    };
    put_back(done.into(), &format!("the rebased {branch}"), carried)
}

pub(crate) fn continued(underway: &Underway, continued: Continued) -> Outcome {
    let Continued::Finished(carried) = continued else {
        return Outcome::Stopped;
    };
    match underway {
        Underway::Rebase(rebasing) => rebased(
            &rebasing.branch,
            &rebasing.onto,
            (Rebased::Replayed, carried),
        ),
        Underway::Merge(merging) => merged(
            &merging.from,
            &merging.into,
            (Merged::WithMergeCommit, carried),
        ),
    }
}

pub(crate) fn aborted(underway: &Underway, carried: Carried) -> Outcome {
    let (what, branch) = what_is_underway(underway);
    put_back(
        format!("Aborted the {what}: {branch} is back where it was").into(),
        branch,
        carried,
    )
}

pub(crate) fn finish_failed(underway: &Underway, finish: Finish) -> SharedString {
    let (what, _) = what_is_underway(underway);
    match (underway, finish) {
        (Underway::Merge(_), Finish::Continue) => "Couldn't commit the merge".into(),
        (_, Finish::Continue) => format!("Couldn't continue the {what}").into(),
        (_, Finish::Abort) => format!("Couldn't abort the {what}").into(),
    }
}

pub(crate) fn resolve_failed(path: &str) -> SharedString {
    format!("Couldn't mark {path} resolved").into()
}

fn what_is_underway(underway: &Underway) -> (&'static str, &str) {
    match underway {
        Underway::Rebase(rebasing) => ("rebase", &rebasing.branch),
        Underway::Merge(merging) => ("merge", &merging.into),
    }
}

pub(crate) fn deleted(label: &RefLabel) -> SharedString {
    match label.on_remote() {
        Some(RemoteBranch { remote, branch }) => format!("Deleted {branch} from {remote}").into(),
        None => format!("Deleted {}", label.name).into(),
    }
}

pub(crate) fn check_out_failed(branch: &str) -> SharedString {
    format!("Couldn't check out {branch}").into()
}

pub(crate) fn merge_failed(from: &str, into: &str) -> SharedString {
    format!("Couldn't merge {from} into {into}").into()
}

pub(crate) fn rebase_failed(branch: &str, onto: &str) -> SharedString {
    format!("Couldn't rebase {branch} onto {onto}").into()
}

pub(crate) fn delete_failed(label: &RefLabel) -> SharedString {
    format!("Couldn't delete {}", label.name).into()
}

pub(crate) fn failure(error: &anyhow::Error) -> Failure {
    let failure = Failure::git_reason(error);
    match error.downcast_ref::<Refusal>() {
        Some(Refusal::Diverged(_)) => failure.followed_by(DIVERGED_ADVICE),
        _ => failure,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Finish, Outcome, aborted, caught_up, checked_out, continued, deleted, failure,
        finish_failed, merged, pulled, pulled_back, pushed, rebased, switched,
    };
    use crate::git::{
        Behind, Carried, CaughtUp, CheckedOut, Continued, Merged, Merging, Pulled, Pushed,
        Rebased, Rebasing, RefKind, RefLabel, Refusal, Tracking, Underway, fixtures::id,
    };

    fn main_tracking() -> Tracking {
        Tracking {
            branch: "main".into(),
            upstream: "origin/main".into(),
        }
    }

    #[test]
    fn each_outcome_names_the_branch_and_its_upstream() {
        assert_eq!(
            pulled(&Pulled::BroughtIn(main_tracking())),
            "Pulled origin/main into main"
        );
        assert_eq!(
            pulled(&Pulled::Rebased(main_tracking())),
            "Rebased main onto origin/main"
        );
        assert_eq!(
            pulled(&Pulled::UpToDate(main_tracking())),
            "main is already up to date with origin/main"
        );
        assert_eq!(
            pushed(Pushed::Sent(main_tracking())),
            Outcome::Done("Pushed main to origin/main".into())
        );
        assert_eq!(
            pushed(Pushed::Forced(main_tracking())),
            Outcome::Done("Force pushed main to origin/main".into())
        );
    }

    fn behind_main() -> Behind {
        Behind {
            tracking: main_tracking(),
            theirs: 2,
            ours: 1,
            checked_out: true,
            upstream_at: id(1),
        }
    }

    #[test]
    fn a_push_the_upstream_is_ahead_of_asks_instead_of_failing() {
        assert_eq!(
            pushed(Pushed::Behind(behind_main())),
            Outcome::Behind(behind_main())
        );
    }

    #[test]
    fn pull_and_push_says_both_steps_or_only_the_one_that_happened() {
        let caught = |pulled, carried, pushed| {
            caught_up(CaughtUp {
                pulled,
                carried,
                pushed,
            })
        };
        let brought_in = Pulled::BroughtIn(main_tracking());
        assert_eq!(
            caught(
                brought_in.clone(),
                Carried::Along,
                Some(Pushed::Sent(main_tracking()))
            ),
            Outcome::Done("Pulled origin/main into main, then pushed main to origin/main".into())
        );
        assert_eq!(
            caught(
                brought_in.clone(),
                Carried::Along,
                Some(Pushed::AlreadyThere(main_tracking()))
            ),
            Outcome::Done("Pulled origin/main into main".into())
        );
        assert_eq!(
            caught(Pulled::Stopped(main_tracking()), Carried::Along, None),
            Outcome::Stopped
        );
        assert_eq!(
            caught(
                brought_in.clone(),
                Carried::Along,
                Some(Pushed::Behind(behind_main()))
            ),
            Outcome::Behind(behind_main())
        );
        assert_eq!(
            caught(
                brought_in,
                Carried::OnlyInStash,
                Some(Pushed::Sent(main_tracking()))
            ),
            Outcome::Warning {
                title: "Pulled origin/main into main, then pushed main to origin/main".into(),
                message: "Your uncommitted changes don't apply on the updated main, so they stay in the stash."
                    .into()
            }
        );
    }

    #[test]
    fn a_refused_fast_forward_points_to_the_other_pull_modes() {
        let diverged = anyhow::Error::from(Refusal::Diverged(main_tracking()));
        assert_eq!(
            failure(&diverged).message(),
            "main and origin/main have diverged, so main can't fast-forward. \
             Choose Fast-forward if possible or Rebase in the Pull menu."
        );
        let git = anyhow::anyhow!("hint: see git help\nfatal: could not read from remote");
        assert_eq!(failure(&git).message(), "fatal: could not read from remote");
    }

    #[test]
    fn branch_actions_name_both_branches_in_the_order_git_moves_them() {
        let done = |text: &str| Outcome::Done(text.into());
        let merged_along = |how| merged("feature/blame", "main", (how, Carried::Along));
        assert_eq!(
            merged_along(Merged::WithMergeCommit),
            done("Merged feature/blame into main")
        );
        assert_eq!(
            merged_along(Merged::FastForwarded),
            done("Fast-forwarded main to feature/blame")
        );
        assert_eq!(
            merged_along(Merged::UpToDate),
            done("main already has every commit of feature/blame")
        );
        assert_eq!(merged_along(Merged::Stopped), Outcome::Stopped);
        assert_eq!(
            rebased("main", "feature/blame", (Rebased::Replayed, Carried::Along)),
            done("Rebased main onto feature/blame")
        );
        assert_eq!(
            checked_out(&CheckedOut::Tracking(Tracking {
                branch: "feature/diff-view".into(),
                upstream: "origin/feature/diff-view".into()
            })),
            "Switched to the new branch feature/diff-view, tracking origin/feature/diff-view"
        );
        assert_eq!(
            checked_out(&CheckedOut::Existing("feature/blame".into())),
            "Switched to feature/blame"
        );
    }

    #[test]
    fn a_check_out_that_left_changes_in_the_stash_warns_under_where_it_switched() {
        let switched_to = |carried| switched("Switched to topic".into(), "topic", carried);
        assert_eq!(
            switched_to(Carried::Along),
            Outcome::Done("Switched to topic".into())
        );
        assert_eq!(
            switched_to(Carried::WithConflicts),
            Outcome::Warning {
                title: "Switched to topic".into(),
                message:
                    "Your uncommitted changes conflict with topic: resolve the conflicted files. \
                          The stash keeps a copy of them."
                        .into()
            }
        );
        assert_eq!(
            switched_to(Carried::OnlyInStash),
            Outcome::Warning {
                title: "Switched to topic".into(),
                message:
                    "Your uncommitted changes don't apply on topic, so they stay in the stash."
                        .into()
            }
        );
    }

    #[test]
    fn changes_put_aside_for_a_rebase_or_a_pull_are_reported_when_they_dont_come_back() {
        assert_eq!(
            rebased("main", "origin/main", (Rebased::Replayed, Carried::WithConflicts)),
            Outcome::Warning {
                title: "Rebased main onto origin/main".into(),
                message: "Your uncommitted changes conflict with the rebased main: resolve the conflicted files. \
                          The stash keeps a copy of them."
                    .into()
            }
        );
        assert_eq!(
            pulled_back((Pulled::Stopped(main_tracking()), Carried::Along)),
            Outcome::Stopped
        );
        assert_eq!(
            pulled_back((Pulled::BroughtIn(main_tracking()), Carried::OnlyInStash)),
            Outcome::Warning {
                title: "Pulled origin/main into main".into(),
                message: "Your uncommitted changes don't apply on the updated main, so they stay in the stash."
                    .into()
            }
        );
    }

    #[test]
    fn finishing_a_stopped_rebase_or_merge_names_its_branches() {
        let rebase = Underway::Rebase(Rebasing {
            branch: "main".into(),
            onto: "origin/main".into(),
            step: 2,
            total: 3,
            replaying: None,
        });
        let merge = Underway::Merge(Merging {
            from: "feature/blame".into(),
            into: "main".into(),
        });
        assert_eq!(
            continued(&rebase, Continued::Finished(Carried::Along)),
            Outcome::Done("Rebased main onto origin/main".into())
        );
        assert_eq!(continued(&rebase, Continued::StoppedAgain), Outcome::Stopped);
        assert_eq!(
            continued(&merge, Continued::Finished(Carried::Along)),
            Outcome::Done("Merged feature/blame into main".into())
        );
        assert_eq!(
            aborted(&rebase, Carried::Along),
            Outcome::Done("Aborted the rebase: main is back where it was".into())
        );
        assert_eq!(finish_failed(&merge, Finish::Continue), "Couldn't commit the merge");
        assert_eq!(finish_failed(&merge, Finish::Abort), "Couldn't abort the merge");
        assert_eq!(finish_failed(&rebase, Finish::Continue), "Couldn't continue the rebase");
    }

    #[test]
    fn deleting_a_remote_branch_names_its_remote() {
        let label = |name: &str, kind| RefLabel {
            name: name.to_owned().into(),
            kind,
            target: id(1),
            is_head: false,
            upstream: None,
        };
        assert_eq!(
            deleted(&label("origin/feature/x", RefKind::RemoteBranch)),
            "Deleted feature/x from origin"
        );
        assert_eq!(
            deleted(&label("feature/x", RefKind::LocalBranch)),
            "Deleted feature/x"
        );
    }
}
