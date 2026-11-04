use super::{History, RefLabel};

impl History {
    pub(crate) fn commits_only_on(&self, label: &RefLabel) -> Option<usize> {
        let tip = self.row_of(&label.target)?;
        let mut reached = vec![false; self.commits.len()];
        let other_tips = self
            .refs
            .iter()
            .filter(|other| other.kind != label.kind || other.name != label.name)
            .map(|other| other.target)
            .chain(self.head_id)
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
}
