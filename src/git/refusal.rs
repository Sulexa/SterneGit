use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tracking {
    pub(crate) branch: String,
    pub(crate) upstream: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    Detached,
    NoUpstream { branch: String },
    NoRemote,
    NoRemoteToPushTo { branch: String },
    Diverged(Tracking),
    RemoteAhead(Tracking),
    ChangesInTheWay,
    MergeStopped,
    RebaseStopped,
    StashKept,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::Detached => write!(f, "HEAD is detached: check out a branch first."),
            Refusal::NoUpstream { branch } => {
                write!(f, "{branch} has no upstream branch to pull from.")
            }
            Refusal::NoRemote => write!(f, "This repository has no remote yet."),
            Refusal::NoRemoteToPushTo { branch } => write!(
                f,
                "{branch} has no upstream branch, and with no origin among the remotes FastGit can't tell where to push it."
            ),
            Refusal::Diverged(Tracking { branch, upstream }) => write!(
                f,
                "{branch} and {upstream} have diverged, so {branch} can't fast-forward."
            ),
            Refusal::RemoteAhead(Tracking { branch, upstream }) => write!(
                f,
                "{upstream} has commits that {branch} doesn't have yet. Pull them, then push again."
            ),
            Refusal::ChangesInTheWay => write!(
                f,
                "Uncommitted changes are in the way: commit or stash them, then pull again."
            ),
            Refusal::MergeStopped => write!(
                f,
                "The merge stopped on conflicts: resolve the conflicted files, stage them and commit."
            ),
            Refusal::RebaseStopped => write!(
                f,
                "The rebase stopped on conflicts: resolve them, then run git rebase --continue, or git rebase --abort to undo it."
            ),
            Refusal::StashKept => write!(
                f,
                "The stash didn't apply cleanly: resolve the conflicted files. The stash is kept, so nothing is lost."
            ),
        }
    }
}

impl std::error::Error for Refusal {}
