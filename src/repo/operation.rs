use gpui_kit::SharedString;

use crate::{
    git::{Pulled, Pushed, Refusal, Tracking},
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

pub(crate) struct Operated {
    pub(crate) operation: Operation,
    pub(crate) outcome: Result<SharedString, Failure>,
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

pub(crate) fn failure(error: &anyhow::Error) -> Failure {
    let failure = Failure::git_reason(error);
    match error.downcast_ref::<Refusal>() {
        Some(Refusal::Diverged(_)) => failure.followed_by(DIVERGED_ADVICE),
        _ => failure,
    }
}

#[cfg(test)]
mod tests {
    use super::{failure, pulled, pushed};
    use crate::git::{Pulled, Pushed, Refusal, Tracking};

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
}
