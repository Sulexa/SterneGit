use gpui_kit::{
    AnyElement, AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        diff::{Diff, DiffFile, DiffMode, DiffState},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    Repository,
    navigation::OpenFile,
    patch_view::{
        PatchStats, PatchTexts, RedrawAfterMeasuring, change_kind, file_patch_files, patch_content,
        renamed_from,
    },
};
use crate::{
    actions::{CloseDiff, WINDOW_CONTEXT},
    git::{ChangeKind, FileChange, Repo, Scope, Side},
    ui::{self, Load, Look as _, SideUi as _, change_badge, run_in_background},
};

const TEXTS: PatchTexts = PatchTexts {
    loading: "Reading changes…",
    failed: "Couldn't compare this file",
    empty: "No changes to show: only line endings or file mode differ.",
};

const TOOLBAR_HEIGHT: f32 = 40.;

pub(crate) struct CloseFileDiff;

enum Compared {
    WorkingFile(OpenFile),
    CommitFile {
        short_id: SharedString,
        file: DiffFile,
    },
}

struct Heading {
    kind: ChangeKind,
    path: SharedString,
    original: Option<SharedString>,
    state: SharedString,
}

pub(crate) struct FileDiffPanel {
    repository: Entity<Repository>,
    compared: Option<Compared>,
    patch: Load<PatchStats>,
    diff: Entity<DiffState>,
    redraw_after_measuring: RedrawAfterMeasuring,
    _diff_changed: Subscription,
}

impl EventEmitter<CloseFileDiff> for FileDiffPanel {}

impl FileDiffPanel {
    pub(crate) fn new(repository: Entity<Repository>, cx: &mut Context<Self>) -> Self {
        let diff = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Split));
        let diff_changed = cx.observe(&diff, |_, _, cx| cx.notify());
        Self {
            repository,
            compared: None,
            patch: Load::Idle,
            diff,
            redraw_after_measuring: RedrawAfterMeasuring::default(),
            _diff_changed: diff_changed,
        }
    }

    pub(crate) fn open_working_file(
        &mut self,
        repo: &Repo,
        change: FileChange,
        side: Side,
        cx: &mut Context<Self>,
    ) {
        let file = OpenFile {
            side,
            path: change.path.clone(),
        };
        let repo = repo.clone();
        let task = run_in_background(
            cx,
            move || file_patch_files(repo.file_patch(&change, side)?),
            |this, result, cx| {
                let stats = result.map(|files| {
                    let stats = PatchStats::of(&files);
                    this.diff.update(cx, |state, cx| state.set_files(files, cx));
                    this.redraw_after_measuring.files_changed();
                    stats
                });
                this.patch.finish(stats);
                cx.notify();
            },
        );
        if matches!(&self.compared, Some(Compared::WorkingFile(shown)) if *shown == file) {
            self.patch.reload(task);
        } else {
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            self.patch.load(task);
        }
        self.compared = Some(Compared::WorkingFile(file));
        cx.notify();
    }

    pub(crate) fn open_commit_file(
        &mut self,
        short_id: SharedString,
        file: DiffFile,
        cx: &mut Context<Self>,
    ) {
        let stats = PatchStats::of(std::slice::from_ref(&file));
        self.diff
            .update(cx, |state, cx| state.set_files([file.clone()], cx));
        self.redraw_after_measuring.files_changed();
        self.patch = Load::Ready(stats);
        self.compared = Some(Compared::CommitFile { short_id, file });
        cx.notify();
    }

    pub(crate) fn close(&mut self, cx: &mut Context<Self>) {
        if self.compared.take().is_some() {
            self.patch.clear();
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            cx.notify();
        }
    }

    fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        self.diff.update(cx, |state, cx| state.set_mode(mode, cx));
    }

    fn move_across(&mut self, cx: &mut Context<Self>) {
        let Some(Compared::WorkingFile(open)) = &self.compared else {
            return;
        };
        let open = open.clone();
        let change = self
            .repository
            .read(cx)
            .changes()
            .and_then(|changes| changes.find(open.side, &open.path))
            .cloned();
        if let Some(change) = change {
            self.repository.update(cx, |repository, cx| {
                repository.move_across(open.side, Scope::Files(vec![change]), cx)
            });
        }
    }

    fn move_button(&self, open: &OpenFile, listed: bool, cx: &mut Context<Self>) -> AnyElement {
        let labels = open.side.labels();
        let writing = self.repository.read(cx).is_writing();
        Button::new("diff-stage")
            .xsmall()
            .icon(labels.icon)
            .label(labels.verb)
            .tooltip(labels.open_file_tooltip)
            .disabled(writing || !listed)
            .on_click(cx.listener(|this, _, _, cx| this.move_across(cx)))
            .into_any_element()
    }
}

impl Render for FileDiffPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.redraw_after_measuring.request(window);
        let (heading, move_button) = match &self.compared {
            None => return div().into_any_element(),
            Some(Compared::WorkingFile(open)) => {
                let change = self
                    .repository
                    .read(cx)
                    .changes()
                    .and_then(|changes| changes.find(open.side, &open.path))
                    .cloned();
                let button = self.move_button(open, change.is_some(), cx);
                (working_heading(open, change), Some(button))
            }
            Some(Compared::CommitFile { short_id, file }) => (commit_heading(short_id, file), None),
        };
        let muted = cx.theme().muted_foreground;
        let mode = self.diff.read(cx).mode();
        let stats = self.patch.value().copied();
        let shown_path = match &heading.original {
            Some(original) => SharedString::from(format!("{original} → {}", heading.path)),
            None => heading.path.clone(),
        };
        let toolbar = h_flex()
            .flex_none()
            .w_full()
            .min_w_0()
            .h(px(TOOLBAR_HEIGHT))
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(change_badge(heading.kind, cx))
            .child(
                div()
                    .ellipsis()
                    .sm_text()
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(shown_path),
            )
            .child(
                div()
                    .flex_none()
                    .xs_text()
                    .text_color(muted)
                    .child(heading.state),
            )
            .when_some(stats, |this, stats| {
                this.child(ui::additions(stats.additions, cx).flex_none().xs_text())
                    .child(ui::deletions(stats.deletions, cx).flex_none().xs_text())
            })
            .child(div().flex_1())
            .child(
                Button::new("diff-split")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Columns2)
                    .label("Split")
                    .selected(mode == DiffMode::Split)
                    .tooltip("Compare side by side")
                    .on_click(cx.listener(|this, _, _, cx| this.set_mode(DiffMode::Split, cx))),
            )
            .child(
                Button::new("diff-unified")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Rows2)
                    .label("Unified")
                    .selected(mode == DiffMode::Unified)
                    .tooltip("Show changes in one column")
                    .on_click(cx.listener(|this, _, _, cx| this.set_mode(DiffMode::Unified, cx))),
            )
            .children(move_button)
            .child(
                Button::new("diff-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip_with_action("Back to the graph", &CloseDiff, Some(WINDOW_CONTEXT))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(CloseFileDiff))),
            );
        let content = patch_content(
            &self.patch,
            |stats| *stats,
            &TEXTS,
            |_| {
                Diff::new(&self.diff)
                    .line_number(true)
                    .soft_wrap(true)
                    .header_visible(false)
                    .border_0()
                    .into_any_element()
            },
            cx,
        );
        v_flex()
            .pane()
            .child(toolbar)
            .child(div().fill_height().child(content))
            .into_any_element()
    }
}

fn working_heading(open: &OpenFile, change: Option<FileChange>) -> Heading {
    Heading {
        kind: change
            .as_ref()
            .map_or(ChangeKind::Modified, |change| change.kind),
        path: open.path.clone(),
        original: change.and_then(|change| change.original_path),
        state: open.side.labels().state.into(),
    }
}

fn commit_heading(short_id: &SharedString, file: &DiffFile) -> Heading {
    Heading {
        kind: change_kind(file),
        path: file.path().clone(),
        original: renamed_from(file).cloned(),
        state: format!("in {short_id}").into(),
    }
}
