use std::{collections::HashSet, sync::Arc};

use gix::ObjectId;
use gpui_kit::{
    App, ClipboardItem, Context, Entity, Window,
    component::{
        WindowExt as _,
        button::ButtonVariant,
        menu::{PopupMenu, PopupMenuItem},
    },
};

use super::repository::Repository;
use crate::{
    git::{ChangeKind, History, ResetMode, WorkingChanges, short_id},
    ui::{Handler, counted, handler},
};

const CHECK_OUT: &str = "Check out this commit";
const BRANCH_HERE: &str = "Create branch here…";
const TAG_HERE: &str = "Create tag here…";
const REVERT: &str = "Revert this commit";
const REVERT_MERGE: &str = "Revert this merge";
const COPY_HASH: &str = "Copy hash";
const COPY_MESSAGE: &str = "Copy message";
const MESSAGE_COPIED: &str = "Commit message copied";
const DETACHED: &str = "HEAD";
const RESET: &str = "Reset";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommitAction {
    CheckOut,
    BranchHere,
    TagHere,
    Revert,
    CherryPick,
    Reset(ResetMode),
    CopyHash,
    CopyMessage,
}

impl CommitAction {
    fn available(self, writing: bool, underway: bool) -> bool {
        match self {
            CommitAction::CopyHash | CommitAction::CopyMessage => true,
            CommitAction::TagHere => !writing,
            _ => !writing && !underway,
        }
    }

    fn text(self, branch: &str, merge: bool) -> String {
        match self {
            CommitAction::CheckOut => CHECK_OUT.to_owned(),
            CommitAction::BranchHere => BRANCH_HERE.to_owned(),
            CommitAction::TagHere => TAG_HERE.to_owned(),
            CommitAction::Revert if merge => REVERT_MERGE.to_owned(),
            CommitAction::Revert => REVERT.to_owned(),
            CommitAction::CherryPick => format!("Cherry-pick onto {branch}"),
            CommitAction::Reset(ResetMode::Soft) => "Soft: keep changes staged".to_owned(),
            CommitAction::Reset(ResetMode::Mixed) => "Mixed: keep changes unstaged".to_owned(),
            CommitAction::Reset(ResetMode::Hard) => "Hard: discard changes…".to_owned(),
            CommitAction::CopyHash => COPY_HASH.to_owned(),
            CommitAction::CopyMessage => COPY_MESSAGE.to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Action(CommitAction),
    Reset,
}

#[derive(Clone)]
pub(crate) struct CommitMenu {
    history: Arc<History>,
    writing: bool,
    underway: bool,
    on_action: Option<Handler<(ObjectId, CommitAction)>>,
}

impl CommitMenu {
    pub(crate) fn new(history: Arc<History>, writing: bool, underway: bool) -> Self {
        Self {
            history,
            writing,
            underway,
            on_action: None,
        }
    }

    pub(crate) fn on_action(
        mut self,
        callback: impl Fn(&(ObjectId, CommitAction), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(handler(callback));
        self
    }

    pub(crate) fn build(
        &self,
        menu: PopupMenu,
        id: ObjectId,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        let branch = self.branch();
        let merge = self.is_merge(&id);
        let mut menu = menu;
        for (ix, group) in self.offered(&id).into_iter().enumerate() {
            if ix > 0 {
                menu = menu.separator();
            }
            for entry in group {
                menu = match entry {
                    Entry::Action(action) => menu.item(self.item(action, id, &branch, merge)),
                    Entry::Reset => {
                        let modes = self.clone();
                        let to = branch.clone();
                        menu.submenu(reset_label(&branch), window, cx, move |submenu, _, _| {
                            ResetMode::ALL.into_iter().fold(submenu, |submenu, mode| {
                                submenu.item(modes.item(CommitAction::Reset(mode), id, &to, merge))
                            })
                        })
                    }
                };
            }
        }
        menu
    }

    fn branch(&self) -> String {
        self.history
            .branch
            .as_deref()
            .unwrap_or(DETACHED)
            .to_owned()
    }

    fn is_merge(&self, id: &ObjectId) -> bool {
        self.history
            .row_of(id)
            .is_some_and(|row| self.history.commits[row].parents.len() > 1)
    }

    fn offered(&self, id: &ObjectId) -> Vec<Vec<Entry>> {
        use CommitAction::*;
        let at_head = self.history.head_id == Some(*id);
        let undo = if self.history.head_holds(id) {
            Some(Revert)
        } else if self.is_merge(id) {
            None
        } else {
            Some(CherryPick)
        };
        let change_branch: Vec<Entry> = undo
            .map(Entry::Action)
            .into_iter()
            .chain((!at_head).then_some(Entry::Reset))
            .collect();
        [
            (!at_head).then(|| vec![Entry::Action(CheckOut)]),
            Some(vec![Entry::Action(BranchHere), Entry::Action(TagHere)]),
            Some(change_branch),
            Some(vec![Entry::Action(CopyHash), Entry::Action(CopyMessage)]),
        ]
        .into_iter()
        .flatten()
        .filter(|group| !group.is_empty())
        .collect()
    }

    fn item(&self, action: CommitAction, id: ObjectId, branch: &str, merge: bool) -> PopupMenuItem {
        let on_action = self.on_action.clone();
        let chosen = (id, action);
        PopupMenuItem::new(action.text(branch, merge))
            .disabled(!action.available(self.writing, self.underway) || on_action.is_none())
            .on_click(move |_, window, cx| {
                if let Some(on_action) = &on_action {
                    on_action(&chosen, window, cx)
                }
            })
    }
}

fn reset_label(branch: &str) -> String {
    format!("{RESET} {branch} to here")
}

pub(crate) fn copy_commit_message(
    repository: &Entity<Repository>,
    id: ObjectId,
    window: &mut Window,
    cx: &mut App,
) {
    let message = repository.read(cx).repo().map(|repo| repo.message(id));
    match message {
        Some(Ok(message)) => {
            cx.write_to_clipboard(ClipboardItem::new_string(message));
            window.push_notification(MESSAGE_COPIED, cx);
        }
        Some(Err(error)) => window.push_notification(error.to_string(), cx),
        None => {}
    }
}

fn tracked_changes(changes: &WorkingChanges) -> usize {
    changes
        .conflicted
        .iter()
        .chain(&changes.staged)
        .chain(&changes.unstaged)
        .filter(|change| change.kind != ChangeKind::Untracked)
        .map(|change| &change.path)
        .collect::<HashSet<_>>()
        .len()
}

struct ResetQuestion {
    title: String,
    description: String,
}

fn reset_hard_question(branch: &str, id: ObjectId, files: usize, lost: Option<usize>) -> ResetQuestion {
    let files = match files {
        0 => "Every tracked file goes back to that commit. There are no uncommitted changes to lose."
            .to_owned(),
        files => format!(
            "Every tracked file goes back to that commit: the uncommitted changes in {} are thrown away, and Git keeps no copy of them. Untracked files stay.",
            counted(files, "file", "files")
        ),
    };
    let lost = match lost {
        Some(0) | None => String::new(),
        Some(1) => " 1 commit is on no other branch or tag, so it leaves the graph.".to_owned(),
        Some(lost) => format!(
            " {} are on no other branch or tag, so they leave the graph.",
            counted(lost, "commit", "commits")
        ),
    };
    ResetQuestion {
        title: format!("{RESET} {branch} to {} and discard changes?", short_id(id)),
        description: format!("{files}{lost}"),
    }
}

pub(crate) fn confirm_reset_hard(
    repository: Entity<Repository>,
    id: ObjectId,
    window: &mut Window,
    cx: &mut App,
) {
    let (history, files) = {
        let repository = repository.read(cx);
        let Some(history) = repository.history().cloned() else {
            return;
        };
        let files = repository.changes().map_or(0, |changes| tracked_changes(changes));
        (history, files)
    };
    let branch = history.branch.as_deref().unwrap_or(DETACHED);
    let question = reset_hard_question(branch, id, files, history.commits_left_by_reset(&id));
    window.open_alert_dialog(cx, move |alert, _, _| {
        let repository = repository.clone();
        alert
            .confirm()
            .title(question.title.clone())
            .description(question.description.clone())
            .ok_text(RESET)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, _, cx| {
                repository.update(cx, |repository, cx| {
                    repository.reset(id, ResetMode::Hard, cx)
                });
                true
            })
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use gix::ObjectId;

    use super::{CommitAction, CommitMenu, Entry, reset_hard_question};
    use crate::git::{ResetMode, fixtures::id, scratch::ScratchRepo};

    fn texts(menu: &CommitMenu, id: &ObjectId) -> Vec<Vec<String>> {
        let branch = menu.branch();
        let merge = menu.is_merge(id);
        menu.offered(id)
            .into_iter()
            .map(|group| {
                group
                    .into_iter()
                    .map(|entry| match entry {
                        Entry::Action(action) => action.text(&branch, merge),
                        Entry::Reset => super::reset_label(&branch),
                    })
                    .collect()
            })
            .collect()
    }

    fn rev(scratch: &ScratchRepo, revision: &str) -> Result<ObjectId> {
        Ok(ObjectId::from_hex(scratch.git(&["rev-parse", revision])?.trim().as_bytes())?)
    }

    fn graph(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        scratch.commit("first")?;
        scratch.git(&["switch", "--quiet", "--create=topic"])?;
        scratch.commit("topic")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        scratch.commit("second")?;
        scratch.git(&["merge", "--quiet", "--no-ff", "--no-edit", "topic"])?;
        scratch.git(&["switch", "--quiet", "--create=other", "main~1"])?;
        scratch.commit("other")?;
        scratch.git(&["switch", "--quiet", "main"])?;
        Ok(scratch)
    }

    fn menu(scratch: &ScratchRepo) -> Result<CommitMenu> {
        let (_, history) = scratch.open()?;
        Ok(CommitMenu::new(Arc::new(history), false, false))
    }

    #[test]
    fn a_commit_the_branch_holds_is_reverted_and_the_branch_reset_to_it() -> Result<()> {
        let scratch = graph("commit-menu-held")?;
        assert_eq!(
            texts(&menu(&scratch)?, &rev(&scratch, "main~1")?),
            [
                vec!["Check out this commit"],
                vec!["Create branch here…", "Create tag here…"],
                vec!["Revert this commit", "Reset main to here"],
                vec!["Copy hash", "Copy message"],
            ]
        );
        Ok(())
    }

    #[test]
    fn a_commit_of_another_branch_is_cherry_picked() -> Result<()> {
        let scratch = graph("commit-menu-other")?;
        let texts = texts(&menu(&scratch)?, &rev(&scratch, "other")?);
        assert_eq!(texts[2], ["Cherry-pick onto main", "Reset main to here"]);
        Ok(())
    }

    #[test]
    fn the_head_commit_has_no_check_out_and_no_reset_and_a_merge_reverts_as_one() -> Result<()> {
        let scratch = graph("commit-menu-head")?;
        assert_eq!(
            texts(&menu(&scratch)?, &rev(&scratch, "main")?),
            [
                vec!["Create branch here…", "Create tag here…"],
                vec!["Revert this merge"],
                vec!["Copy hash", "Copy message"],
            ]
        );
        Ok(())
    }

    #[test]
    fn a_detached_head_names_head() -> Result<()> {
        let scratch = graph("commit-menu-detached")?;
        scratch.git(&["switch", "--quiet", "--detach", "main"])?;
        let texts = texts(&menu(&scratch)?, &rev(&scratch, "other")?);
        assert_eq!(texts[2], ["Cherry-pick onto HEAD", "Reset HEAD to here"]);
        Ok(())
    }

    #[test]
    fn only_tags_and_copies_wait_for_nothing_while_an_operation_stops() {
        use CommitAction::*;
        let open = |writing, underway| {
            [CheckOut, BranchHere, TagHere, Revert, CherryPick, Reset(ResetMode::Soft), CopyHash, CopyMessage]
                .into_iter()
                .filter(|action| action.available(writing, underway))
                .collect::<Vec<_>>()
        };
        assert_eq!(open(false, true), [TagHere, CopyHash, CopyMessage]);
        assert_eq!(open(true, false), [CopyHash, CopyMessage]);
        assert_eq!(open(false, false).len(), 8);
    }

    #[test]
    fn the_reset_question_counts_files_and_lost_commits() {
        let question = reset_hard_question("main", id(1), 5, Some(1));
        assert_eq!(question.title, format!("Reset main to {} and discard changes?", crate::git::short_id(id(1))));
        assert_eq!(
            question.description,
            "Every tracked file goes back to that commit: the uncommitted changes in 5 files are thrown away, and Git keeps no copy of them. Untracked files stay. 1 commit is on no other branch or tag, so it leaves the graph."
        );
        assert_eq!(
            reset_hard_question("main", id(1), 0, Some(0)).description,
            "Every tracked file goes back to that commit. There are no uncommitted changes to lose."
        );
    }
}
