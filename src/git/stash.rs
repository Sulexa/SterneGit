use anyhow::Result;
use gix::ObjectId;
use gpui_kit::SharedString;

use super::{
    ChangeKind, Refusal, Repo,
    command::{Exit, run},
};

const STASH: &str = "refs/stash";
const STASH_LEFT_UNAPPLIED: [&str; 3] = [
    "conflicts in index",
    "could not restore untracked files",
    "would be overwritten by",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stash {
    pub(crate) id: ObjectId,
    pub(crate) name: SharedString,
    pub(crate) message: SharedString,
}

#[derive(Clone, Copy)]
enum BringBack {
    KeepingTheStash,
    DroppingTheStash,
}

impl BringBack {
    fn verb(self) -> &'static str {
        match self {
            BringBack::KeepingTheStash => "apply",
            BringBack::DroppingTheStash => "pop",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Carried {
    Along,
    WithConflicts,
    OnlyInStash,
}

impl Repo {
    pub(crate) fn has_stash(&self) -> bool {
        self.latest_stash().is_some()
    }

    pub(crate) fn stash(&self) -> Result<()> {
        run(
            &self.location,
            &["stash", "push", "--include-untracked", "--quiet"],
            Exit::Success,
        )
        .map(drop)
    }

    pub(crate) fn pop_stash(&self) -> Result<()> {
        all_carried_along(self.reapply_stash()?)
    }

    pub(crate) fn apply_stash(&self, id: ObjectId) -> Result<()> {
        let name = self.current_name_of_stash(id)?;
        all_carried_along(self.bring_back(BringBack::KeepingTheStash, &name)?)
    }

    pub(crate) fn pop_stash_entry(&self, id: ObjectId) -> Result<()> {
        let name = self.current_name_of_stash(id)?;
        all_carried_along(self.bring_back(BringBack::DroppingTheStash, &name)?)
    }

    pub(crate) fn drop_stash(&self, id: ObjectId) -> Result<()> {
        let name = self.current_name_of_stash(id)?;
        run(
            &self.location,
            &["stash", "drop", "--quiet", &name],
            Exit::Success,
        )
        .map(drop)
    }

    pub(super) fn stash_aside(&self) -> Result<bool> {
        let before = self.latest_stash();
        self.stash()?;
        Ok(self.latest_stash() != before)
    }

    pub(super) fn reapply_stash(&self) -> Result<Carried> {
        self.bring_back(BringBack::DroppingTheStash, &stash_name(0))
    }

    fn bring_back(&self, how: BringBack, stash: &str) -> Result<Carried> {
        match run(
            &self.location,
            &["stash", how.verb(), "--index", "--quiet", stash],
            Exit::Success,
        ) {
            Ok(_) => Ok(Carried::Along),
            Err(_) if self.has_conflicts() => Ok(Carried::WithConflicts),
            Err(error) if left_unapplied(&error) => Ok(Carried::OnlyInStash),
            Err(error) => Err(error),
        }
    }

    fn current_name_of_stash(&self, id: ObjectId) -> Result<String> {
        stash_ids_newest_first(&self.local())
            .iter()
            .position(|stash| *stash == id)
            .map(stash_name)
            .ok_or_else(|| Refusal::StashGone.into())
    }

    fn latest_stash(&self) -> Option<ObjectId> {
        let local = self.local();
        let stash = local.try_find_reference(STASH).ok()??;
        stash.try_id().map(|id| id.detach())
    }

    fn has_conflicts(&self) -> bool {
        self.status().is_ok_and(|changes| {
            changes
                .unstaged
                .iter()
                .chain(&changes.staged)
                .any(|change| change.kind == ChangeKind::Conflicted)
        })
    }
}

pub(super) fn stash_name(index: usize) -> String {
    format!("stash@{{{index}}}")
}

fn all_carried_along(carried: Carried) -> Result<()> {
    match carried {
        Carried::Along => Ok(()),
        Carried::WithConflicts => Err(Refusal::StashKept.into()),
        Carried::OnlyInStash => Err(Refusal::StashNotApplied.into()),
    }
}

pub(super) fn stash_ids_newest_first(repo: &gix::Repository) -> Vec<ObjectId> {
    let Ok(Some(stash)) = repo.try_find_reference(STASH) else {
        return Vec::new();
    };
    let mut log = stash.log_iter();
    let mut ids: Vec<ObjectId> = match log.all() {
        Ok(Some(lines)) => lines
            .filter_map(Result::ok)
            .map(|line| line.new_oid())
            .filter(|id| !id.is_null())
            .collect(),
        _ => Vec::new(),
    };
    if ids.is_empty() {
        ids.extend(stash.try_id().map(|id| id.detach()));
    }
    ids.reverse();
    ids
}

fn left_unapplied(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_lowercase();
    STASH_LEFT_UNAPPLIED.iter().any(|why| message.contains(why))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::git::{Refusal, scratch::ScratchRepo};

    fn changed_paths(scratch: &ScratchRepo) -> Result<Vec<String>> {
        let status = scratch.open()?.0.status()?;
        let mut paths: Vec<String> = status
            .unstaged
            .iter()
            .chain(&status.staged)
            .map(|change| change.path.to_string())
            .collect();
        paths.sort();
        Ok(paths)
    }

    #[test]
    fn a_stash_puts_every_change_aside_until_it_is_popped() -> Result<()> {
        let scratch = ScratchRepo::new("stash")?;
        scratch.write("tracked.txt", "one\n")?;
        scratch.git(&["add", "tracked.txt"])?;
        scratch.commit("first")?;
        scratch.write("tracked.txt", "two\n")?;
        scratch.write("untracked.txt", "new\n")?;
        let (repo, _) = scratch.open()?;
        assert!(!repo.has_stash());
        repo.stash()?;
        assert!(changed_paths(&scratch)?.is_empty());
        assert!(repo.has_stash());
        repo.pop_stash()?;
        assert_eq!(changed_paths(&scratch)?, ["tracked.txt", "untracked.txt"]);
        assert!(!repo.has_stash());
        Ok(())
    }

    #[test]
    fn a_popped_stash_keeps_what_was_staged_staged() -> Result<()> {
        let scratch = ScratchRepo::new("stash-staged")?;
        scratch.write("staged.txt", "one\n")?;
        scratch.write("unstaged.txt", "one\n")?;
        scratch.git(&["add", "staged.txt", "unstaged.txt"])?;
        scratch.commit("first")?;
        scratch.write("staged.txt", "two\n")?;
        scratch.git(&["add", "staged.txt"])?;
        scratch.write("unstaged.txt", "two\n")?;
        let (repo, _) = scratch.open()?;
        repo.stash()?;
        repo.pop_stash()?;
        assert_eq!(
            scratch.git(&["diff", "--cached", "--name-only"])?,
            "staged.txt\n"
        );
        assert_eq!(scratch.git(&["diff", "--name-only"])?, "unstaged.txt\n");
        Ok(())
    }

    #[test]
    fn a_stash_that_cannot_apply_is_kept_and_changes_nothing() -> Result<()> {
        let scratch = ScratchRepo::new("stash-not-applied")?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.write("shared.txt", "stashed\n")?;
        scratch.git(&["add", "shared.txt"])?;
        let (repo, _) = scratch.open()?;
        repo.stash()?;
        scratch.write("shared.txt", "committed\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("second")?;
        let error = repo
            .pop_stash()
            .expect_err("the staged change contradicts the commit");
        assert_eq!(
            error.downcast_ref::<Refusal>(),
            Some(&Refusal::StashNotApplied)
        );
        assert!(changed_paths(&scratch)?.is_empty());
        assert!(repo.has_stash());
        Ok(())
    }

    #[test]
    fn every_stash_is_listed_newest_first_with_its_message() -> Result<()> {
        let scratch = ScratchRepo::new("stash-list")?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", "two\n")?;
        scratch.git(&["stash", "push", "--quiet", "-m", "older"])?;
        scratch.write("a.txt", "three\n")?;
        scratch.git(&["stash", "push", "--quiet", "-m", "newer"])?;
        let (_, history) = scratch.open()?;
        let messages: Vec<&str> = history
            .stashes
            .iter()
            .map(|stash| stash.message.as_ref())
            .collect();
        assert_eq!(messages, ["On main: newer", "On main: older"]);
        let names: Vec<&str> = history
            .stashes
            .iter()
            .map(|stash| stash.name.as_ref())
            .collect();
        assert_eq!(names, ["stash@{0}", "stash@{1}"]);
        assert_eq!(
            history.stashes[0].id.to_string(),
            scratch.git(&["rev-parse", "stash@{0}"])?.trim()
        );
        Ok(())
    }

    fn two_stashes(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", "older\n")?;
        scratch.git(&["stash", "push", "--quiet", "-m", "older"])?;
        scratch.write("b.txt", "newer\n")?;
        scratch.git(&["add", "b.txt"])?;
        scratch.git(&["stash", "push", "--quiet", "-m", "newer"])?;
        Ok(scratch)
    }

    fn stash_messages(scratch: &ScratchRepo) -> Result<String> {
        scratch.git(&["stash", "list", "--format=%gs"])
    }

    #[test]
    fn applying_an_older_stash_brings_it_back_and_keeps_every_stash() -> Result<()> {
        let scratch = two_stashes("stash-apply")?;
        let (repo, history) = scratch.open()?;
        repo.apply_stash(history.stashes[1].id)?;
        assert_eq!(changed_paths(&scratch)?, ["a.txt"]);
        assert_eq!(stash_messages(&scratch)?, "On main: newer\nOn main: older\n");
        Ok(())
    }

    #[test]
    fn popping_an_older_stash_drops_only_that_stash() -> Result<()> {
        let scratch = two_stashes("stash-pop-older")?;
        let (repo, history) = scratch.open()?;
        repo.pop_stash_entry(history.stashes[1].id)?;
        assert_eq!(changed_paths(&scratch)?, ["a.txt"]);
        assert_eq!(stash_messages(&scratch)?, "On main: newer\n");
        Ok(())
    }

    #[test]
    fn a_popped_stash_entry_keeps_what_was_staged_staged() -> Result<()> {
        let scratch = two_stashes("stash-pop-staged")?;
        let (repo, history) = scratch.open()?;
        repo.pop_stash_entry(history.stashes[0].id)?;
        assert_eq!(
            scratch.git(&["diff", "--cached", "--name-only"])?,
            "b.txt\n"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_stash_leaves_the_others_and_the_work_tree_alone() -> Result<()> {
        let scratch = two_stashes("stash-drop")?;
        let (repo, history) = scratch.open()?;
        repo.drop_stash(history.stashes[0].id)?;
        assert!(changed_paths(&scratch)?.is_empty());
        assert_eq!(stash_messages(&scratch)?, "On main: older\n");
        Ok(())
    }

    #[test]
    fn a_stash_already_gone_is_refused_by_name() -> Result<()> {
        let scratch = two_stashes("stash-gone")?;
        let (repo, history) = scratch.open()?;
        scratch.git(&["stash", "drop", "--quiet", "stash@{1}"])?;
        let error = repo
            .drop_stash(history.stashes[1].id)
            .expect_err("the stash was dropped behind FastGit's back");
        assert_eq!(error.downcast_ref::<Refusal>(), Some(&Refusal::StashGone));
        assert_eq!(stash_messages(&scratch)?, "On main: newer\n");
        Ok(())
    }

    #[test]
    fn dropping_an_older_stash_moves_the_ref_digest() -> Result<()> {
        let scratch = ScratchRepo::new("stash-drop-digest")?;
        scratch.write("a.txt", "one\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        for content in ["two\n", "three\n"] {
            scratch.write("a.txt", content)?;
            scratch.git(&["stash", "push", "--quiet"])?;
        }
        let (repo, _) = scratch.open()?;
        let both = repo.ref_digest()?;
        scratch.git(&["stash", "drop", "--quiet", "stash@{1}"])?;
        assert_ne!(repo.ref_digest()?, both);
        Ok(())
    }

    #[test]
    fn a_pop_that_conflicts_keeps_the_stash() -> Result<()> {
        let scratch = ScratchRepo::new("stash-conflict")?;
        scratch.write("shared.txt", "one\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("first")?;
        scratch.write("shared.txt", "stashed\n")?;
        let (repo, _) = scratch.open()?;
        repo.stash()?;
        scratch.write("shared.txt", "committed\n")?;
        scratch.git(&["add", "shared.txt"])?;
        scratch.commit("second")?;
        let error = repo.pop_stash().expect_err("the stash conflicts");
        assert_eq!(error.downcast_ref::<Refusal>(), Some(&Refusal::StashKept));
        assert!(repo.has_stash());
        Ok(())
    }
}
