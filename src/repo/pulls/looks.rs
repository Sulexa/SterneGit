use sternegit_protocol::{
    Check, CheckState, MergeMethod, Mergeability, PullDetails, PullState, PullSummary,
    ReviewStanding,
};
use gpui_kit::{App, Hsla, assets::IconName, component::ActiveTheme as _};

use crate::ui::thousands;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tint {
    Green,
    Yellow,
    Red,
    Magenta,
    Muted,
}

impl Tint {
    pub(crate) fn color(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Tint::Green => theme.green,
            Tint::Yellow => theme.yellow,
            Tint::Red => theme.red,
            Tint::Magenta => theme.magenta,
            Tint::Muted => theme.muted_foreground,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) icon: IconName,
    pub(crate) tint: Tint,
    pub(crate) label: &'static str,
    pub(crate) spins: bool,
}

const fn mark(icon: IconName, tint: Tint, label: &'static str) -> Mark {
    Mark {
        icon,
        tint,
        label,
        spins: false,
    }
}

pub(crate) fn state_mark(state: PullState, draft: bool) -> Mark {
    match (state, draft) {
        (PullState::Open, true) => mark(IconName::GitPullRequestDraft, Tint::Muted, "Draft"),
        (PullState::Open, false) => mark(IconName::GitPullRequest, Tint::Green, "Open"),
        (PullState::Merged, _) => mark(IconName::GitMerge, Tint::Magenta, "Merged"),
        (PullState::Closed, _) => mark(IconName::GitPullRequestClosed, Tint::Red, "Closed"),
        (PullState::Unknown, _) => mark(IconName::GitPullRequest, Tint::Muted, "Unknown"),
    }
}

pub(crate) fn review_mark(review: ReviewStanding) -> Option<Mark> {
    match review {
        ReviewStanding::Approved => Some(mark(IconName::CircleCheck, Tint::Green, "Approved")),
        ReviewStanding::ChangesRequested => {
            Some(mark(IconName::CircleAlert, Tint::Red, "Changes requested"))
        }
        ReviewStanding::ReviewRequired => Some(mark(IconName::Eye, Tint::Yellow, "Review required")),
        ReviewStanding::Unknown => None,
    }
}

const WORST_FIRST: [CheckState; 9] = [
    CheckState::Failure,
    CheckState::ActionRequired,
    CheckState::Running,
    CheckState::Pending,
    CheckState::Success,
    CheckState::Neutral,
    CheckState::Skipped,
    CheckState::Cancelled,
    CheckState::Unknown,
];

pub(crate) fn check_mark(state: CheckState) -> Mark {
    match state {
        CheckState::Failure => mark(IconName::CircleX, Tint::Red, "failing"),
        CheckState::ActionRequired => mark(IconName::CircleAlert, Tint::Yellow, "waiting for you"),
        CheckState::Running => Mark {
            spins: true,
            ..mark(IconName::Loader, Tint::Yellow, "running")
        },
        CheckState::Pending => mark(IconName::Clock, Tint::Yellow, "waiting"),
        CheckState::Success => mark(IconName::CircleCheck, Tint::Green, "passed"),
        CheckState::Neutral => mark(IconName::CircleMinus, Tint::Muted, "neutral"),
        CheckState::Skipped => mark(IconName::CircleMinus, Tint::Muted, "skipped"),
        CheckState::Cancelled => mark(IconName::CircleMinus, Tint::Muted, "cancelled"),
        CheckState::Unknown => mark(IconName::CircleMinus, Tint::Muted, "unknown"),
    }
}

pub(crate) fn rollup(checks: &[Check]) -> Option<CheckState> {
    WORST_FIRST
        .into_iter()
        .find(|state| checks.iter().any(|check| check.state == *state))
}

pub(crate) fn checks_summary(checks: &[Check]) -> String {
    WORST_FIRST
        .into_iter()
        .filter_map(|state| {
            let count = checks.iter().filter(|check| check.state == state).count();
            (count > 0).then(|| format!("{} {}", thousands(count), check_mark(state).label))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn method_label(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Merge => "Create a merge commit",
        MergeMethod::Squash => "Squash and merge",
        MergeMethod::Rebase => "Rebase and merge",
        MergeMethod::SemiLinear => "Semi-linear merge",
        MergeMethod::FastForward => "Fast-forward",
        MergeMethod::Unknown => "Merge",
    }
}

pub(crate) fn method_button(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Squash => "Squash and merge",
        MergeMethod::Rebase => "Rebase and merge",
        MergeMethod::FastForward => "Fast-forward",
        MergeMethod::Merge | MergeMethod::SemiLinear | MergeMethod::Unknown => "Merge",
    }
}

pub(crate) fn method_about(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Merge => "Keeps every commit and adds a merge commit",
        MergeMethod::Squash => "Makes one commit of all the changes",
        MergeMethod::Rebase => "Replays each commit onto the base, without a merge commit",
        MergeMethod::SemiLinear => "Rebases, then adds a merge commit",
        MergeMethod::FastForward => "Moves the base to the head, without a merge commit",
        MergeMethod::Unknown => "",
    }
}

pub(crate) fn makes_a_commit(method: MergeMethod) -> bool {
    matches!(method, MergeMethod::Merge | MergeMethod::Squash | MergeMethod::SemiLinear)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MergeWords {
    pub(crate) mark: Mark,
    pub(crate) title: String,
    pub(crate) about: String,
    pub(crate) can_merge: bool,
}

pub(crate) fn merge_words(details: &PullDetails) -> MergeWords {
    let summary = &details.summary;
    let (base, head) = (&summary.base.branch, &summary.head.branch);
    let words = |mark: Mark, title: String, about: String, can_merge: bool| MergeWords {
        mark,
        title,
        about,
        can_merge,
    };
    match (summary.state, &details.mergeability) {
        (PullState::Merged, _) => words(
            mark(IconName::GitMerge, Tint::Magenta, "Merged"),
            format!("Merged into {base}"),
            format!("The branch {head} isn't needed any more."),
            false,
        ),
        (PullState::Closed, _) => words(
            mark(IconName::GitPullRequestClosed, Tint::Red, "Closed"),
            "Closed without merging".to_owned(),
            "Reopen it on the server to merge it.".to_owned(),
            false,
        ),
        (_, _) if summary.draft => words(
            mark(IconName::GitPullRequestDraft, Tint::Muted, "Draft"),
            "Still a draft".to_owned(),
            "Mark it ready for review on the server to merge it.".to_owned(),
            false,
        ),
        (_, Mergeability::Ready) => words(
            mark(IconName::CircleCheck, Tint::Green, "Ready"),
            "Ready to merge".to_owned(),
            format!("No conflicts with {base}, and the checks the repository requires passed."),
            true,
        ),
        (_, Mergeability::FailingChecks) => words(
            mark(IconName::TriangleAlert, Tint::Yellow, "Failing"),
            "Some checks failed".to_owned(),
            "The repository doesn't require them, so it can still be merged.".to_owned(),
            true,
        ),
        (_, Mergeability::Checking) => words(
            Mark {
                spins: true,
                ..mark(IconName::Loader, Tint::Muted, "Checking")
            },
            "Checking whether it can merge…".to_owned(),
            "The server is still working it out. SterneGit asks again after the next fetch."
                .to_owned(),
            false,
        ),
        (_, Mergeability::Conflicts) => words(
            mark(IconName::CircleX, Tint::Red, "Conflicts"),
            format!("Conflicts with {base}"),
            format!("Merge {base} into {head} or rebase it, resolve the conflicts, then push."),
            false,
        ),
        (_, Mergeability::Behind) => words(
            mark(IconName::ArrowDown, Tint::Yellow, "Behind"),
            format!("{base} moved on"),
            format!("The repository requires {head} to contain {base}: merge it in or rebase, then push."),
            false,
        ),
        (_, Mergeability::Draft) => words(
            mark(IconName::GitPullRequestDraft, Tint::Muted, "Draft"),
            "Still a draft".to_owned(),
            "Mark it ready for review on the server to merge it.".to_owned(),
            false,
        ),
        (_, Mergeability::Blocked { reason }) => words(
            mark(IconName::Lock, Tint::Red, "Blocked"),
            "Merging is blocked".to_owned(),
            reason.clone(),
            false,
        ),
        (_, Mergeability::Unknown) => words(
            mark(IconName::CircleMinus, Tint::Muted, "Unknown"),
            "SterneGit can't tell whether it can merge".to_owned(),
            "The server didn't say. Try merging on the server.".to_owned(),
            false,
        ),
    }
}

pub(crate) fn readable_head(summary: &PullSummary, base: &sternegit_protocol::RepoRef) -> String {
    match &summary.head.repo {
        Some(repo) if !same_repository(repo, base) => format!("{}:{}", repo.owner, summary.head.branch),
        _ => summary.head.branch.clone(),
    }
}

pub(crate) fn same_repository(one: &sternegit_protocol::RepoRef, other: &sternegit_protocol::RepoRef) -> bool {
    one.owner.eq_ignore_ascii_case(&other.owner) && one.name.eq_ignore_ascii_case(&other.name)
}

#[cfg(test)]
mod tests {
    use sternegit_protocol::{
        Check, CheckState, FetchSpec, MergeMethod, Mergeability, Person, PullDetails, PullState,
        PullSummary, RepoRef, Tip, Verdict,
    };

    use super::{checks_summary, merge_words, readable_head, rollup};

    fn check(state: CheckState) -> Check {
        Check {
            name: "build".into(),
            state,
            description: None,
            web_url: None,
        }
    }

    fn summary(state: PullState, draft: bool) -> PullSummary {
        let tip = |branch: &str, owner: &str| Tip {
            branch: branch.into(),
            sha: "8ff3473".into(),
            repo: Some(RepoRef {
                owner: owner.into(),
                name: "sternegit".into(),
            }),
        };
        PullSummary {
            number: 12,
            title: "Render side-by-side diffs".into(),
            state,
            draft,
            author: Person {
                login: "sam-lee".into(),
                name: None,
                avatar_url: None,
            },
            base: tip("main", "sternegit-org"),
            head: tip("feature/diff-view", "sternegit-org"),
            review: None,
            checks: None,
            created_at: "2026-11-03T18:50:00Z".into(),
            updated_at: "2026-11-04T08:15:00Z".into(),
            web_url: "https://github.com/sternegit-org/sternegit/pull/12".into(),
        }
    }

    fn details(state: PullState, mergeability: Mergeability) -> PullDetails {
        PullDetails {
            summary: summary(state, false),
            body: String::new(),
            base_sha: "1fe68e7".into(),
            mergeability,
            merge_methods: vec![MergeMethod::Merge, MergeMethod::Squash],
            verdicts: vec![Verdict::Comment],
            fetch: FetchSpec {
                url: None,
                refspec: "refs/pull/12/head".into(),
                sha: "8ff3473".into(),
            },
        }
    }

    #[test]
    fn the_summary_counts_checks_worst_first() {
        let checks = [
            check(CheckState::Success),
            check(CheckState::Failure),
            check(CheckState::Success),
            check(CheckState::Running),
        ];
        assert_eq!(checks_summary(&checks), "1 failing, 1 running, 2 passed");
        assert_eq!(rollup(&checks), Some(CheckState::Failure));
        assert_eq!(rollup(&[]), None);
    }

    #[test]
    fn only_ready_or_unrequired_failures_can_merge() {
        let can = |state, mergeability| merge_words(&details(state, mergeability)).can_merge;
        assert!(can(PullState::Open, Mergeability::Ready));
        assert!(can(PullState::Open, Mergeability::FailingChecks));
        assert!(!can(PullState::Open, Mergeability::Conflicts));
        assert!(!can(PullState::Open, Mergeability::Checking));
        assert!(!can(PullState::Merged, Mergeability::Ready));
        let blocked = merge_words(&details(PullState::Open, Mergeability::Blocked { reason: "One approving review is required.".into() }));
        assert_eq!((blocked.title.as_str(), blocked.about.as_str()), ("Merging is blocked", "One approving review is required."));
    }

    #[test]
    fn a_draft_never_merges_whatever_the_server_says() {
        let mut draft = details(PullState::Open, Mergeability::Ready);
        draft.summary.draft = true;
        let words = merge_words(&draft);
        assert_eq!((words.title.as_str(), words.can_merge), ("Still a draft", false));
    }

    #[test]
    fn a_head_from_a_fork_is_named_with_its_owner() {
        let base = RepoRef {
            owner: "sternegit-org".into(),
            name: "sternegit".into(),
        };
        let mut pull = summary(PullState::Open, false);
        assert_eq!(readable_head(&pull, &base), "feature/diff-view");
        pull.head.repo = Some(RepoRef {
            owner: "robin-chen".into(),
            name: "sternegit".into(),
        });
        assert_eq!(readable_head(&pull, &base), "robin-chen:feature/diff-view");
    }
}
