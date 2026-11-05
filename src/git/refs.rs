use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use anyhow::Result;
use gix::{
    ObjectId,
    bstr::BStr,
    refs::{Category, FullName, FullNameRef, TargetRef},
    remote::Direction,
};
use gpui_kit::SharedString;

use super::{stash::stash_ids_newest_first, text::decode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

impl RefKind {
    fn of(category: Category<'_>) -> Option<Self> {
        match category {
            Category::LocalBranch => Some(RefKind::LocalBranch),
            Category::RemoteBranch => Some(RefKind::RemoteBranch),
            Category::Tag => Some(RefKind::Tag),
            _ => None,
        }
    }

    fn namespace(self) -> &'static str {
        match self {
            RefKind::LocalBranch => "refs/heads/",
            RefKind::RemoteBranch => "refs/remotes/",
            RefKind::Tag => "refs/tags/",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RefLabel {
    pub(crate) name: SharedString,
    pub(crate) kind: RefKind,
    pub(crate) target: ObjectId,
    pub(crate) is_head: bool,
    pub(crate) upstream: Option<SharedString>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ShownRef<'a> {
    pub(crate) label: &'a RefLabel,
    pub(crate) with_upstream: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RemoteBranch<'a> {
    pub(crate) remote: &'a str,
    pub(crate) branch: &'a str,
}

impl RefLabel {
    pub(crate) fn is_branch(&self) -> bool {
        matches!(self.kind, RefKind::LocalBranch | RefKind::RemoteBranch)
    }

    pub(crate) fn full_name(&self) -> String {
        format!("{}{}", self.kind.namespace(), self.name)
    }

    pub(crate) fn on_remote(&self) -> Option<RemoteBranch<'_>> {
        let (remote, branch) = self.name.split_once('/')?;
        (self.kind == RefKind::RemoteBranch).then_some(RemoteBranch { remote, branch })
    }

    pub(crate) fn local_name(&self) -> &str {
        self.on_remote().map_or(&self.name, |remote| remote.branch)
    }

    pub(crate) fn revision(&self, among: &[RefLabel]) -> String {
        let shares_its_name = among
            .iter()
            .any(|other| other.kind != self.kind && other.name == self.name);
        if shares_its_name {
            self.full_name()
        } else {
            self.name.to_string()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RefDigest(u64);

impl RefDigest {
    pub(crate) fn of(repo: &gix::Repository) -> Result<Self> {
        let mut hasher = DefaultHasher::new();
        repo.head_name()?
            .map(|name| name.as_bstr().to_vec())
            .hash(&mut hasher);
        repo.head_id().ok().map(|id| id.detach()).hash(&mut hasher);
        for reference in repo.references()?.all()? {
            let Ok(reference) = reference else { continue };
            reference.name().as_bstr().hash(&mut hasher);
            match reference.target() {
                TargetRef::Object(id) => id.as_bytes().hash(&mut hasher),
                TargetRef::Symbolic(name) => name.as_bstr().hash(&mut hasher),
            }
        }
        stash_ids_newest_first(repo).hash(&mut hasher);
        Ok(Self(hasher.finish()))
    }
}

pub(crate) fn read_labels(
    repo: &gix::Repository,
    head: Option<&FullName>,
) -> Result<Vec<RefLabel>> {
    let mut labels = Vec::new();
    for reference in repo.references()?.all()? {
        let Ok(mut reference) = reference else {
            continue;
        };
        let Some((category, short_name)) = reference.name().category_and_short_name() else {
            continue;
        };
        let Some(kind) = RefKind::of(category) else {
            continue;
        };
        if kind == RefKind::RemoteBranch && is_alias_for_another_remote_branch(short_name) {
            continue;
        }
        let name = decode(short_name).into();
        let is_head = kind == RefKind::LocalBranch
            && head.is_some_and(|head| head.as_bstr() == reference.name().as_bstr());
        let upstream = match kind {
            RefKind::LocalBranch => upstream_of(repo, reference.name()),
            RefKind::RemoteBranch | RefKind::Tag => None,
        };
        let Ok(commit) = reference.peel_to_commit() else {
            continue;
        };
        labels.push(RefLabel {
            name,
            kind,
            target: commit.id,
            is_head,
            upstream,
        });
    }
    labels.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.name.as_str().cmp(b.name.as_str()))
    });
    Ok(labels)
}

fn is_alias_for_another_remote_branch(short_name: &BStr) -> bool {
    short_name.ends_with(b"/HEAD")
}

fn upstream_of(repo: &gix::Repository, branch: &FullNameRef) -> Option<SharedString> {
    let tracking = repo
        .branch_remote_tracking_ref_name(branch, Direction::Fetch)?
        .ok()?;
    match tracking.as_ref().category_and_short_name()? {
        (Category::RemoteBranch, short_name) => Some(decode(short_name).into()),
        _ => None,
    }
}

pub(crate) fn fold_upstreams_in<'a>(same_commit: &[&'a RefLabel]) -> Vec<ShownRef<'a>> {
    let is_here = |upstream: &SharedString| {
        same_commit
            .iter()
            .any(|label| label.kind == RefKind::RemoteBranch && label.name == *upstream)
    };
    let is_folded = |label: &RefLabel| {
        label.kind == RefKind::RemoteBranch
            && same_commit
                .iter()
                .any(|local| local.upstream.as_ref() == Some(&label.name))
    };
    same_commit
        .iter()
        .filter(|label| !is_folded(label))
        .map(|&label| ShownRef {
            label,
            with_upstream: label.upstream.as_ref().is_some_and(is_here),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use crate::git::{History, scratch::ScratchRepo};

    fn shown(history: &History, id: ObjectId) -> Vec<(&str, bool)> {
        history
            .shown_on(&id)
            .iter()
            .map(|shown| (shown.label.name.as_str(), shown.with_upstream))
            .collect()
    }

    #[test]
    fn a_branch_on_the_same_commit_as_its_upstream_takes_it_in() -> Result<()> {
        let origin = ScratchRepo::bare("fold-origin")?;
        let ours = ScratchRepo::new("fold-ours")?;
        ours.commit("first")?;
        ours.git(&["remote", "add", "origin", &origin.location()])?;
        ours.git(&["push", "--quiet", "--set-upstream", "origin", "main"])?;
        ours.git(&["push", "--quiet", "origin", "main:topic"])?;
        ours.git(&["branch", "--no-track", "topic"])?;
        let (_, history) = ours.open()?;
        let first = history.head_id.expect("a commit");
        assert_eq!(
            shown(&history, first),
            [("main", true), ("topic", false), ("origin/topic", false)],
            "origin/topic shares a name with topic but isn't its upstream"
        );
        ours.commit("second")?;
        let (_, history) = ours.open()?;
        let second = history.head_id.expect("a commit");
        assert_eq!(shown(&history, second), [("main", false)]);
        assert_eq!(
            shown(&history, first),
            [("topic", false), ("origin/main", false), ("origin/topic", false)],
            "main moved ahead of origin/main"
        );
        Ok(())
    }

    #[test]
    fn the_ref_digest_follows_head_alone() -> Result<()> {
        let scratch = ScratchRepo::new("head-digest")?;
        scratch.commit("first")?;
        scratch.commit("second")?;
        scratch.git(&["branch", "topic"])?;
        let (repo, _) = scratch.open()?;
        let on_default_branch = repo.ref_digest()?;
        scratch.git(&["checkout", "--quiet", "topic"])?;
        assert_ne!(
            repo.ref_digest()?,
            on_default_branch,
            "switching to a branch at the same commit"
        );
        scratch.git(&["checkout", "--quiet", "--detach", "HEAD"])?;
        let detached = repo.ref_digest()?;
        scratch.git(&["checkout", "--quiet", "--detach", "HEAD~1"])?;
        assert_ne!(repo.ref_digest()?, detached, "moving a detached HEAD");
        Ok(())
    }

    #[test]
    fn the_ref_digest_follows_moved_refs() -> Result<()> {
        let scratch = ScratchRepo::new("digest")?;
        scratch.commit("first")?;
        let (repo, history) = scratch.open()?;
        assert_eq!(repo.ref_digest()?, history.digest, "nothing moved yet");
        scratch.commit("second")?;
        let after_commit = repo.ref_digest()?;
        assert_ne!(
            after_commit, history.digest,
            "a new commit moves the branch"
        );
        scratch.git(&["branch", "topic"])?;
        assert_ne!(
            repo.ref_digest()?,
            after_commit,
            "a new branch is a new ref"
        );
        Ok(())
    }
}
