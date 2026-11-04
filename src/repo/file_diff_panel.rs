use gpui_kit::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        diff::{Diff, DiffMode, DiffState},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    Repository,
    navigation::OpenFile,
    patch_view::{PatchStats, PatchTexts, RedrawAfterMeasuring, file_patch_files, patch_content},
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

pub(crate) struct FileDiffPanel {
    repository: Entity<Repository>,
    open: Option<OpenFile>,
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
            open: None,
            patch: Load::Idle,
            diff,
            redraw_after_measuring: RedrawAfterMeasuring::default(),
            _diff_changed: diff_changed,
        }
    }

    pub(crate) fn open(
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
        if self.open.as_ref() == Some(&file) {
            self.patch.reload(task);
        } else {
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            self.patch.load(task);
        }
        self.open = Some(file);
        cx.notify();
    }

    pub(crate) fn close(&mut self, cx: &mut Context<Self>) {
        if self.open.take().is_some() {
            self.patch.clear();
            self.diff.update(cx, |state, cx| state.set_files([], cx));
            cx.notify();
        }
    }

    fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        self.diff.update(cx, |state, cx| state.set_mode(mode, cx));
    }

    fn move_across(&mut self, cx: &mut Context<Self>) {
        let Some(open) = self.open.clone() else {
            return;
        };
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
}

impl Render for FileDiffPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.redraw_after_measuring.request(window);
        let Some(open) = self.open.clone() else {
            return div().into_any_element();
        };
        let repository = self.repository.read(cx);
        let writing = repository.is_writing();
        let change = repository
            .changes()
            .and_then(|changes| changes.find(open.side, &open.path))
            .cloned();
        let muted = cx.theme().muted_foreground;
        let kind = change
            .as_ref()
            .map_or(ChangeKind::Modified, |change| change.kind);
        let original = change
            .as_ref()
            .and_then(|change| change.original_path.clone());
        let mode = self.diff.read(cx).mode();
        let stats = self.patch.value().copied();
        let labels = open.side.labels();
        let toolbar = h_flex()
            .flex_none()
            .w_full()
            .min_w_0()
            .h(px(TOOLBAR_HEIGHT))
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(change_badge(kind, cx))
            .child(
                div()
                    .ellipsis()
                    .text_sm()
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(match &original {
                        Some(original) => SharedString::from(format!("{original} → {}", open.path)),
                        None => open.path.clone(),
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .text_xs()
                    .text_color(muted)
                    .child(labels.state),
            )
            .when_some(stats, |this, stats| {
                this.child(ui::additions(stats.additions, cx).flex_none().text_xs())
                    .child(ui::deletions(stats.deletions, cx).flex_none().text_xs())
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
            .child(
                Button::new("diff-stage")
                    .xsmall()
                    .icon(labels.icon)
                    .label(labels.verb)
                    .tooltip(labels.open_file_tooltip)
                    .disabled(writing || change.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.move_across(cx))),
            )
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
            Diff::new(&self.diff)
                .line_number(true)
                .soft_wrap(true)
                .header_visible(false),
            cx,
        );
        v_flex()
            .pane()
            .child(toolbar)
            .child(div().fill_height().child(content))
            .into_any_element()
    }
}
