use gix::ObjectId;

use super::{History, RefLabel};

impl History {
    pub(crate) fn head_holds(&self, id: &ObjectId) -> bool {
        let Some(head) = self.head_id.and_then(|head| self.row_of(&head)) else {
            return false;
        };
        let Some(row) = self.row_of(id) else {
            return false;
        };
        let mut reached = vec![false; self.commits.len()];
        self.reach([head], &mut reached);
        reached[row]
    }

    pub(crate) fn commits_left_by_reset(&self, target: &ObjectId) -> Option<usize> {
        let head = self.row_of(&self.head_id?)?;
        let mut reached = vec![false; self.commits.len()];
        let other_tips = self
            .refs
            .iter()
            .filter(|label| !label.is_head)
            .map(|label| label.target)
            .chain([*target])
            .chain(self.stashes.iter().map(|stash| stash.id))
            .filter_map(|id| self.row_of(&id));
        self.reach(other_tips, &mut reached);
        Some(self.reach([head], &mut reached))
    }

    pub(crate) fn commits_only_on(&self, label: &RefLabel) -> Option<usize> {
        let tip = self.row_of(&label.target)?;
        let mut reached = vec![false; self.commits.len()];
        let other_tips = self
            .refs
            .iter()
            .filter(|other| other.kind != label.kind || other.name != label.name)
            .map(|other| other.target)
            .chain(self.head_id)
            .chain(self.stashes.iter().map(|stash| stash.id))
            .filter_map(|id| self.row_of(&id));
        self.reach(other_tips, &mut reached);
        Some(self.reach([tip], &mut reached))
    }

    fn reach(&self, tips: impl IntoIterator<Item = usize>, reached: &mut [bool]) -> usize {
        let mut pending: Vec<usize> = tips.into_iter().collect();
        let mut newly_reached = 0;
        while let Some(row) = pending.pop() {
            if std::mem::replace(&mut reached[row], true) {
                continue;
            }
            newly_reached += 1;
            let parents = self.commits[row].parents.iter();
            pending.extend(parents.filter_map(|parent| self.row_of(parent)));
        }
        newly_reached
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use gix::ObjectId;

    use crate::git::{History, RefKind, RefLabel, scratch::ScratchRepo};

    fn branch<'a>(history: &'a History, name: &str) -> &'a RefLabel {
        history
            .refs
            .iter()
            .find(|label| label.kind == RefKind::LocalBranch && label.name.as_str() == name)
            .expect("the branch exists")
    }

    #[test]
    fn commits_reached_from_another_ref_are_not_only_on_the_branch() -> Result<()> {
        let scratch = ScratchRepo::new("only-on")?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.commit("topic one")?;
        scratch.commit("topic two")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.git(&["branch", "merged", "main"])?;
        scratch.git(&["tag", "tagged", "topic~1"])?;
        let (_, history) = scratch.open()?;
        assert_eq!(history.commits_only_on(branch(&history, "topic")), Some(1));
        assert_eq!(history.commits_only_on(branch(&history, "merged")), Some(0));
        scratch.git(&["tag", "--delete", "tagged"])?;
        let (_, history) = scratch.open()?;
        assert_eq!(history.commits_only_on(branch(&history, "topic")), Some(2));
        Ok(())
    }

    #[test]
    fn a_detached_head_keeps_the_commits_it_sits_on() -> Result<()> {
        let scratch = ScratchRepo::new("only-on-detached")?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.commit("topic")?;
        scratch.git(&["checkout", "--quiet", "--detach"])?;
        let (_, history) = scratch.open()?;
        assert_eq!(history.commits_only_on(branch(&history, "topic")), Some(0));
        Ok(())
    }

    fn id(scratch: &ScratchRepo, revision: &str) -> Result<ObjectId> {
        Ok(ObjectId::from_hex(scratch.git(&["rev-parse", revision])?.trim().as_bytes())?)
    }

    #[test]
    fn head_holds_its_ancestors_and_not_other_branches() -> Result<()> {
        let scratch = ScratchRepo::new("head-holds")?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.commit("topic")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.commit("second")?;
        let (_, history) = scratch.open()?;
        assert!(history.head_holds(&id(&scratch, "main~1")?));
        assert!(history.head_holds(&id(&scratch, "main")?));
        assert!(!history.head_holds(&id(&scratch, "topic")?));
        Ok(())
    }

    #[test]
    fn a_reset_leaves_behind_the_commits_no_other_ref_holds() -> Result<()> {
        let scratch = ScratchRepo::new("reset-leaves")?;
        scratch.commit("first")?;
        scratch.commit("second")?;
        scratch.commit("third")?;
        scratch.git(&["tag", "kept", "main~1"])?;
        let (_, history) = scratch.open()?;
        let first = id(&scratch, "main~2")?;
        assert_eq!(history.commits_left_by_reset(&first), Some(1));
        assert_eq!(history.commits_left_by_reset(&id(&scratch, "main~1")?), Some(1));
        scratch.git(&["branch", "copy", "main"])?;
        let (_, history) = scratch.open()?;
        assert_eq!(history.commits_left_by_reset(&first), Some(0));
        Ok(())
    }

    #[test]
    fn a_stash_keeps_the_commits_it_was_made_on() -> Result<()> {
        let scratch = ScratchRepo::new("only-on-stash")?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("topic")?;
        scratch.write("a.txt", "two\n")?;
        scratch.git(&["stash", "push", "--quiet"])?;
        scratch.git(&["switch", "--quiet", "main"])?;
        let (_, history) = scratch.open()?;
        assert_eq!(history.commits_only_on(branch(&history, "topic")), Some(0));
        Ok(())
    }
}
