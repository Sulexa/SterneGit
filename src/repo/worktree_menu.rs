use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Window,
    component::{
        ActiveTheme as _, Disableable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        dialog::{DialogAction, DialogClose, DialogDescription, DialogFooter},
        input::{Input, InputState},
        menu::{PopupMenu, PopupMenuItem},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::repository::Repository;
use crate::{
    git::{History, Worktree},
    ui::{Handler, Look as _, handler},
};

const WIDTH: f32 = 440.;
const LOCK_PLACEHOLDER: &str = "Reason (optional)";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorktreeAction {
    OpenInTab,
    ShowInFolder,
    CopyPath,
    Lock,
    Unlock,
    Remove,
    Forget,
}

impl WorktreeAction {
    fn text(self) -> &'static str {
        match self {
            WorktreeAction::OpenInTab => "Open in a new tab",
            WorktreeAction::ShowInFolder => "Show in folder",
            WorktreeAction::CopyPath => "Copy path",
            WorktreeAction::Lock => "Lock…",
            WorktreeAction::Unlock => "Unlock",
            WorktreeAction::Remove => "Remove worktree…",
            WorktreeAction::Forget => "Forget this worktree",
        }
    }

    fn writes(self) -> bool {
        matches!(
            self,
            WorktreeAction::Lock | WorktreeAction::Unlock | WorktreeAction::Remove | WorktreeAction::Forget
        )
    }
}

#[derive(Clone)]
pub(crate) struct WorktreeMenu {
    writing: bool,
    on_action: Option<Handler<(Worktree, WorktreeAction)>>,
}

impl WorktreeMenu {
    pub(crate) fn new(writing: bool) -> Self {
        Self {
            writing,
            on_action: None,
        }
    }

    pub(crate) fn on_action(
        mut self,
        callback: impl Fn(&(Worktree, WorktreeAction), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(handler(callback));
        self
    }

    pub(crate) fn build(&self, menu: PopupMenu, worktree: &Worktree) -> PopupMenu {
        offered(worktree)
            .into_iter()
            .enumerate()
            .fold(menu, |menu, (ix, group)| {
                let menu = if ix > 0 { menu.separator() } else { menu };
                group
                    .into_iter()
                    .fold(menu, |menu, action| menu.item(self.item(action, worktree)))
            })
    }

    fn item(&self, action: WorktreeAction, worktree: &Worktree) -> PopupMenuItem {
        let on_action = self.on_action.clone();
        let chosen = (worktree.clone(), action);
        PopupMenuItem::new(action.text())
            .disabled((action.writes() && self.writing) || on_action.is_none())
            .on_click(move |_, window, cx| {
                if let Some(on_action) = &on_action {
                    on_action(&chosen, window, cx)
                }
            })
    }
}

fn offered(worktree: &Worktree) -> Vec<Vec<WorktreeAction>> {
    use WorktreeAction::*;
    if worktree.is_missing {
        return vec![vec![Forget], vec![CopyPath]];
    }
    let lock = match worktree.is_locked() {
        true => Unlock,
        false => Lock,
    };
    [
        (!worktree.is_here).then(|| vec![OpenInTab]),
        Some(vec![ShowInFolder, CopyPath]),
        worktree.can_be_locked().then(|| vec![lock]),
        worktree.can_be_removed().then(|| vec![Remove]),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub(crate) fn removal_question(worktree: &Worktree, changed: usize) -> String {
    let mut parts = vec![format!("Deletes the folder {}.", worktree.location.display())];
    match changed {
        0 => {}
        1 => parts.push("Its 1 uncommitted change is lost.".to_owned()),
        _ => parts.push(format!("Its {changed} uncommitted changes are lost.")),
    }
    if let Some(branch) = &worktree.branch {
        parts.push(format!("Its commits stay on {branch}."));
    }
    parts.join(" ")
}

struct RemoveForm {
    branch: Option<String>,
    merged: bool,
    delete_branch: bool,
    into: String,
}

impl Render for RemoveForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .gap_1()
            .mt_3()
            .when_some(self.branch.clone(), |this, branch| {
                this.child(
                    Checkbox::new("delete-worktree-branch")
                        .label(format!("Also delete the branch {branch}"))
                        .checked(self.delete_branch)
                        .disabled(!self.merged)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.delete_branch = *checked;
                            cx.notify();
                        })),
                )
                .when(!self.merged, |this| {
                    this.child(
                        div()
                            .pl_6()
                            .xs_text()
                            .text_color(muted)
                            .child(format!("{branch} isn't merged into {}, so it is kept", self.into)),
                    )
                })
            })
    }
}

pub(crate) fn confirm_remove(
    repository: Entity<Repository>,
    worktree: Worktree,
    changed: usize,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(history) = repository.read(cx).history().cloned() else {
        return;
    };
    let merged = merged_into_head(&history, &worktree);
    let form = cx.new(|_| RemoveForm {
        branch: worktree.branch.as_ref().map(ToString::to_string),
        merged,
        delete_branch: false,
        into: history.branch.as_deref().unwrap_or("HEAD").to_owned(),
    });
    let title = format!("Remove the worktree {}?", worktree.name);
    let description = removal_question(&worktree, changed);
    window.open_dialog(cx, move |dialog, _, _| {
        let form_on_ok = form.clone();
        let repository = repository.clone();
        let worktree = worktree.clone();
        dialog
            .title(title.clone())
            .w(px(WIDTH))
            .child(DialogDescription::new().child(description.clone()))
            .child(form.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(
                        DialogAction::new()
                            .child(Button::new("remove-worktree").danger().label("Remove")),
                    ),
            )
            .on_ok(move |_, _, cx| {
                let form = form_on_ok.read(cx);
                let delete_branch = form.branch.clone().filter(|_| form.delete_branch && form.merged);
                let removed = vec![(PathBuf::from(&*worktree.location), worktree.name.clone())];
                repository.update(cx, |repository, cx| {
                    repository.remove_worktrees(removed, delete_branch, cx)
                });
                true
            })
    });
}

pub(crate) fn merged_into_head(history: &History, worktree: &Worktree) -> bool {
    worktree.head.is_some_and(|head| history.head_holds(&head))
}

pub(crate) fn open_lock_dialog(
    repository: Entity<Repository>,
    worktree: Worktree,
    window: &mut Window,
    cx: &mut App,
) {
    let reason = cx.new(|cx| InputState::new(window, cx).placeholder(LOCK_PLACEHOLDER));
    let focused = reason.clone();
    let title = format!("Lock the worktree {}", worktree.name);
    window.open_dialog(cx, move |dialog, _, _| {
        let reason_on_ok = reason.clone();
        let repository = repository.clone();
        let worktree = worktree.clone();
        dialog
            .title(title.clone())
            .w(px(WIDTH))
            .child(DialogDescription::new().child(
                "A locked worktree is never forgotten by git worktree prune, even while its folder is on a drive that isn't plugged in, and can't be removed until it is unlocked.",
            ))
            .child(div().mt_3().child(Input::new(&reason)))
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(DialogAction::new().child(Button::new("lock-worktree").primary().label("Lock"))),
            )
            .on_ok(move |_, _, cx| {
                let reason = reason_on_ok.read(cx).value().trim().to_owned();
                let location = PathBuf::from(&*worktree.location);
                repository.update(cx, |repository, cx| {
                    repository.lock_worktree(location, worktree.name.clone(), Some(reason), cx)
                });
                true
            })
    });
    window.defer(cx, move |window, cx| {
        focused.update(cx, |state, cx| state.focus(window, cx))
    });
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use super::{WorktreeAction::*, offered, removal_question};
    use crate::git::{Worktree, fixtures::id};

    fn worktree(name: &str) -> Worktree {
        Worktree {
            location: Arc::from(Path::new("/projects/sternegit-worktrees").join(name)),
            name: name.to_owned().into(),
            branch: Some(format!("feature/{name}").into()),
            head: Some(id(1)),
            is_main: false,
            is_here: false,
            lock: None,
            is_missing: false,
        }
    }

    #[test]
    fn another_worktree_opens_shows_locks_and_removes() {
        assert_eq!(
            offered(&worktree("diff-view")),
            [vec![OpenInTab], vec![ShowInFolder, CopyPath], vec![Lock], vec![Remove]]
        );
    }

    #[test]
    fn the_main_worktree_seen_from_another_tab_can_only_be_opened_and_shown() {
        let main = Worktree {
            is_main: true,
            ..worktree("sternegit")
        };
        assert_eq!(offered(&main), [vec![OpenInTab], vec![ShowInFolder, CopyPath]]);
    }

    #[test]
    fn a_locked_worktree_is_unlocked_before_it_can_be_removed() {
        let locked = Worktree {
            lock: Some("on a stick".into()),
            ..worktree("blame")
        };
        assert_eq!(offered(&locked), [vec![OpenInTab], vec![ShowInFolder, CopyPath], vec![Unlock]]);
    }

    #[test]
    fn this_tabs_worktree_is_neither_opened_nor_removed() {
        let here = Worktree {
            is_here: true,
            ..worktree("diff-view")
        };
        assert_eq!(offered(&here), [vec![ShowInFolder, CopyPath], vec![Lock]]);
    }

    #[test]
    fn a_worktree_whose_folder_is_gone_can_only_be_forgotten() {
        let gone = Worktree {
            is_missing: true,
            ..worktree("stash-ui")
        };
        assert_eq!(offered(&gone), [vec![Forget], vec![CopyPath]]);
    }

    #[test]
    fn the_removal_question_names_the_folder_counts_the_losses_and_keeps_the_commits() {
        let question = removal_question(&worktree("diff-view"), 3);
        assert!(question.ends_with("Its 3 uncommitted changes are lost. Its commits stay on feature/diff-view."));
        assert!(question.starts_with("Deletes the folder "));
    }
}
