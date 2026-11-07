use std::{path::Path, sync::Arc};

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, ClipboardItem, Context, Entity, EventEmitter,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        diff::DiffFile,
        h_flex,
        tooltip::Tooltip,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    commit_files::CommitFiles,
    message_editor::{Field, MessageEditor, StopEditing},
    patch_view::{PatchStats, PatchTexts, parse_patch, patch_content},
    repository::Repository,
};
use crate::{
    actions::{CopyHash, WINDOW_CONTEXT},
    git::{Commit, Repo, RewordScope, short_id},
    ui::{
        self, FileView, FileViewToggle, Folded, Load, Look as _, commit_subject, counted,
        run_in_background,
    },
};

const AVATAR_SIZE: Pixels = px(20.);
const BODY_HEIGHT: Pixels = px(140.);
const EDIT_TIP: &str = "Click to edit the message";

const TEXTS: PatchTexts = PatchTexts {
    loading: "Reading commit…",
    failed: "Couldn't read this commit",
    empty: "This commit doesn't change any files.",
};

struct Summary {
    body: SharedString,
    files: Arc<[DiffFile]>,
    stats: PatchStats,
}

pub(crate) struct OpenCommitFile(pub(crate) DiffFile);

pub(crate) struct ShowFileHistory(pub(crate) DiffFile);

struct Editing {
    view: Entity<MessageEditor>,
    _stop: Subscription,
}

pub(crate) struct CommitPanel {
    repository: Entity<Repository>,
    commit: Option<Commit>,
    root: Option<Arc<Path>>,
    details: Load<Summary>,
    open: Option<SharedString>,
    folded: Folded<SharedString>,
    files_scroll: UniformListScrollHandle,
    scope: Option<RewordScope>,
    editing: Option<Editing>,
}

impl EventEmitter<OpenCommitFile> for CommitPanel {}

impl EventEmitter<ShowFileHistory> for CommitPanel {}

impl CommitPanel {
    pub(crate) fn new(repository: Entity<Repository>) -> Self {
        Self {
            repository,
            commit: None,
            root: None,
            details: Load::Idle,
            open: None,
            folded: Folded::default(),
            files_scroll: UniformListScrollHandle::new(),
            scope: None,
            editing: None,
        }
    }

    pub(crate) fn show(&mut self, repo: &Repo, commit: &Commit, cx: &mut Context<Self>) {
        let already_shown = self
            .commit
            .as_ref()
            .is_some_and(|shown| shown.id == commit.id);
        self.rescope(commit, already_shown, cx);
        if already_shown && self.details.failure().is_none() {
            return;
        }
        if !already_shown {
            self.open = None;
            self.folded = Folded::default();
            self.files_scroll = UniformListScrollHandle::new();
        }
        self.commit = Some(commit.clone());
        self.root = Some(Arc::from(repo.location()));
        let repo = repo.clone();
        let id = commit.id;
        let task = run_in_background(
            cx,
            move || {
                let details = repo.commit_details(id)?;
                Ok((details.body, parse_patch(&details.patch)?))
            },
            |this, result: anyhow::Result<(SharedString, Vec<DiffFile>)>, cx| {
                this.details.finish(result.map(|(body, files)| Summary {
                    body,
                    stats: PatchStats::of(&files),
                    files: files.into(),
                }));
                cx.notify();
            },
        );
        self.details.load(task);
        cx.notify();
    }

    pub(crate) fn clear(&mut self, cx: &mut Context<Self>) {
        self.commit = None;
        self.details.clear();
        self.open = None;
        self.folded = Folded::default();
        self.scope = None;
        self.editing = None;
        cx.notify();
    }

    pub(crate) fn is_editing(&self, id: ObjectId, cx: &App) -> bool {
        self.editing
            .as_ref()
            .is_some_and(|editing| editing.view.read(cx).id() == id)
    }

    fn rescope(&mut self, commit: &Commit, already_shown: bool, cx: &mut Context<Self>) {
        self.scope = self
            .repository
            .read(cx)
            .history()
            .and_then(|history| history.reword_scope(&commit.id));
        if !already_shown || self.scope.is_none() {
            self.editing = None;
        }
        if let (Some(editing), Some(scope)) = (&self.editing, self.scope.clone()) {
            editing.view.update(cx, |editor, cx| editor.rescope(scope, cx));
        }
    }

    fn can_edit(&self, cx: &App) -> bool {
        let repository = self.repository.read(cx);
        self.scope.is_some()
            && self.details.value().is_some()
            && !repository.is_writing()
            && repository.underway().is_none()
    }

    fn start_editing(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() || !self.can_edit(cx) {
            return;
        }
        let (Some(commit), Some(scope), Some(summary)) =
            (&self.commit, self.scope.clone(), self.details.value())
        else {
            return;
        };
        let shown = (commit.subject.clone(), summary.body.clone());
        let (repository, id) = (self.repository.clone(), commit.id);
        let view = cx.new(|cx| MessageEditor::new(repository, id, scope, shown, field, window, cx));
        let stop = cx.subscribe(&view, |this, _, _: &StopEditing, cx| {
            this.editing = None;
            cx.notify();
        });
        self.editing = Some(Editing { view, _stop: stop });
        cx.notify();
    }

    fn message(&self, commit: &Commit, body: Option<SharedString>, cx: &mut Context<Self>) -> AnyElement {
        if let Some(editing) = &self.editing {
            return editing.view.clone().into_any_element();
        }
        let subject = div()
            .id("message-subject")
            .base_text()
            .font_medium()
            .child(commit_subject(commit));
        let body = body.map(|body| {
            div()
                .id("message-body")
                .max_h(BODY_HEIGHT)
                .overflow_y_scroll()
                .sm_text()
                .text_color(cx.theme().muted_foreground)
                .child(body)
        });
        let block = v_flex().id("commit-message").gap_1p5();
        if !self.can_edit(cx) {
            return block.child(subject).children(body).into_any_element();
        }
        let hover = cx.theme().list_hover;
        let on_subject = cx.listener(|this, _: &ClickEvent, window, cx| {
            this.start_editing(Field::Summary, window, cx)
        });
        let on_body = cx.listener(|this, _: &ClickEvent, window, cx| {
            this.start_editing(Field::Description, window, cx)
        });
        block
            .mx(px(-8.))
            .my(px(-4.))
            .px_2()
            .py_1()
            .rounded_md()
            .cursor_text()
            .hover(move |style| style.bg(hover))
            .tooltip(|window, cx| Tooltip::new(EDIT_TIP).build(window, cx))
            .child(subject.on_click(on_subject))
            .children(body.map(|body| body.on_click(on_body)))
            .into_any_element()
    }

    pub(crate) fn files(&self) -> Option<Arc<[DiffFile]>> {
        self.details.value().map(|summary| summary.files.clone())
    }

    pub(crate) fn commit(&self) -> Option<&Commit> {
        self.commit.as_ref()
    }

    pub(crate) fn files_of(&self, id: ObjectId) -> Option<Arc<[DiffFile]>> {
        self.commit.as_ref().filter(|commit| commit.id == id)?;
        self.files()
    }

    pub(crate) fn first_parent_of(&self, id: ObjectId) -> Option<ObjectId> {
        let commit = self.commit.as_ref().filter(|commit| commit.id == id)?;
        commit.parents.first().copied()
    }

    pub(crate) fn mark_open(&mut self, path: Option<SharedString>, cx: &mut Context<Self>) {
        if self.open != path {
            self.open = path;
            cx.notify();
        }
    }
}

pub(crate) fn copy_commit_hash(id: ObjectId, window: &mut Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(id.to_string()));
    window.push_notification("Commit hash copied", cx);
}

impl Render for CommitPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(commit) = self.commit.clone() else {
            return ui::hint("Select a commit to see its changes.", cx);
        };
        let muted = cx.theme().muted_foreground;
        let on_open =
            cx.listener(|_, file: &DiffFile, _, cx| cx.emit(OpenCommitFile(file.clone())));
        let on_history =
            cx.listener(|_, file: &DiffFile, _, cx| cx.emit(ShowFileHistory(file.clone())));
        let on_fold = cx.listener(|this, folder: &SharedString, _, cx| {
            this.folded.toggle(folder.clone());
            cx.notify();
        });
        let view = FileView::current(cx);
        let content = patch_content(
            &self.details,
            |summary| summary.stats,
            &TEXTS,
            |summary| {
                CommitFiles::new(
                    summary.files.clone(),
                    view,
                    &self.folded,
                    self.files_scroll.clone(),
                )
                .open(self.open.clone())
                .root(self.root.clone())
                .on_open(on_open)
                .on_history(on_history)
                .on_fold(on_fold)
                .into_any_element()
            },
            cx,
        );
        let body = self
            .details
            .value()
            .filter(|summary| !summary.body.is_empty())
            .map(|summary| summary.body.clone());
        let stats = self
            .details
            .value()
            .map(|summary| summary.stats)
            .filter(|stats| stats.files > 0);
        let parents = commit
            .parents
            .iter()
            .map(|id| short_id(*id).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let id = commit.id;
        let message = self.message(&commit, body, cx);
        v_flex()
            .pane()
            .child(
                v_flex()
                    .flex_none()
                    .px_4()
                    .py_3()
                    .gap_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(message)
                    .child(
                        h_flex()
                            .gap_2()
                            .xs_text()
                            .text_color(muted)
                            .child(ui::author_avatar(&commit.author, &commit.author_email, AVATAR_SIZE))
                            .child(commit.author.clone())
                            .child("·")
                            .child(commit.authored.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .xs_text()
                            .text_color(muted)
                            .child(
                                div()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child(commit.short_id.clone()),
                            )
                            .when(!parents.is_empty(), |this| {
                                this.child(format!(
                                    "{} {parents}",
                                    if commit.parents.len() > 1 {
                                        "merges"
                                    } else {
                                        "parent"
                                    }
                                ))
                            })
                            .child(div().flex_1())
                            .child(
                                Button::new("copy-hash")
                                    .ghost()
                                    .xsmall()
                                    .label("Copy hash")
                                    .tooltip_with_action(
                                        "Copy the full hash",
                                        &CopyHash,
                                        Some(WINDOW_CONTEXT),
                                    )
                                    .on_click(move |_, window, cx| {
                                        copy_commit_hash(id, window, cx)
                                    }),
                            ),
                    )
                    .when_some(stats, |this, stats| {
                        this.child(
                            h_flex()
                                .gap_3()
                                .xs_text()
                                .child(div().text_color(muted).child(counted(
                                    stats.files,
                                    "file",
                                    "files",
                                )))
                                .child(ui::additions(stats.additions, cx))
                                .child(ui::deletions(stats.deletions, cx))
                                .child(div().flex_1())
                                .child(FileViewToggle),
                        )
                    }),
            )
            .child(div().fill_height().child(content))
            .into_any_element()
    }
}
