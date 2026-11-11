use std::{path::Path, rc::Rc, sync::Arc};

use gix::ObjectId;
use gpui_kit::SharedString;

use crate::{
    git::{History, Side, WorkingChanges},
    ui::{Pick, Picked},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OpenFile {
    pub(crate) side: Side,
    pub(crate) path: SharedString,
}

impl OpenFile {
    pub(crate) fn follow(&self, changes: &WorkingChanges) -> Option<OpenFile> {
        [self.side, self.side.other()].into_iter().find_map(|side| {
            changes.find(side, &self.path).map(|change| OpenFile {
                side,
                path: change.path.clone(),
            })
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Target {
    #[default]
    Nothing,
    Commit {
        id: ObjectId,
        open: Option<SharedString>,
    },
    Commits {
        picked: Rc<Picked<ObjectId>>,
        open: Option<SharedString>,
    },
    WorkingTree {
        open: Option<OpenFile>,
    },
    Worktree {
        location: Arc<Path>,
        open: Option<OpenFile>,
    },
    Pull {
        number: u64,
        open: Option<SharedString>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Newer,
    Older,
}

impl Target {
    pub(crate) fn at_commit(id: ObjectId) -> Self {
        Target::Commit { id, open: None }
    }

    pub(crate) fn commit(&self) -> Option<ObjectId> {
        match self {
            Target::Commit { id, .. } => Some(*id),
            Target::Nothing
            | Target::Commits { .. }
            | Target::WorkingTree { .. }
            | Target::Worktree { .. }
            | Target::Pull { .. } => None,
        }
    }

    pub(crate) fn several(&self) -> Option<&Rc<Picked<ObjectId>>> {
        match self {
            Target::Commits { picked, .. } => Some(picked),
            _ => None,
        }
    }

    pub(crate) fn picked(&self) -> Rc<Picked<ObjectId>> {
        match self {
            Target::Commits { picked, .. } => picked.clone(),
            Target::Commit { id, .. } => Rc::new(Picked::only(*id)),
            Target::Nothing | Target::WorkingTree { .. } | Target::Worktree { .. } | Target::Pull { .. } => {
                Rc::default()
            }
        }
    }

    pub(crate) fn pull(&self) -> Option<u64> {
        match self {
            Target::Pull { number, .. } => Some(*number),
            Target::Nothing
            | Target::Commit { .. }
            | Target::Commits { .. }
            | Target::WorkingTree { .. }
            | Target::Worktree { .. } => None,
        }
    }

    pub(crate) fn worktree(&self) -> Option<&Arc<Path>> {
        match self {
            Target::Worktree { location, .. } => Some(location),
            Target::Nothing
            | Target::Commit { .. }
            | Target::Commits { .. }
            | Target::WorkingTree { .. }
            | Target::Pull { .. } => None,
        }
    }

    pub(crate) fn has_open_file(&self) -> bool {
        match self {
            Target::Commit { open, .. } | Target::Commits { open, .. } | Target::Pull { open, .. } => {
                open.is_some()
            }
            Target::WorkingTree { open } | Target::Worktree { open, .. } => open.is_some(),
            Target::Nothing => false,
        }
    }

    pub(crate) fn close_file(&mut self) -> bool {
        match self {
            Target::Commit { open, .. } | Target::Commits { open, .. } | Target::Pull { open, .. } => {
                open.take().is_some()
            }
            Target::WorkingTree { open } | Target::Worktree { open, .. } => open.take().is_some(),
            Target::Nothing => false,
        }
    }

    pub(crate) fn working_file(&self) -> Option<&OpenFile> {
        match self {
            Target::WorkingTree { open } | Target::Worktree { open, .. } => open.as_ref(),
            Target::Nothing | Target::Commit { .. } | Target::Commits { .. } | Target::Pull { .. } => None,
        }
    }

    pub(crate) fn after_pick(&self, id: ObjectId, pick: Pick, shown: &[ObjectId]) -> Option<Target> {
        of_picked(self.picked().after(id, pick, shown))
    }

    pub(crate) fn grown(&self, step: Step, history: &History) -> Option<Target> {
        let shown = history.pickable();
        let cursor = self.focus()?;
        let at = shown.iter().position(|id| *id == cursor)?;
        let next = match step {
            Step::Newer => at.checked_sub(1)?,
            Step::Older => at + 1,
        };
        self.after_pick(*shown.get(next)?, Pick::Run, &shown)
    }

    pub(crate) fn back_to_one(&self) -> Option<Target> {
        match self {
            Target::Commits { picked, open: None } => picked.anchor().copied().map(Target::at_commit),
            _ => None,
        }
    }

    fn focus(&self) -> Option<ObjectId> {
        match self {
            Target::Commit { id, .. } => Some(*id),
            Target::Commits { picked, .. } => picked.cursor().copied(),
            Target::Nothing | Target::WorkingTree { .. } | Target::Worktree { .. } | Target::Pull { .. } => {
                None
            }
        }
    }

    pub(crate) fn with_working_file(&self, file: OpenFile) -> Target {
        match self {
            Target::Worktree { location, .. } => Target::Worktree {
                location: location.clone(),
                open: Some(file),
            },
            _ => Target::WorkingTree { open: Some(file) },
        }
    }

    pub(crate) fn is_working_tree(&self) -> bool {
        matches!(self, Target::WorkingTree { .. })
    }

    pub(crate) fn after_history_read(
        &self,
        history: &History,
        changes: Option<&WorkingChanges>,
    ) -> Target {
        match self {
            Target::WorkingTree { .. }
                if changes.is_some_and(|changes| clean_at_shown_head(changes, history)) =>
            {
                head_or_working_tree(history)
            }
            Target::WorkingTree { .. } => self.clone(),
            Target::Worktree { location, .. } if history.changes_row_of(location).is_some() => {
                self.clone()
            }
            Target::Worktree { location, .. } => history
                .worktree(location)
                .and_then(|worktree| worktree.head)
                .filter(|head| history.row_of(head).is_some())
                .map_or_else(|| commit_or_nothing(head_or_newest(history)), Target::at_commit),
            Target::Commit { id, .. } if history.row_of(id).is_some() => self.clone(),
            Target::Commit { .. } => commit_or_nothing(history.newest()),
            Target::Commits { picked, .. } => {
                let mut kept = (**picked).clone();
                kept.retain(|id| history.row_of(id).is_some() && history.stash_index(id).is_none());
                if kept == **picked {
                    self.clone()
                } else {
                    of_picked(kept).unwrap_or_else(|| commit_or_nothing(history.newest()))
                }
            }
            Target::Pull { .. } => self.clone(),
            Target::Nothing => commit_or_nothing(head_or_newest(history)),
        }
    }

    pub(crate) fn after_changes_read(&self, changes: &WorkingChanges, history: &History) -> Target {
        match self {
            Target::Nothing if !changes.is_empty() => Target::WorkingTree { open: None },
            Target::WorkingTree { .. } if clean_at_shown_head(changes, history) => {
                head_or_working_tree(history)
            }
            Target::WorkingTree { open: Some(file) } => Target::WorkingTree {
                open: file.follow(changes),
            },
            _ => self.clone(),
        }
    }

    pub(crate) fn step(&self, step: Step, history: &History, has_changes: bool) -> Option<Target> {
        let newest = history.newest()?;
        match (self, step) {
            (Target::Pull { .. }, _) => None,
            (Target::WorkingTree { .. }, Step::Older) => Some(Target::at_commit(newest)),
            (Target::WorkingTree { .. } | Target::Worktree { .. }, Step::Newer) => None,
            (Target::Worktree { location, .. }, Step::Older) => history
                .changes_row_of(location)
                .and_then(|row| history.commits.get(row + 1))
                .map(|commit| Target::at_commit(commit.id)),
            (Target::Commit { .. } | Target::Commits { .. }, Step::Newer)
                if self.focus() == Some(newest) && has_changes =>
            {
                Some(Target::WorkingTree { open: None })
            }
            _ => {
                let row = self
                    .focus()
                    .and_then(|id| history.row_of(&id))
                    .unwrap_or(0);
                let last = history.commits.len() - 1;
                let mut row = row;
                loop {
                    let next = match step {
                        Step::Newer => row.checked_sub(1),
                        Step::Older => Some(row + 1).filter(|next| *next <= last),
                    };
                    match next {
                        Some(next) if history.worktree_on_row(next).is_some() => row = next,
                        Some(next) => break Some(Target::at_commit(history.commits[next].id)),
                        None => break self.focus().map(Target::at_commit),
                    }
                }
            }
        }
    }
}

fn of_picked(picked: Picked<ObjectId>) -> Option<Target> {
    match picked.len() {
        0 => None,
        1 => picked.keys().next().copied().map(Target::at_commit),
        _ => Some(Target::Commits {
            picked: Rc::new(picked),
            open: None,
        }),
    }
}

fn commit_or_nothing(id: Option<ObjectId>) -> Target {
    id.map_or(Target::Nothing, Target::at_commit)
}

fn head_or_newest(history: &History) -> Option<ObjectId> {
    history
        .head_id
        .filter(|head| history.row_of(head).is_some())
        .or(history.newest())
}

fn head_or_working_tree(history: &History) -> Target {
    head_or_newest(history).map_or(Target::WorkingTree { open: None }, Target::at_commit)
}

fn clean_at_shown_head(changes: &WorkingChanges, history: &History) -> bool {
    changes.is_empty() && changes.head == history.head_id
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{OpenFile, Step, Target};
    use crate::{
        git::{
            ChangeKind, FileChange, Side, WorkingChanges,
            fixtures::{id, linear_history},
        },
        ui::Pick,
    };

    fn file(path: &str) -> FileChange {
        FileChange::new(path.as_bytes(), None, ChangeKind::Modified)
    }

    fn changes(unstaged: &[&str], staged: &[&str]) -> WorkingChanges {
        WorkingChanges {
            unstaged: unstaged.iter().copied().map(file).collect(),
            staged: staged.iter().copied().map(file).collect(),
            ..WorkingChanges::default()
        }
    }

    fn clean_at(head: u8) -> WorkingChanges {
        dirty_at(head, &[], &[])
    }

    fn dirty_at(head: u8, unstaged: &[&str], staged: &[&str]) -> WorkingChanges {
        WorkingChanges {
            head: Some(id(head)),
            ..changes(unstaged, staged)
        }
    }

    fn at(byte: u8) -> Target {
        Target::at_commit(id(byte))
    }

    fn open_in(byte: u8, path: &str) -> Target {
        Target::Commit {
            id: id(byte),
            open: Some(path.to_owned().into()),
        }
    }

    fn open(side: Side, path: &str) -> Target {
        Target::WorkingTree {
            open: Some(OpenFile {
                side,
                path: path.to_owned().into(),
            }),
        }
    }

    #[test]
    fn a_pull_request_stays_selected_through_reads_and_ignores_steps() {
        let history = linear_history(&[3, 2, 1], Some(2));
        let pull = Target::Pull {
            number: 12,
            open: Some("src/main.rs".into()),
        };
        assert_eq!(pull.after_history_read(&history, None), pull);
        assert_eq!(pull.after_changes_read(&dirty_at(2, &["a.txt"], &[]), &history), pull);
        assert_eq!(pull.step(Step::Older, &history, true), None);
        assert_eq!((pull.pull(), pull.commit(), pull.has_open_file()), (Some(12), None, true));
        let mut closing = pull.clone();
        assert!(closing.close_file());
        assert_eq!(closing, Target::Pull { number: 12, open: None });
    }

    #[test]
    fn the_first_read_selects_head() {
        let history = linear_history(&[3, 2, 1], Some(2));
        assert_eq!(Target::Nothing.after_history_read(&history, None), at(2));
    }

    #[test]
    fn without_head_the_first_read_selects_the_newest_commit() {
        let history = linear_history(&[3, 2, 1], None);
        assert_eq!(Target::Nothing.after_history_read(&history, None), at(3));
    }

    #[test]
    fn an_empty_history_selects_nothing() {
        let history = linear_history(&[], None);
        assert_eq!(
            Target::Nothing.after_history_read(&history, None),
            Target::Nothing
        );
    }

    #[test]
    fn a_reload_keeps_the_selected_commit_while_it_exists() {
        let history = linear_history(&[4, 3, 2, 1], Some(4));
        assert_eq!(at(2).after_history_read(&history, None), at(2));
    }

    #[test]
    fn a_reload_falls_back_to_the_newest_commit_when_the_selected_one_is_gone() {
        let history = linear_history(&[4, 3], Some(3));
        assert_eq!(at(9).after_history_read(&history, None), at(4));
    }

    #[test]
    fn a_reload_keeps_the_uncommitted_changes_and_their_open_file() {
        let history = linear_history(&[2, 1], Some(2));
        let target = open(Side::Unstaged, "a.rs");
        let left = dirty_at(2, &["a.rs"], &[]);
        assert_eq!(target.after_history_read(&history, Some(&left)), target);
        assert_eq!(target.after_history_read(&history, None), target);
    }

    #[test]
    fn once_the_changes_are_committed_the_reload_selects_the_new_head() {
        let history = linear_history(&[3, 2, 1], Some(3));
        let target = open(Side::Unstaged, "a.rs");
        assert_eq!(
            target.after_history_read(&history, Some(&clean_at(3))),
            at(3)
        );
    }

    #[test]
    fn changes_are_shown_when_there_is_no_commit_to_select() {
        let history = linear_history(&[], None);
        let some = changes(&["a.rs"], &[]);
        assert_eq!(
            Target::Nothing.after_changes_read(&some, &history),
            Target::WorkingTree { open: None }
        );
        assert_eq!(
            Target::Nothing.after_changes_read(&changes(&[], &[]), &history),
            Target::Nothing
        );
    }

    #[test]
    fn a_selected_commit_stays_when_the_working_tree_becomes_clean() {
        let history = linear_history(&[2, 1], Some(2));
        assert_eq!(at(1).after_changes_read(&clean_at(2), &history), at(1));
        assert_eq!(
            at(1).after_history_read(&history, Some(&clean_at(2))),
            at(1)
        );
    }

    #[test]
    fn changes_that_disappear_select_head_rather_than_the_newest_commit() {
        let history = linear_history(&[3, 2, 1], Some(2));
        assert_eq!(
            open(Side::Staged, "a.rs").after_changes_read(&clean_at(2), &history),
            at(2)
        );
    }

    #[test]
    fn a_clean_status_from_a_newer_head_waits_for_the_history_that_holds_it() {
        let stale = linear_history(&[2, 1], Some(2));
        let waiting = open(Side::Staged, "a.rs").after_changes_read(&clean_at(3), &stale);
        assert_eq!(waiting, Target::WorkingTree { open: None });
        assert_eq!(
            waiting.after_history_read(&stale, Some(&clean_at(3))),
            Target::WorkingTree { open: None }
        );
        let fresh = linear_history(&[3, 2, 1], Some(3));
        assert_eq!(
            waiting.after_history_read(&fresh, Some(&clean_at(3))),
            at(3)
        );
    }

    #[test]
    fn changes_left_at_head_keep_the_working_tree_selected() {
        let history = linear_history(&[2, 1], Some(2));
        let left = dirty_at(2, &["a.rs"], &[]);
        let target = Target::WorkingTree { open: None };
        assert_eq!(target.after_changes_read(&left, &history), target);
        assert_eq!(target.after_history_read(&history, Some(&left)), target);
    }

    #[test]
    fn a_clean_tree_without_a_head_commit_selects_the_newest_commit() {
        let history = linear_history(&[3, 2, 1], None);
        let clean = changes(&[], &[]);
        assert_eq!(
            open(Side::Unstaged, "a.rs").after_changes_read(&clean, &history),
            at(3)
        );
    }

    #[test]
    fn without_commits_the_working_tree_stays_selected_when_it_is_clean() {
        let history = linear_history(&[], None);
        let clean = changes(&[], &[]);
        let target = Target::WorkingTree { open: None };
        assert_eq!(target.after_changes_read(&clean, &history), target);
        assert_eq!(target.after_history_read(&history, Some(&clean)), target);
    }

    #[test]
    fn the_open_file_follows_its_change_to_the_other_side() {
        let history = linear_history(&[1], Some(1));
        let staged_in_full = dirty_at(1, &[], &["a.rs"]);
        assert_eq!(
            open(Side::Unstaged, "a.rs").after_changes_read(&staged_in_full, &history),
            open(Side::Staged, "a.rs")
        );
    }

    #[test]
    fn a_file_changed_on_both_sides_stays_on_its_side() {
        let history = linear_history(&[1], Some(1));
        let both = dirty_at(1, &["a.rs"], &["a.rs"]);
        assert_eq!(
            open(Side::Staged, "a.rs").after_changes_read(&both, &history),
            open(Side::Staged, "a.rs")
        );
    }

    #[test]
    fn the_open_file_closes_when_its_change_is_gone() {
        let history = linear_history(&[1], Some(1));
        assert_eq!(
            open(Side::Unstaged, "a.rs").after_changes_read(&dirty_at(1, &["b.rs"], &[]), &history),
            Target::WorkingTree { open: None }
        );
    }

    #[test]
    fn stepping_newer_from_the_newest_commit_selects_the_changes() {
        let history = linear_history(&[2, 1], Some(2));
        assert_eq!(
            at(2).step(Step::Newer, &history, true),
            Some(Target::WorkingTree { open: None })
        );
        assert_eq!(at(2).step(Step::Newer, &history, false), Some(at(2)));
    }

    #[test]
    fn stepping_older_from_the_changes_selects_the_newest_commit_and_drops_the_open_file() {
        let history = linear_history(&[2, 1], Some(1));
        assert_eq!(
            open(Side::Unstaged, "a.rs").step(Step::Older, &history, true),
            Some(at(2))
        );
        assert_eq!(
            open(Side::Unstaged, "a.rs").step(Step::Newer, &history, true),
            None
        );
    }

    #[test]
    fn steps_stop_at_both_ends() {
        let history = linear_history(&[3, 2, 1], Some(3));
        assert_eq!(at(1).step(Step::Older, &history, false), Some(at(1)));
        assert_eq!(at(3).step(Step::Newer, &history, false), Some(at(3)));
        assert_eq!(at(2).step(Step::Older, &history, false), Some(at(1)));
        assert_eq!(at(2).step(Step::Newer, &history, false), Some(at(3)));
    }

    #[test]
    fn a_reload_keeps_the_file_open_in_the_selected_commit() {
        let history = linear_history(&[3, 2, 1], Some(3));
        let target = open_in(2, "a.rs");
        assert_eq!(target.after_history_read(&history, None), target);
        assert_eq!(
            target.after_history_read(&history, Some(&clean_at(3))),
            target
        );
    }

    #[test]
    fn a_file_open_in_a_commit_that_is_gone_closes_with_it() {
        let history = linear_history(&[4, 3], Some(4));
        assert_eq!(open_in(9, "a.rs").after_history_read(&history, None), at(4));
    }

    #[test]
    fn new_changes_leave_the_file_open_in_a_commit_alone() {
        let history = linear_history(&[2, 1], Some(2));
        let target = open_in(1, "a.rs");
        assert_eq!(
            target.after_changes_read(&dirty_at(2, &["a.rs"], &[]), &history),
            target
        );
    }

    #[test]
    fn stepping_away_from_a_commit_closes_its_open_file() {
        let history = linear_history(&[3, 2, 1], Some(3));
        assert_eq!(
            open_in(2, "a.rs").step(Step::Older, &history, false),
            Some(at(1))
        );
        assert_eq!(
            open_in(3, "a.rs").step(Step::Newer, &history, true),
            Some(Target::WorkingTree { open: None })
        );
    }

    #[test]
    fn closing_the_file_keeps_what_is_selected() {
        let mut in_commit = open_in(2, "a.rs");
        assert!(in_commit.has_open_file());
        assert!(in_commit.close_file());
        assert_eq!(in_commit, at(2));
        assert!(!in_commit.has_open_file());
        assert!(!in_commit.close_file());

        let mut in_working_tree = open(Side::Staged, "a.rs");
        assert!(in_working_tree.close_file());
        assert_eq!(in_working_tree, Target::WorkingTree { open: None });
        assert!(!Target::Nothing.close_file());
    }

    fn picks(target: &Target) -> Vec<u8> {
        let mut bytes: Vec<u8> = (1..=9)
            .filter(|byte| target.picked().contains(&id(*byte)))
            .collect();
        bytes.sort_unstable();
        bytes
    }

    fn shown(bytes: &[u8]) -> Vec<gix::ObjectId> {
        bytes.iter().copied().map(id).collect()
    }

    #[test]
    fn ctrl_and_shift_pick_several_commits_and_one_left_is_a_plain_selection() {
        let rows = shown(&[5, 4, 3, 2, 1]);
        let added = at(4).after_pick(id(2), Pick::Toggle, &rows).expect("a pick");
        assert_eq!(picks(&added), [2, 4]);
        assert!(matches!(added, Target::Commits { open: None, .. }));
        assert_eq!(added.commit(), None);
        let run = added.after_pick(id(5), Pick::Run, &rows).expect("a run");
        assert_eq!(picks(&run), [2, 3, 4, 5]);
        assert_eq!(added.after_pick(id(2), Pick::Toggle, &rows), Some(at(4)));
        assert_eq!(at(4).after_pick(id(4), Pick::Toggle, &rows), None);
        assert_eq!(run.after_pick(id(1), Pick::Only, &rows), Some(at(1)));
    }

    #[test]
    fn shift_steps_grow_and_shrink_the_run_from_the_last_click() {
        let history = linear_history(&[5, 4, 3, 2, 1], Some(5));
        let grown = at(3).grown(Step::Older, &history).expect("grown");
        assert_eq!(picks(&grown), [2, 3]);
        let grown = grown.grown(Step::Older, &history).expect("grown again");
        assert_eq!(picks(&grown), [1, 2, 3]);
        assert_eq!(grown.grown(Step::Older, &history), None);
        let shrunk = grown.grown(Step::Newer, &history).expect("shrunk");
        assert_eq!(picks(&shrunk), [2, 3]);
        assert_eq!(shrunk.step(Step::Older, &history, false), Some(at(1)));
        assert_eq!(shrunk.step(Step::Newer, &history, false), Some(at(3)));
        assert_eq!(shrunk.back_to_one(), Some(at(3)));
    }

    #[test]
    fn a_reload_keeps_the_picked_commits_that_are_still_there() {
        let rows = shown(&[5, 4, 3, 2, 1]);
        let picked = at(5)
            .after_pick(id(3), Pick::Toggle, &rows)
            .and_then(|target| target.after_pick(id(1), Pick::Toggle, &rows))
            .expect("a pick");
        let mut opened = picked.clone();
        if let Target::Commits { open, .. } = &mut opened {
            *open = Some("a.rs".into());
        }
        let all_there = linear_history(&[5, 4, 3, 2, 1], Some(5));
        assert_eq!(opened.after_history_read(&all_there, None), opened);
        let one_gone = linear_history(&[5, 4, 3, 2], Some(5));
        assert_eq!(picks(&picked.after_history_read(&one_gone, None)), [3, 5]);
        let two_gone = linear_history(&[5, 4, 2], Some(5));
        assert_eq!(picked.after_history_read(&two_gone, None), at(5));
        let all_gone = linear_history(&[7, 6], Some(7));
        assert_eq!(picked.after_history_read(&all_gone, None), at(7));
    }

    #[test]
    fn a_pick_with_a_file_open_closes_the_file_before_going_back_to_one_commit() {
        let rows = shown(&[3, 2, 1]);
        let picked = at(3).after_pick(id(1), Pick::Run, &rows).expect("a run");
        let mut opened = picked.clone();
        if let Target::Commits { open, .. } = &mut opened {
            *open = Some("a.rs".into());
        }
        assert!(opened.has_open_file());
        assert_eq!(opened.back_to_one(), None);
        assert!(opened.close_file());
        assert_eq!(opened, picked);
        assert_eq!(opened.back_to_one(), Some(at(3)));
        assert!(Rc::ptr_eq(&opened.picked(), opened.several().expect("several")));
    }

    #[test]
    fn an_empty_history_has_nowhere_to_step() {
        let history = linear_history(&[], None);
        assert_eq!(
            Target::WorkingTree { open: None }.step(Step::Older, &history, true),
            None
        );
        assert_eq!(Target::Nothing.step(Step::Newer, &history, true), None);
    }
}
