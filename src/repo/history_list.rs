use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, FocusHandle, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, UniformListScrollHandle, Window,
    component::{ActiveTheme as _, h_flex, menu::ContextMenuExt as _, scroll::Scrollbar, v_flex},
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};

use super::stash_menu::StashMenu;
use crate::{
    actions::HISTORY_CONTEXT,
    git::{History, RefLabel, ShownRef, WorkingChanges},
    ui::{
        self, GraphNode, Handler, Look as _, Palette, ROW_HEIGHT, commit_subject, handler,
        is_double_click, list_row, ref_badge, stash_badge, thousands,
    },
};

const MAX_BADGES: usize = 3;
const AUTHOR_WIDTH: Pixels = px(120.);
const AVATAR_SIZE: Pixels = px(16.);
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
    on_check_out: Option<Handler<RefLabel>>,
    stash_menu: Option<StashMenu>,
}

#[derive(Clone)]
struct RowActions {
    on_select: Option<Handler<ObjectId>>,
    on_check_out: Option<Handler<RefLabel>>,
    stash_menu: Option<StashMenu>,
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
            on_check_out: None,
            stash_menu: None,
        }
    }

    pub(crate) fn stash_menu(mut self, menu: StashMenu) -> Self {
        self.stash_menu = Some(menu);
        self
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

    pub(crate) fn on_check_out(
        mut self,
        callback: impl Fn(&RefLabel, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_check_out = Some(handler(callback));
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
            on_check_out,
            stash_menu,
        } = self;
        let actions = RowActions {
            on_select: on_select_commit,
            on_check_out,
            stash_menu,
        };
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
                                &actions,
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
    actions: &RowActions,
    cx: &App,
) -> AnyElement {
    let commit = &history.commits[row];
    let id = commit.id;
    let graph_row = history.graph.rows[row].clone();
    let stash = history.stash_on(&commit.id);
    let node = match stash {
        Some(_) => GraphNode::Stash,
        None if history.head_id == Some(commit.id) => GraphNode::Head,
        None => GraphNode::Commit,
    };
    let labels = history.shown_on(&commit.id);
    let color = palette.lane(graph_row.node_lane);
    let row = list_row(SharedString::from(commit.id.to_string()), selected)
        .px_3()
        .child(
            h_flex()
                .h(ROW_HEIGHT)
                .w_full()
                .min_w_0()
                .gap_3()
                .child(ui::graph_cell(graph_row, node, palette, graph_width))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .gap_1p5()
                        .children(stash.map(|stash| stash_badge(stash.name.clone(), color, cx)))
                        .children(
                            labels
                                .iter()
                                .take(MAX_BADGES)
                                .map(|&shown| {
                                    branch_badge(shown, color, actions.on_check_out.clone(), cx)
                                }),
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
                    h_flex()
                        .flex_none()
                        .w(AUTHOR_WIDTH)
                        .gap_1p5()
                        .xs_text()
                        .text_color(palette.muted)
                        .child(ui::author_avatar(&commit.author, &commit.author_email, AVATAR_SIZE))
                        .child(div().ellipsis().child(commit.author.clone())),
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
        .when_some(actions.on_select.clone(), |row, on_select| {
            row.on_click(move |_, window, cx| on_select(&id, window, cx))
        });
    match (stash, actions.stash_menu.clone()) {
        (Some(stash), Some(stash_menu)) => {
            let right_clicked = stash.clone();
            row.context_menu(move |menu, _, _| stash_menu.build(menu, &right_clicked))
                .into_any_element()
        }
        _ => row.into_any_element(),
    }
}

fn branch_badge(
    shown: ShownRef,
    color: Hsla,
    on_check_out: Option<Handler<RefLabel>>,
    cx: &App,
) -> AnyElement {
    let label = shown.label.clone();
    let on_check_out = on_check_out.filter(|_| label.can_be_checked_out());
    ref_badge(shown, color, cx)
        .when_some(on_check_out, |badge, on_check_out| {
            badge.on_click(move |event, window, cx| {
                if is_double_click(event) {
                    on_check_out(&label, window, cx)
                }
            })
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
    if !changes.conflicted.is_empty() {
        summary.push(format!("{} conflicted", thousands(changes.conflicted.len())));
    }
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
