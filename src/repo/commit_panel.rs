use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    App, ClipboardItem, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _,
    UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        diff::DiffFile,
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    commit_files::CommitFiles,
    patch_view::{PatchStats, PatchTexts, parse_patch, patch_content},
};
use crate::{
    actions::{CopyHash, WINDOW_CONTEXT},
    git::{Commit, Repo, short_id},
    ui::{
        self, FileView, FileViewToggle, Folded, Load, Look as _, commit_subject, counted,
        run_in_background,
    },
};

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

pub(crate) struct CommitPanel {
    commit: Option<Commit>,
    details: Load<Summary>,
    open: Option<SharedString>,
    folded: Folded<SharedString>,
    files_scroll: UniformListScrollHandle,
}

impl EventEmitter<OpenCommitFile> for CommitPanel {}

impl CommitPanel {
    pub(crate) fn new() -> Self {
        Self {
            commit: None,
            details: Load::Idle,
            open: None,
            folded: Folded::default(),
            files_scroll: UniformListScrollHandle::new(),
        }
    }

    pub(crate) fn show(&mut self, repo: &Repo, commit: &Commit, cx: &mut Context<Self>) {
        let already_shown = self
            .commit
            .as_ref()
            .is_some_and(|shown| shown.id == commit.id);
        if already_shown && self.details.failure().is_none() {
            return;
        }
        if !already_shown {
            self.open = None;
            self.folded = Folded::default();
            self.files_scroll = UniformListScrollHandle::new();
        }
        self.commit = Some(commit.clone());
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
        cx.notify();
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
                .on_open(on_open)
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
                    .child(
                        div()
                            .base_text()
                            .font_medium()
                            .child(commit_subject(&commit)),
                    )
                    .when_some(body, |this, body| {
                        this.child(
                            div()
                                .id("message-body")
                                .max_h(px(140.))
                                .overflow_y_scroll()
                                .sm_text()
                                .text_color(muted)
                                .child(body),
                        )
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .xs_text()
                            .text_color(muted)
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
