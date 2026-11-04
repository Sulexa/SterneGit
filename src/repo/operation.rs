use gpui_kit::SharedString;

use crate::{
    git::{
        Carried, CheckedOut, Merged, Pulled, Pushed, Rebased, RefLabel, Refusal, RemoteBranch,
        Tracking,
    },
    ui::Failure,
};

const DIVERGED_ADVICE: &str = "Choose Fast-forward if possible or Rebase in the Pull menu.";
pub(crate) const STASHED: &str = "Stashed the uncommitted changes";
pub(crate) const POPPED: &str = "Brought back the latest stash";

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Done(SharedString),
    Warning {
        title: SharedString,
        message: SharedString,
    },
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
    }
}

pub(crate) fn pushed(pushed: &Pushed) -> SharedString {
    match pushed {
        Pushed::Sent(Tracking { branch, upstream }) => {
            format!("Pushed {branch} to {upstream}").into()
        }
        Pushed::AlreadyThere(Tracking { branch, upstream }) => {
            format!("{upstream} already has every commit of {branch}").into()
        }
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
    let message = match carried {
        Carried::Along => return Outcome::Done(done),
        Carried::WithConflicts => format!(
            "Your uncommitted changes conflict with {branch}: resolve the conflicted files. The stash keeps a copy of them."
        ),
        Carried::OnlyInStash => {
            format!("Your uncommitted changes don't apply on {branch}, so they stay in the stash.")
        }
    };
    Outcome::Warning {
        title: done,
        message: message.into(),
    }
}

pub(crate) fn merged(from: &str, into: &str, merged: Merged) -> SharedString {
    match merged {
        Merged::UpToDate => format!("{into} already has every commit of {from}").into(),
        Merged::FastForwarded => format!("Fast-forwarded {into} to {from}").into(),
        Merged::WithMergeCommit => format!("Merged {from} into {into}").into(),
    }
}

pub(crate) fn rebased(branch: &str, onto: &str, rebased: Rebased) -> SharedString {
    match rebased {
        Rebased::UpToDate => format!("{branch} is already on top of {onto}").into(),
        Rebased::Replayed => format!("Rebased {branch} onto {onto}").into(),
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
        Outcome, checked_out, deleted, failure, merged, pulled, pushed, rebased, switched,
    };
    use crate::git::{
        Carried, CheckedOut, Merged, Pulled, Pushed, Rebased, RefKind, RefLabel, Refusal, Tracking,
        fixtures::id,
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
            pushed(&Pushed::Sent(main_tracking())),
            "Pushed main to origin/main"
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
        assert_eq!(
            merged("feature/blame", "main", Merged::WithMergeCommit),
            "Merged feature/blame into main"
        );
        assert_eq!(
            merged("feature/blame", "main", Merged::FastForwarded),
            "Fast-forwarded main to feature/blame"
        );
        assert_eq!(
            merged("feature/blame", "main", Merged::UpToDate),
            "main already has every commit of feature/blame"
        );
        assert_eq!(
            rebased("main", "feature/blame", Rebased::Replayed),
            "Rebased main onto feature/blame"
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
    fn deleting_a_remote_branch_names_its_remote() {
        let label = |name: &str, kind| RefLabel {
            name: name.to_owned().into(),
            kind,
            target: id(1),
            is_head: false,
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
