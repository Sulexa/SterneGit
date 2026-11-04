use std::hash::{DefaultHasher, Hash as _, Hasher as _};

use anyhow::Result;
use gix::{
    ObjectId,
    bstr::BStr,
    refs::{Category, FullName, TargetRef},
};
use gpui_kit::SharedString;

use super::text::decode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

impl RefKind {
    pub(crate) const ALL: [RefKind; 3] =
        [RefKind::LocalBranch, RefKind::RemoteBranch, RefKind::Tag];

    fn of(category: Category<'_>) -> Option<Self> {
        match category {
            Category::LocalBranch => Some(RefKind::LocalBranch),
            Category::RemoteBranch => Some(RefKind::RemoteBranch),
            Category::Tag => Some(RefKind::Tag),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RefLabel {
    pub(crate) name: SharedString,
    pub(crate) kind: RefKind,
    pub(crate) target: ObjectId,
    pub(crate) is_head: bool,
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
        let Ok(commit) = reference.peel_to_commit() else {
            continue;
        };
        labels.push(RefLabel {
            name,
            kind,
            target: commit.id,
            is_head,
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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::git::scratch::ScratchRepo;

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
