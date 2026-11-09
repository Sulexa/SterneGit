use std::fmt;

const CHANGES_IN_THE_WAY: [&str; 3] = [
    "You have unstaged changes",
    "Your index contains uncommitted changes",
    "would be overwritten by",
];

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
    CantFastForward(Tracking),
    RemoteAhead(Tracking),
    UpstreamMoved(Tracking),
    ChangesInTheWay,
    StashKept,
    StashNotApplied,
    StashGone,
    TagTakenThere { tag: String, remote: String },
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
                "{branch} has no upstream branch, and with no origin among the remotes SterneGit can't tell where to push it."
            ),
            Refusal::Diverged(Tracking { branch, upstream }) => write!(
                f,
                "{branch} and {upstream} have diverged, so {branch} can't fast-forward."
            ),
            Refusal::CantFastForward(Tracking { branch, upstream }) => write!(
                f,
                "{branch} and {upstream} have diverged, so {branch} can't fast-forward. Check it out to merge or rebase."
            ),
            Refusal::RemoteAhead(Tracking { branch, upstream }) => write!(
                f,
                "{upstream} has commits that {branch} doesn't have yet. Pull them, then push again."
            ),
            Refusal::UpstreamMoved(Tracking { upstream, .. }) => write!(
                f,
                "{upstream} moved again since SterneGit fetched it, so nothing was overwritten. Push again to see what it has now."
            ),
            Refusal::ChangesInTheWay => write!(
                f,
                "Uncommitted changes are in the way: commit or stash them, then try again."
            ),
            Refusal::StashKept => write!(
                f,
                "The stash didn't apply cleanly: resolve the conflicted files. The stash is kept, so nothing is lost."
            ),
            Refusal::StashNotApplied => write!(
                f,
                "The stash doesn't apply on top of the current files, so nothing changed. The stash is kept, so nothing is lost."
            ),
            Refusal::StashGone => write!(
                f,
                "That stash isn't there anymore: it was popped or deleted outside SterneGit."
            ),
            Refusal::TagTakenThere { tag, remote } => write!(
                f,
                "{remote} already has a different {tag}. Delete {tag} from {remote} first, then push it again."
            ),
        }
    }
}

impl std::error::Error for Refusal {}

pub(super) fn why_git_stopped(error: anyhow::Error) -> anyhow::Error {
    let message = error.to_string();
    if CHANGES_IN_THE_WAY.iter().any(|why| message.contains(why)) {
        Refusal::ChangesInTheWay.into()
    } else {
        error
    }
}
