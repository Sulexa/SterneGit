use std::collections::HashMap;

use sternegit_protocol::{PullState, PullSummary, RepoRef};

use super::looks::same_repository;
use crate::{
    git::{RefKind, RefLabel, fork_branch_name},
    ui::PullMark,
};

pub(crate) type PullMarks = HashMap<String, PullMark>;

pub(crate) fn marks_for(pulls: &[PullSummary], refs: &[RefLabel], remote: &str, repo: &RepoRef) -> PullMarks {
    let mut marks = PullMarks::new();
    for pull in pulls.iter().filter(|pull| pull.state == PullState::Open) {
        let mark = PullMark {
            number: pull.number,
            draft: pull.draft,
        };
        for label in heads_of(pull, refs, remote, repo) {
            marks.entry(label.full_name()).or_insert(mark);
        }
    }
    marks
}

fn heads_of<'a>(pull: &PullSummary, refs: &'a [RefLabel], remote: &str, repo: &RepoRef) -> Vec<&'a RefLabel> {
    let local = |label: &&RefLabel| label.kind == RefKind::LocalBranch;
    let from_this_repository = pull
        .head
        .repo
        .as_ref()
        .is_some_and(|head| same_repository(head, repo));
    if !from_this_repository {
        let name = fork_branch_name(pull.number, &pull.head.branch);
        return refs.iter().filter(local).filter(|label| label.name == name).collect();
    }
    let tracking = format!("{remote}/{}", pull.head.branch);
    let locals: Vec<&RefLabel> = refs
        .iter()
        .filter(local)
        .filter(|label| match label.upstream.as_deref() {
            Some(upstream) => upstream == tracking,
            None => label.name == pull.head.branch,
        })
        .collect();
    if !locals.is_empty() {
        return locals;
    }
    refs.iter()
        .filter(|label| label.kind == RefKind::RemoteBranch && label.name == tracking)
        .collect()
}

#[cfg(test)]
mod tests {
    use sternegit_protocol::{PullState, PullSummary};
    use gix::ObjectId;

    use super::marks_for;
    use crate::{
        git::{RefKind, RefLabel},
        repo::pulls::samples::{pull, repo},
        ui::PullMark,
    };

    fn label(name: &str, kind: RefKind, upstream: Option<&str>) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind,
            target: ObjectId::null(gix::hash::Kind::Sha1),
            is_head: false,
            upstream: upstream.map(|upstream| upstream.to_owned().into()),
            from_upstream: None,
            worktree: None,
        }
    }

    fn draft(pull: PullSummary) -> PullSummary {
        PullSummary { draft: true, ..pull }
    }

    #[test]
    fn the_local_branch_tracking_the_head_carries_the_number() {
        let refs = [
            label("diff-view", RefKind::LocalBranch, Some("origin/feature/diff-view")),
            label("origin/feature/diff-view", RefKind::RemoteBranch, None),
        ];
        let marks = marks_for(&[pull(12, "feature/diff-view", "sternegit-org")], &refs, "origin", &repo("sternegit-org"));
        assert_eq!(marks.get("refs/heads/diff-view"), Some(&PullMark { number: 12, draft: false }));
        assert_eq!(marks.len(), 1);
    }

    #[test]
    fn without_a_local_branch_the_remote_branch_carries_it() {
        let refs = [label("origin/feature/blame", RefKind::RemoteBranch, None)];
        let marks = marks_for(&[draft(pull(11, "feature/blame", "sternegit-org"))], &refs, "origin", &repo("sternegit-org"));
        assert_eq!(marks.get("refs/remotes/origin/feature/blame"), Some(&PullMark { number: 11, draft: true }));
    }

    #[test]
    fn a_fork_pull_request_marks_only_the_branch_sternegit_made_for_it() {
        let refs = [
            label("patch-1", RefKind::LocalBranch, None),
            label("pr/9-patch-1", RefKind::LocalBranch, None),
        ];
        let marks = marks_for(&[pull(9, "patch-1", "robin-chen")], &refs, "origin", &repo("sternegit-org"));
        assert_eq!(marks.keys().collect::<Vec<_>>(), ["refs/heads/pr/9-patch-1"]);
    }

    #[test]
    fn closed_pull_requests_leave_no_mark() {
        let refs = [label("topic/tabs", RefKind::LocalBranch, None)];
        let merged = PullSummary {
            state: PullState::Merged,
            ..pull(13, "topic/tabs", "sternegit-org")
        };
        assert!(marks_for(&[merged], &refs, "origin", &repo("sternegit-org")).is_empty());
    }
}
