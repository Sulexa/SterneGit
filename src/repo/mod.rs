mod branch_dialog;
mod branch_menu;
mod changes_list;
mod commit_files;
mod commit_form;
mod commit_panel;
mod compare;
mod file_diff_panel;
mod file_tree;
mod history_list;
mod hunk_actions;
mod navigation;
mod operation;
mod patch_view;
mod ref_tree;
mod refs_list;
mod repository;
mod status_bar;
mod toolbar;

use std::{path::PathBuf, sync::Arc};

use gix::ObjectId;
use gpui_kit::{
    AnyElement, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, ScrollStrategy, SharedString, Subscription,
    UniformListScrollHandle, Window,
    component::{
        WindowExt as _,
        button::{Button, ButtonVariant},
        diff::DiffFile,
        notification::Notification,
        resizable::{h_resizable, resizable_panel},
        v_flex,
    },
    div,
};

use crate::{
    actions::{
        CloseDiff, CopyHash, Fetch, NewerCommit, NextChange, NextFile, OlderCommit, PreviousChange,
        PreviousFile, Refresh, WholeWindow,
    },
    git::{FileChange, HISTORY_LIMIT, History, PullMode, RefLabel, Scope, Side, short_id},
    ui::{self, Failure, FileView, Folded, Look as _, Notice, thousands},
};
use branch_dialog::open_branch_dialog;
use branch_menu::{BranchAction, BranchMenu, confirm_delete};
use changes_list::{ChangesList, SideFolds, SideScrolls, discard_question};
use commit_form::CommitForm;
use commit_panel::{CommitPanel, OpenCommitFile, copy_commit_hash};
use compare::{Direction, FilePlace, Position};
use file_diff_panel::{FileDiffEvent, FileDiffPanel};
use file_tree::{FileTreeRow, file_rows};
use history_list::HistoryList;
use navigation::{OpenFile, Step, Target};
use operation::Operation;
use ref_tree::RefGroup;
use refs_list::RefsList;
pub(crate) use repository::{Repository, RepositoryEvent};
use status_bar::RepoStatusBar;
use toolbar::{RepoToolbar, ToolbarState, notification};

#[derive(Clone)]
enum Openable {
    Working(Side, FileChange),
    Commit(DiffFile),
}

pub(crate) struct RepoView {
    repository: Entity<Repository>,
    target: Target,
    folded_refs: Folded<RefGroup>,
    folded_changes: SideFolds,
    commit_panel: Entity<CommitPanel>,
    commit_form: Entity<CommitForm>,
    file_diff: Entity<FileDiffPanel>,
    whole_window: bool,
    middle_focus: FocusHandle,
    history_scroll: UniformListScrollHandle,
    refs_scroll: UniformListScrollHandle,
    changes_scrolls: SideScrolls,
    _subscriptions: [Subscription; 4],
}

impl RepoView {
    pub(crate) fn new(path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let repository = cx.new(|_| Repository::new(path));
        let commit_panel = cx.new(|_| CommitPanel::new());
        let commit_form = cx.new(|cx| CommitForm::new(repository.clone(), window, cx));
        let file_diff = cx.new(|cx| FileDiffPanel::new(repository.clone(), cx));
        let subscriptions = [
            cx.subscribe_in(&repository, window, |this, _, event, window, cx| match event {
                RepositoryEvent::HistoryRead => this.on_history_read(cx),
                RepositoryEvent::ChangesRead => this.on_changes_read(cx),
                RepositoryEvent::Committed => {}
                RepositoryEvent::Operated(operated) => {
                    window.push_notification(notification(operated), cx)
                }
                RepositoryEvent::PullModeNotSaved(failure) => window.push_notification(
                    Notification::error(failure.message())
                        .title("Couldn't save the pull mode")
                        .autohide(false),
                    cx,
                ),
            }),
            cx.subscribe(&file_diff, |this, _, event: &FileDiffEvent, cx| match event {
                FileDiffEvent::Close => this.close_file(cx),
                FileDiffEvent::WholeWindow => this.toggle_whole_window(cx),
                FileDiffEvent::StepFile(direction) => this.step_file(*direction, cx),
            }),
            cx.subscribe(&commit_panel, |this, _, OpenCommitFile(file), cx| {
                this.open_commit_file(file.clone(), cx)
            }),
            cx.observe(&commit_panel, |this, _, cx| this.place_file(cx)),
        ];
        Self {
            repository,
            target: Target::Nothing,
            folded_refs: Folded::default(),
            folded_changes: SideFolds::default(),
            commit_panel,
            commit_form,
            file_diff,
            whole_window: false,
            middle_focus: cx.focus_handle().tab_stop(true),
            history_scroll: UniformListScrollHandle::new(),
            refs_scroll: UniformListScrollHandle::new(),
            changes_scrolls: SideScrolls::default(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn repository(&self) -> &Entity<Repository> {
        &self.repository
    }

    pub(crate) fn focus_handle(&self) -> FocusHandle {
        self.middle_focus.clone()
    }

    pub(crate) fn copy_hash(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.target.commit() {
            copy_commit_hash(id, window, cx);
        }
    }

    fn on_history_read(&mut self, cx: &mut Context<Self>) {
        let repository = self.repository.read(cx);
        let changes = repository.changes().map(Arc::as_ref);
        let surviving = match repository.history() {
            Some(history) => self.target.after_history_read(history, changes),
            None => Target::Nothing,
        };
        match surviving {
            Target::Commit { id, .. } => self.show_commit(id, cx),
            Target::Nothing => {
                self.target = Target::Nothing;
                self.commit_panel.update(cx, |panel, cx| panel.clear(cx));
                self.file_diff.update(cx, |panel, cx| panel.close(cx));
            }
            working_tree => self.target = working_tree,
        }
        cx.notify();
    }

    fn on_changes_read(&mut self, cx: &mut Context<Self>) {
        let repository = self.repository.read(cx);
        let (Some(repo), Some(history), Some(changes)) = (
            repository.repo().cloned(),
            repository.history().cloned(),
            repository.changes().cloned(),
        ) else {
            return;
        };
        let target = self.target.after_changes_read(&changes, &history);
        match target {
            Target::Commit { id, .. } if self.target.commit() != Some(id) => {
                self.show_commit(id, cx)
            }
            Target::WorkingTree { open } => {
                let followed = open.as_ref().and_then(|file| {
                    let change = changes.find(file.side, &file.path)?;
                    Some((file.side, change.clone()))
                });
                self.target = Target::WorkingTree { open };
                self.file_diff.update(cx, |panel, cx| match followed {
                    Some((side, change)) => panel.open_working_file(&repo, change, side, cx),
                    None => panel.close(cx),
                });
            }
            target => self.target = target,
        }
        self.place_file(cx);
        cx.notify();
    }

    fn select_commit(&mut self, id: ObjectId, cx: &mut Context<Self>) {
        if self.target.commit() == Some(id) {
            self.close_file(cx);
        } else {
            self.show_commit(id, cx);
        }
    }

    fn show_commit(&mut self, id: ObjectId, cx: &mut Context<Self>) {
        let repository = self.repository.read(cx);
        let (Some(repo), Some(history)) =
            (repository.repo().cloned(), repository.history().cloned())
        else {
            return;
        };
        let Some(row) = history.row_of(&id) else {
            return;
        };
        if self.target.commit() != Some(id) {
            self.target = Target::at_commit(id);
            self.file_diff.update(cx, |panel, cx| panel.close(cx));
        }
        self.commit_panel
            .update(cx, |panel, cx| panel.show(&repo, &history.commits[row], cx));
        self.history_scroll
            .scroll_to_item(row, ScrollStrategy::Nearest);
        cx.notify();
    }

    fn select_working_tree(&mut self, cx: &mut Context<Self>) {
        if !self.target.is_working_tree() {
            self.close_file(cx);
            self.target = Target::WorkingTree { open: None };
            cx.notify();
        }
    }

    fn open_working_file(&mut self, side: Side, change: FileChange, cx: &mut Context<Self>) {
        let Some(repo) = self.repository.read(cx).repo().cloned() else {
            return;
        };
        self.target = Target::WorkingTree {
            open: Some(OpenFile {
                side,
                path: change.path.clone(),
            }),
        };
        self.file_diff.update(cx, |panel, cx| {
            panel.open_working_file(&repo, change, side, cx)
        });
        self.place_file(cx);
        cx.notify();
    }

    fn open_commit_file(&mut self, file: DiffFile, cx: &mut Context<Self>) {
        let Target::Commit { id, open } = &mut self.target else {
            return;
        };
        let path = file.path().clone();
        *open = Some(path.clone());
        let id = *id;
        let parent = self
            .repository
            .read(cx)
            .history()
            .and_then(|history| history.row_of(&id).map(|row| history.commits[row].parents.first().copied()))
            .flatten();
        self.commit_panel
            .update(cx, |panel, cx| panel.mark_open(Some(path), cx));
        self.file_diff.update(cx, |panel, cx| {
            panel.open_commit_file(id, parent, short_id(id), file, cx)
        });
        self.place_file(cx);
        cx.notify();
    }

    fn toggle_whole_window(&mut self, cx: &mut Context<Self>) {
        self.whole_window = !self.whole_window;
        self.place_file(cx);
        cx.notify();
    }

    fn files_in_order(&self, cx: &Context<Self>) -> Vec<Openable> {
        let view = FileView::current(cx);
        let unfolded = Folded::default();
        match &self.target {
            Target::WorkingTree { .. } => {
                let Some(changes) = self.repository.read(cx).changes().cloned() else {
                    return Vec::new();
                };
                [Side::Staged, Side::Unstaged]
                    .into_iter()
                    .flat_map(|side| {
                        let files = changes.files(side);
                        file_rows(view, files.iter().map(|change| change.path.as_ref()), &unfolded)
                            .into_iter()
                            .filter_map(|row| match row {
                                FileTreeRow::File { ix, .. } => {
                                    Some(Openable::Working(side, files[ix].clone()))
                                }
                                FileTreeRow::Folder { .. } => None,
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect()
            }
            Target::Commit { .. } => {
                let Some(files) = self.commit_panel.read(cx).files() else {
                    return Vec::new();
                };
                file_rows(view, files.iter().map(|file| file.path().as_ref()), &unfolded)
                    .into_iter()
                    .filter_map(|row| match row {
                        FileTreeRow::File { ix, .. } => Some(Openable::Commit(files[ix].clone())),
                        FileTreeRow::Folder { .. } => None,
                    })
                    .collect()
            }
            Target::Nothing => Vec::new(),
        }
    }

    fn open_position(&self, files: &[Openable]) -> Option<usize> {
        files.iter().position(|file| match (file, &self.target) {
            (Openable::Working(side, change), Target::WorkingTree { open: Some(open) }) => {
                *side == open.side && change.path == open.path
            }
            (Openable::Commit(file), Target::Commit { open: Some(path), .. }) => file.path() == path,
            _ => false,
        })
    }

    fn place_file(&mut self, cx: &mut Context<Self>) {
        let files = self.files_in_order(cx);
        let position = self.open_position(&files).map(|index| Position {
            index,
            count: files.len(),
        });
        let place = FilePlace {
            whole_window: self.whole_window,
            position,
        };
        self.file_diff.update(cx, |panel, cx| panel.place(place, cx));
    }

    fn step_file(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let files = self.files_in_order(cx);
        let Some(current) = self.open_position(&files) else {
            return;
        };
        let next = match direction {
            Direction::Back => current.checked_sub(1),
            Direction::Forward => Some(current + 1).filter(|next| *next < files.len()),
        };
        match next.and_then(|next| files.get(next)).cloned() {
            Some(Openable::Working(side, change)) => self.open_working_file(side, change, cx),
            Some(Openable::Commit(file)) => self.open_commit_file(file, cx),
            None => {}
        }
    }

    fn step_change(&mut self, direction: Direction, cx: &mut Context<Self>) {
        if self.target.has_open_file() {
            self.file_diff
                .update(cx, |panel, cx| panel.step_change(direction, cx));
        }
    }

    fn close_file(&mut self, cx: &mut Context<Self>) {
        if self.target.close_file() {
            self.commit_panel
                .update(cx, |panel, cx| panel.mark_open(None, cx));
            self.file_diff.update(cx, |panel, cx| panel.close(cx));
            cx.notify();
        }
    }

    fn step(&mut self, step: Step, cx: &mut Context<Self>) {
        let repository = self.repository.read(cx);
        let Some(history) = repository.history() else {
            return;
        };
        match self.target.step(step, history, repository.has_changes()) {
            Some(Target::Commit { id, .. }) => self.select_commit(id, cx),
            Some(Target::WorkingTree { .. }) => self.select_working_tree(cx),
            Some(Target::Nothing) | None => {}
        }
    }

    fn reveal_ref(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(history) = self.repository.read(cx).history().cloned() else {
            return;
        };
        let Some(label) = history.refs.get(ix) else {
            return;
        };
        match history.row_of(&label.target) {
            Some(row) => {
                self.select_commit(label.target, cx);
                self.history_scroll
                    .scroll_to_item(row, ScrollStrategy::Center);
            }
            None => window.push_notification(
                format!(
                    "{} points to a commit older than the newest {} shown.",
                    label.name,
                    thousands(HISTORY_LIMIT)
                ),
                cx,
            ),
        }
    }

    fn toggle_ref_fold(&mut self, group: RefGroup, cx: &mut Context<Self>) {
        self.folded_refs.toggle(group);
        cx.notify();
    }

    fn toggle_change_fold(&mut self, side: Side, folder: SharedString, cx: &mut Context<Self>) {
        self.folded_changes.of_mut(side).toggle(folder);
        cx.notify();
    }

    fn move_files(&mut self, from: Side, scope: Scope, cx: &mut Context<Self>) {
        self.repository
            .update(cx, |repository, cx| repository.move_across(from, scope, cx));
    }

    fn confirm_discard(&mut self, change: FileChange, window: &mut Window, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        let question = discard_question(&change);
        window.open_alert_dialog(cx, move |alert, _, _| {
            let repository = repository.clone();
            let change = change.clone();
            alert
                .confirm()
                .title(question.title.clone())
                .description(question.description.clone())
                .ok_text(question.confirm)
                .ok_variant(ButtonVariant::Danger)
                .on_ok(move |_, _, cx| {
                    repository.update(cx, |repository, cx| repository.discard(change.clone(), cx));
                    true
                })
        });
    }

    fn run(&mut self, operation: Operation, window: &mut Window, cx: &mut Context<Self>) {
        let repository = self.repository.clone();
        match operation {
            Operation::Branch => open_branch_dialog(repository, None, window, cx),
            Operation::Pull => repository.update(cx, |repository, cx| repository.pull(cx)),
            Operation::Push => repository.update(cx, |repository, cx| repository.push(cx)),
            Operation::Stash => repository.update(cx, |repository, cx| repository.stash(cx)),
            Operation::Pop => repository.update(cx, |repository, cx| repository.pop_stash(cx)),
        }
    }

    fn act_on_branch(
        &mut self,
        label: RefLabel,
        action: BranchAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let repository = self.repository.clone();
        match action {
            BranchAction::CheckOut => {
                repository.update(cx, |repository, cx| repository.check_out(label, cx))
            }
            BranchAction::Pull => {
                repository.update(cx, |repository, cx| repository.pull_branch(label, cx))
            }
            BranchAction::Push => {
                repository.update(cx, |repository, cx| repository.push_branch(label, cx))
            }
            BranchAction::Merge => {
                repository.update(cx, |repository, cx| repository.merge(label, cx))
            }
            BranchAction::Rebase => {
                repository.update(cx, |repository, cx| repository.rebase_onto(label, cx))
            }
            BranchAction::BranchHere => {
                let start = (!label.is_head).then_some(label);
                open_branch_dialog(repository, start, window, cx)
            }
            BranchAction::Delete => confirm_delete(repository, label, window, cx),
        }
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> RepoToolbar {
        RepoToolbar::new(ToolbarState::of(self.repository.read(cx)))
            .on_run(cx.listener(|this, operation: &Operation, window, cx| {
                this.run(*operation, window, cx)
            }))
            .on_pull_mode(cx.listener(|this, mode: &PullMode, _, cx| {
                this.repository
                    .update(cx, |repository, cx| repository.choose_pull_mode(*mode, cx))
            }))
    }

    fn render_body(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let repository = self.repository.read(cx);
        let Some(history) = repository.history().cloned() else {
            return if repository.failure().is_none() {
                ui::loading("Reading history…")
            } else {
                self.render_failure(cx)
            };
        };
        if self.whole_window && self.target.has_open_file() {
            return div()
                .pane()
                .track_focus(&self.middle_focus)
                .child(self.file_diff.clone())
                .into_any_element();
        }
        let rem = window.rem_size();
        h_resizable("fastgit-panes")
            .child(
                resizable_panel()
                    .size(rem * 14.)
                    .size_range(rem * 10. ..rem * 30.)
                    .child(self.refs_list(history.clone(), cx)),
            )
            .child(resizable_panel().size_range(rem * 30. ..rem * 400.).child(
                if self.target.has_open_file() {
                    div()
                        .pane()
                        .track_focus(&self.middle_focus)
                        .child(self.file_diff.clone())
                        .into_any_element()
                } else {
                    self.history_list(history, cx).into_any_element()
                },
            ))
            .child(
                resizable_panel()
                    .size(rem * 30.)
                    .size_range(rem * 22. ..rem * 90.)
                    .child(match &self.target {
                        Target::WorkingTree { open } => {
                            self.changes_list(open.clone(), cx).into_any_element()
                        }
                        Target::Commit { .. } => self.commit_panel.clone().into_any_element(),
                        Target::Nothing => ui::hint("Select a commit to see its changes.", cx),
                    }),
            )
            .into_any_element()
    }

    fn render_failure(&self, cx: &mut Context<Self>) -> AnyElement {
        let reason = self
            .repository
            .read(cx)
            .failure()
            .map(Failure::message)
            .unwrap_or_default();
        Notice::new(
            "Couldn't open this repository",
            reason,
            Button::new("retry")
                .label("Retry")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.repository
                        .update(cx, |repository, cx| repository.refresh(cx))
                })),
        )
        .into_any_element()
    }

    fn refs_list(&self, history: Arc<History>, cx: &mut Context<Self>) -> RefsList {
        let repository = self.repository.read(cx);
        let branch_menu = BranchMenu::new(
            history.branch.clone(),
            repository.pull_mode(),
            repository.is_writing(),
        )
        .on_action(cx.listener(
            |this, (label, action): &(RefLabel, BranchAction), window, cx| {
                this.act_on_branch(label.clone(), *action, window, cx)
            },
        ));
        RefsList::new(history, &self.folded_refs, self.refs_scroll.clone())
            .selected(self.target.commit())
            .branch_menu(branch_menu)
            .on_reveal(cx.listener(|this, ix: &usize, window, cx| this.reveal_ref(*ix, window, cx)))
            .on_toggle(
                cx.listener(|this, group: &RefGroup, _, cx| {
                    this.toggle_ref_fold(group.clone(), cx)
                }),
            )
    }

    fn history_list(&self, history: Arc<History>, cx: &mut Context<Self>) -> HistoryList {
        let changes = self.repository.read(cx).changes().cloned();
        HistoryList::new(
            history,
            self.middle_focus.clone(),
            self.history_scroll.clone(),
        )
        .selected(self.target.commit())
        .uncommitted(changes, self.target.is_working_tree())
        .on_select_commit(cx.listener(|this, id: &ObjectId, _, cx| this.select_commit(*id, cx)))
        .on_select_uncommitted(cx.listener(|this, _: &(), _, cx| this.select_working_tree(cx)))
    }

    fn changes_list(&self, open: Option<OpenFile>, cx: &mut Context<Self>) -> ChangesList {
        let repository = self.repository.read(cx);
        ChangesList::new(
            repository.changes().cloned(),
            self.commit_form.clone(),
            FileView::current(cx),
            &self.folded_changes,
            self.changes_scrolls.clone(),
        )
        .root(repository.repo().map(|repo| Arc::from(repo.location())))
        .open(open)
        .writing(repository.is_writing())
        .on_open(
            cx.listener(|this, (side, change): &(Side, FileChange), _, cx| {
                this.open_working_file(*side, change.clone(), cx)
            }),
        )
        .on_move(cx.listener(|this, (side, scope): &(Side, Scope), _, cx| {
            this.move_files(*side, scope.clone(), cx)
        }))
        .on_discard(cx.listener(|this, change: &FileChange, window, cx| {
            this.confirm_discard(change.clone(), window, cx)
        }))
        .on_fold(
            cx.listener(|this, (side, folder): &(Side, SharedString), _, cx| {
                this.toggle_change_fold(*side, folder.clone(), cx)
            }),
        )
    }
}

impl Render for RepoView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_bar = RepoStatusBar::new(self.repository.read(cx), self.target.commit());
        v_flex()
            .pane()
            .on_action(cx.listener(|this, _: &OlderCommit, _, cx| this.step(Step::Older, cx)))
            .on_action(cx.listener(|this, _: &NewerCommit, _, cx| this.step(Step::Newer, cx)))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| {
                this.repository
                    .update(cx, |repository, cx| repository.refresh(cx))
            }))
            .on_action(cx.listener(|this, _: &Fetch, _, cx| {
                this.repository
                    .update(cx, |repository, cx| repository.fetch(cx))
            }))
            .on_action(cx.listener(|this, _: &CopyHash, window, cx| this.copy_hash(window, cx)))
            .on_action(cx.listener(|this, _: &CloseDiff, _, cx| this.close_file(cx)))
            .on_action(cx.listener(|this, _: &PreviousChange, _, cx| {
                this.step_change(Direction::Back, cx)
            }))
            .on_action(cx.listener(|this, _: &NextChange, _, cx| {
                this.step_change(Direction::Forward, cx)
            }))
            .on_action(cx.listener(|this, _: &PreviousFile, _, cx| {
                this.step_file(Direction::Back, cx)
            }))
            .on_action(cx.listener(|this, _: &NextFile, _, cx| {
                this.step_file(Direction::Forward, cx)
            }))
            .on_action(cx.listener(|this, _: &WholeWindow, _, cx| {
                this.toggle_whole_window(cx)
            }))
            .child(self.toolbar(cx))
            .child(div().fill_height().child(self.render_body(window, cx)))
            .child(status_bar)
    }
}
