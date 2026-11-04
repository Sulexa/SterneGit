use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, FocusHandle, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, SharedString, Styled as _, UniformListScrollHandle,
    Window,
    component::{ActiveTheme as _, h_flex, scroll::Scrollbar, v_flex},
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};

use crate::{
    actions::HISTORY_CONTEXT,
    git::{History, RefLabel, WorkingChanges},
    ui::{
        self, Handler, Look as _, Palette, ROW_HEIGHT, commit_subject, handler, list_row,
        ref_badge, thousands,
    },
};

const MAX_BADGES: usize = 3;
const AUTHOR_WIDTH: Pixels = px(120.);
const DATE_WIDTH: Pixels = px(104.);
const ID_WIDTH: Pixels = px(56.);

#[derive(IntoElement)]
pub(crate) struct HistoryList {
    history: Arc<History>,
    selected: Option<usize>,
    uncommitted: Option<(Arc<WorkingChanges>, bool)>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    on_select_commit: Option<Handler<ObjectId>>,
    on_select_uncommitted: Option<Handler<()>>,
}

impl HistoryList {
    pub(crate) fn new(
        history: Arc<History>,
        focus: FocusHandle,
        scroll: UniformListScrollHandle,
    ) -> Self {
        Self {
            history,
            selected: None,
            uncommitted: None,
            focus,
            scroll,
            on_select_commit: None,
            on_select_uncommitted: None,
        }
    }

    pub(crate) fn selected(mut self, commit: Option<ObjectId>) -> Self {
        self.selected = commit.and_then(|id| self.history.row_of(&id));
        self
    }

    pub(crate) fn uncommitted(
        mut self,
        changes: Option<Arc<WorkingChanges>>,
        selected: bool,
    ) -> Self {
        self.uncommitted = changes
            .filter(|changes| !changes.is_empty())
            .map(|changes| (changes, selected));
        self
    }

    pub(crate) fn on_select_commit(
        mut self,
        callback: impl Fn(&ObjectId, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select_commit = Some(handler(callback));
        self
    }

    pub(crate) fn on_select_uncommitted(
        mut self,
        callback: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select_uncommitted = Some(handler(callback));
        self
    }
}

impl RenderOnce for HistoryList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            history,
            selected,
            uncommitted,
            focus,
            scroll,
            on_select_commit,
            on_select_uncommitted,
        } = self;
        let palette = Palette::new(cx);
        let uncommitted_row = uncommitted.map(|(changes, selected)| {
            uncommitted_row(&changes, selected, &palette, on_select_uncommitted, cx)
        });
        let focus_on_click = focus.clone();
        let list = div()
            .id("history")
            .relative()
            .flex_1()
            .w_full()
            .min_h_0()
            .track_focus(&focus)
            .key_context(HISTORY_CONTEXT)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus_on_click.focus(window, cx)
            })
            .child(
                uniform_list("commits", history.commits.len(), move |range, _, cx| {
                    let graph_width = ui::graph_width(&history.graph.rows[range.clone()]);
                    range
                        .map(|row| {
                            commit_row(
                                &history,
                                row,
                                selected == Some(row),
                                graph_width,
                                &palette,
                                on_select_commit.clone(),
                                cx,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll));
        v_flex()
            .size_full()
            .min_h_0()
            .children(uncommitted_row)
            .child(list)
    }
}

fn commit_row(
    history: &History,
    row: usize,
    selected: bool,
    graph_width: Pixels,
    palette: &Palette,
    on_select: Option<Handler<ObjectId>>,
    cx: &App,
) -> AnyElement {
    let commit = &history.commits[row];
    let id = commit.id;
    let graph_row = history.graph.rows[row].clone();
    let is_head = history.head_id == Some(commit.id);
    let labels: Vec<&RefLabel> = history.labels_on(&commit.id).collect();
    let color = palette.lane(graph_row.node_lane);
    list_row(SharedString::from(commit.id.to_string()), selected)
        .px_3()
        .child(
            h_flex()
                .h(ROW_HEIGHT)
                .w_full()
                .min_w_0()
                .gap_3()
                .child(ui::graph_cell(graph_row, is_head, palette, graph_width))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .gap_1p5()
                        .children(
                            labels
                                .iter()
                                .take(MAX_BADGES)
                                .map(|label| ref_badge(label, color, cx)),
                        )
                        .when(labels.len() > MAX_BADGES, |this| {
                            this.child(
                                div()
                                    .flex_none()
                                    .xs_text()
                                    .text_color(palette.muted)
                                    .child(format!("+{}", labels.len() - MAX_BADGES)),
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .ellipsis()
                                .sm_text()
                                .when(commit.subject.is_empty(), |this| {
                                    this.text_color(palette.muted)
                                })
                                .child(commit_subject(commit)),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .w(AUTHOR_WIDTH)
                        .truncate()
                        .xs_text()
                        .text_color(palette.muted)
                        .child(commit.author.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(DATE_WIDTH)
                        .xs_text()
                        .text_color(palette.muted)
                        .child(commit.committed.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(ID_WIDTH)
                        .xs_text()
                        .font_family(palette.mono.clone())
                        .text_color(palette.muted)
                        .child(commit.short_id.clone()),
                ),
        )
        .when_some(on_select, |row, on_select| {
            row.on_click(move |_, window, cx| on_select(&id, window, cx))
        })
        .into_any_element()
}

fn uncommitted_row(
    changes: &WorkingChanges,
    selected: bool,
    palette: &Palette,
    on_select: Option<Handler<()>>,
    cx: &App,
) -> AnyElement {
    let mut summary = Vec::new();
    if !changes.unstaged.is_empty() {
        summary.push(format!("{} unstaged", thousands(changes.unstaged.len())));
    }
    if !changes.staged.is_empty() {
        summary.push(format!("{} staged", thousands(changes.staged.len())));
    }
    list_row("wip", selected)
        .flex_none()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .h(ROW_HEIGHT)
                .w_full()
                .min_w_0()
                .gap_3()
                .child(ui::uncommitted_cell(palette.muted))
                .child(
                    div()
                        .flex_1()
                        .ellipsis()
                        .sm_text()
                        .italic()
                        .child("Uncommitted changes"),
                )
                .child(
                    div()
                        .flex_none()
                        .xs_text()
                        .text_color(palette.muted)
                        .child(summary.join(" · ")),
                ),
        )
        .when_some(on_select, |row, on_select| {
            row.on_click(move |_, window, cx| on_select(&(), window, cx))
        })
        .into_any_element()
}
