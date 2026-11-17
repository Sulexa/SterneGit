use std::{path::Path, rc::Rc, sync::Arc};

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, FocusHandle, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, UniformListScrollHandle, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, h_flex, menu::ContextMenuExt as _, scroll::Scrollbar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, relative, uniform_list,
};

use super::{
    branch_menu::BranchMenu,
    commit_menu::CommitMenu,
    commit_search::{CommitFinder, CommitsFound},
    pulls::PullMarks,
    ref_menus::RefMenus,
    stash_menu::StashMenu,
    tag_menu::TagMenu,
    worktree_menu::WorktreeMenu,
};
use crate::{
    actions::HISTORY_CONTEXT,
    git::{History, RefLabel, ShownRef, WorkingChanges, Worktree, WorktreeChanges},
    ui::{
        self, GraphNode, Handler, Look as _, Palette, Pick, Picked, ROW_HEIGHT, commit_subject,
        found_text, handler, is_double_click, list_row, ref_badge, stash_badge, thousands,
        worktree_badge,
    },
};

const UNCOMMITTED: &str = "Uncommitted changes";

const MAX_BADGES: usize = 3;
const AUTHOR_WIDTH: Pixels = px(120.);
const AVATAR_SIZE: Pixels = px(16.);
const DATE_WIDTH: Pixels = px(104.);
const ID_WIDTH: Pixels = px(56.);
const UNFOUND_OPACITY: f32 = 0.35;
const TICKS_WIDTH: Pixels = px(10.);
const TICK_HEIGHT: Pixels = px(2.);
const TICK_PLACES: usize = 512;

#[derive(Clone)]
pub(crate) struct Finding {
    pub(crate) finder: Rc<CommitFinder>,
    pub(crate) found: Rc<CommitsFound>,
}

impl Finding {
    fn ranges(&self, row: usize, text: &str) -> Vec<std::ops::Range<usize>> {
        if self.found.has(row) {
            self.finder.ranges(text)
        } else {
            Vec::new()
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct HistoryList {
    history: Arc<History>,
    selected: Option<usize>,
    picked: Rc<Picked<ObjectId>>,
    uncommitted: Option<(Arc<WorkingChanges>, bool)>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    on_select_commit: Option<Handler<(ObjectId, Pick)>>,
    on_select_uncommitted: Option<Handler<()>>,
    on_check_out: Option<Handler<RefLabel>>,
    on_select_worktree: Option<Handler<Arc<Path>>>,
    menus: RefMenus,
    stash_menu: Option<StashMenu>,
    commit_menu: Option<CommitMenu>,
    worktree_menu: Option<WorktreeMenu>,
    elsewhere: Arc<[WorktreeChanges]>,
    marks: Arc<PullMarks>,
    finding: Option<Finding>,
}

#[derive(Clone)]
struct RowActions {
    on_select: Option<Handler<(ObjectId, Pick)>>,
    on_check_out: Option<Handler<RefLabel>>,
    on_select_worktree: Option<Handler<Arc<Path>>>,
    menus: RefMenus,
    stash_menu: Option<StashMenu>,
    commit_menu: Option<CommitMenu>,
    worktree_menu: Option<WorktreeMenu>,
    elsewhere: Arc<[WorktreeChanges]>,
    marks: Arc<PullMarks>,
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
            picked: Rc::default(),
            uncommitted: None,
            focus,
            scroll,
            on_select_commit: None,
            on_select_uncommitted: None,
            on_check_out: None,
            on_select_worktree: None,
            menus: RefMenus::default(),
            stash_menu: None,
            commit_menu: None,
            worktree_menu: None,
            elsewhere: Arc::from([]),
            marks: Arc::default(),
            finding: None,
        }
    }

    pub(crate) fn finding(mut self, finding: Option<Finding>) -> Self {
        self.finding = finding;
        self
    }

    pub(crate) fn worktrees(
        mut self,
        elsewhere: Vec<WorktreeChanges>,
        selected: Option<&Path>,
        menu: WorktreeMenu,
    ) -> Self {
        if let Some(row) = selected.and_then(|location| self.history.changes_row_of(location)) {
            self.selected = Some(row);
        }
        self.elsewhere = elsewhere.into();
        self.worktree_menu = Some(menu);
        self
    }

    pub(crate) fn on_select_worktree(
        mut self,
        callback: impl Fn(&Arc<Path>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select_worktree = Some(handler(callback));
        self
    }

    pub(crate) fn marks(mut self, marks: Arc<PullMarks>) -> Self {
        self.marks = marks;
        self
    }

    pub(crate) fn branch_menu(mut self, menu: BranchMenu) -> Self {
        self.menus.branch = Some(menu);
        self
    }

    pub(crate) fn tag_menu(mut self, menu: TagMenu) -> Self {
        self.menus.tag = Some(menu);
        self
    }

    pub(crate) fn stash_menu(mut self, menu: StashMenu) -> Self {
        self.stash_menu = Some(menu);
        self
    }

    pub(crate) fn commit_menu(mut self, menu: CommitMenu) -> Self {
        self.commit_menu = Some(menu);
        self
    }

    pub(crate) fn picked(mut self, picked: Rc<Picked<ObjectId>>) -> Self {
        self.picked = picked;
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
        callback: impl Fn(&(ObjectId, Pick), &mut Window, &mut App) + 'static,
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
            picked,
            uncommitted,
            focus,
            scroll,
            on_select_commit,
            on_select_uncommitted,
            on_check_out,
            on_select_worktree,
            menus,
            stash_menu,
            commit_menu,
            worktree_menu,
            elsewhere,
            marks,
            finding,
        } = self;
        let actions = RowActions {
            on_select: on_select_commit,
            on_check_out,
            on_select_worktree,
            menus,
            stash_menu,
            commit_menu,
            worktree_menu,
            elsewhere,
            marks,
        };
        let palette = Palette::new(cx);
        let uncommitted_row = uncommitted.map(|(changes, selected)| {
            uncommitted_row(&changes, selected, &palette, on_select_uncommitted, cx)
        });
        let focus_on_click = focus.clone();
        let ticks = finding
            .as_ref()
            .map(|finding| found_ticks(&finding.found, history.commits.len()));
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
                                selected == Some(row) || picked.contains(&history.commits[row].id),
                                graph_width,
                                &palette,
                                &actions,
                                finding.as_ref(),
                                cx,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .children(ticks)
            .child(Scrollbar::vertical(&scroll));
        v_flex()
            .size_full()
            .min_h_0()
            .children(uncommitted_row)
            .child(list)
    }
}

fn found_ticks(found: &CommitsFound, rows: usize) -> AnyElement {
    let mut places: Vec<usize> = found
        .rows
        .iter()
        .map(|row| row * TICK_PLACES / rows.max(1))
        .collect();
    places.dedup();
    let color = ui::found_color();
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .w(TICKS_WIDTH)
        .children(places.into_iter().map(|place| {
            div()
                .absolute()
                .left(px(1.))
                .right(px(1.))
                .top(relative(place as f32 / TICK_PLACES as f32))
                .h(TICK_HEIGHT)
                .rounded(px(1.))
                .bg(color)
        }))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn commit_row(
    history: &History,
    row: usize,
    selected: bool,
    graph_width: Pixels,
    palette: &Palette,
    actions: &RowActions,
    finding: Option<&Finding>,
    cx: &App,
) -> AnyElement {
    if let Some(worktree) = history.worktree_on_row(row) {
        return changes_row(history, row, worktree, selected, graph_width, palette, actions, cx);
    }
    let unfound = finding.is_some_and(|finding| !finding.found.has(row));
    let ranges = |text: &str| finding.map(|finding| finding.ranges(row, text)).unwrap_or_default();
    let unseen = finding.and_then(|finding| finding.found.unseen.get(&row).copied());
    let id_ranges = finding
        .filter(|finding| finding.found.has(row))
        .map(|finding| finding.finder.shown_id_ranges(&history.commits[row].id.to_string()))
        .unwrap_or_default();
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
    let shown = if unfound { UNFOUND_OPACITY } else { 1. };
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
                        .opacity(shown)
                        .children(stash.map(|stash| {
                            stash_badge(stash.name.clone(), color, ranges(&stash.name), cx)
                        }))
                        .children(history.detached_worktrees_on(&id).map(|worktree| {
                            worktree_tag(worktree, color, actions, ranges(&worktree.name), cx)
                        }))
                        .children(labels.iter().take(MAX_BADGES).map(|&shown| {
                            let found = ranges(&shown.label.name);
                            branch_badge(shown, color, actions, found, cx)
                        }))
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
                                .when(unseen.is_none(), |this| this.flex_1())
                                .ellipsis()
                                .sm_text()
                                .when(commit.subject.is_empty(), |this| {
                                    this.text_color(palette.muted)
                                })
                                .child(found_text(commit_subject(commit), ranges(&commit.subject))),
                        )
                        .children(unseen.map(|unseen| unseen_chip(unseen.label()))),
                )
                .child(
                    h_flex()
                        .flex_none()
                        .w(AUTHOR_WIDTH)
                        .gap_1p5()
                        .xs_text()
                        .text_color(palette.muted)
                        .opacity(shown)
                        .child(ui::author_avatar(&commit.author, &commit.author_email, AVATAR_SIZE))
                        .child(
                            div()
                                .ellipsis()
                                .child(found_text(commit.author.clone(), ranges(&commit.author))),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .w(DATE_WIDTH)
                        .xs_text()
                        .text_color(palette.muted)
                        .opacity(shown)
                        .child(commit.committed.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(ID_WIDTH)
                        .xs_text()
                        .font_family(palette.mono.clone())
                        .text_color(palette.muted)
                        .opacity(shown)
                        .child(found_text(commit.short_id.clone(), id_ranges)),
                ),
        )
        .when_some(actions.on_select.clone(), |row, on_select| {
            let alone = stash.is_some();
            row.on_click(move |event, window, cx| {
                let modifiers = event.modifiers();
                let pick = match alone {
                    true => Pick::Only,
                    false => Pick::new(modifiers.secondary(cx), modifiers.shift),
                };
                on_select(&(id, pick), window, cx)
            })
        });
    match (stash, actions.stash_menu.clone(), actions.commit_menu.clone()) {
        (Some(stash), Some(stash_menu), _) => {
            let right_clicked = stash.clone();
            row.context_menu(move |menu, _, _| stash_menu.build(menu, &right_clicked))
                .into_any_element()
        }
        (None, _, Some(commit_menu)) => row
            .context_menu(move |menu, window, cx| commit_menu.build(menu, id, window, cx))
            .into_any_element(),
        _ => row.into_any_element(),
    }
}

fn unseen_chip(label: &'static str) -> AnyElement {
    let color = ui::found_color();
    h_flex()
        .flex_none()
        .h(px(18.))
        .px_1p5()
        .gap_1()
        .rounded(px(9.))
        .xs_text()
        .text_color(color)
        .bg(color.opacity(0.12))
        .child(Icon::new(IconName::Search).xsmall().text_color(color))
        .child(label)
        .into_any_element()
}

fn branch_badge(
    shown: ShownRef,
    color: Hsla,
    actions: &RowActions,
    found: Vec<std::ops::Range<usize>>,
    cx: &App,
) -> AnyElement {
    let label = shown.label.clone();
    let on_check_out = actions
        .on_check_out
        .clone()
        .filter(|_| label.can_be_checked_out());
    let double_clicked = label.clone();
    let mark = actions.marks.get(&label.full_name()).copied();
    let badge = ref_badge(shown, color, mark, found, cx).when_some(on_check_out, |badge, on_check_out| {
        badge.on_click(move |event, window, cx| {
            if is_double_click(event) {
                on_check_out(&double_clicked, window, cx)
            }
        })
    });
    actions.menus.attach(badge, &label)
}

fn worktree_tag(
    worktree: &Worktree,
    color: Hsla,
    actions: &RowActions,
    found: Vec<std::ops::Range<usize>>,
    cx: &App,
) -> AnyElement {
    let badge = worktree_badge(worktree.name.clone(), color, found, cx);
    match actions.worktree_menu.clone() {
        Some(menu) => {
            let right_clicked = worktree.clone();
            badge
                .context_menu(move |popup, _, _| menu.build(popup, &right_clicked))
                .into_any_element()
        }
        None => badge.into_any_element(),
    }
}

#[allow(clippy::too_many_arguments)]
fn changes_row(
    history: &History,
    row: usize,
    worktree: &Worktree,
    selected: bool,
    graph_width: Pixels,
    palette: &Palette,
    actions: &RowActions,
    cx: &App,
) -> AnyElement {
    let graph_row = history.graph.rows[row].clone();
    let color = palette.lane(graph_row.node_lane);
    let changes = actions
        .elsewhere
        .iter()
        .find(|read| read.location == worktree.location)
        .map(|read| &read.changes)
        .or_else(|| history.changes_read_in(&worktree.location));
    let location = worktree.location.clone();
    let row = list_row(SharedString::from(format!("wip/{}", worktree.location.display())), selected)
        .px_3()
        .child(
            h_flex()
                .h(ROW_HEIGHT)
                .w_full()
                .min_w_0()
                .gap_3()
                .child(ui::graph_cell(graph_row, GraphNode::Changes, palette, graph_width))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .gap_1p5()
                        .child(worktree_badge(worktree.name.clone(), color, Vec::new(), cx))

                        .child(div().flex_1().ellipsis().sm_text().italic().child(UNCOMMITTED)),
                )
                .child(
                    div()
                        .flex_none()
                        .w(AUTHOR_WIDTH + DATE_WIDTH + ID_WIDTH)
                        .ellipsis()
                        .xs_text()
                        .text_color(palette.muted)
                        .child(changes.map(|changes| summary(changes)).unwrap_or_default()),
                ),
        )
        .when_some(actions.on_select_worktree.clone(), |row, on_select| {
            row.on_click(move |_, window, cx| on_select(&location, window, cx))
        });
    match actions.worktree_menu.clone() {
        Some(menu) => {
            let right_clicked = worktree.clone();
            row.context_menu(move |popup, _, _| menu.build(popup, &right_clicked))
                .into_any_element()
        }
        None => row.into_any_element(),
    }
}

pub(crate) fn summary(changes: &WorkingChanges) -> String {
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
    summary.join(" · ")
}

fn uncommitted_row(
    changes: &WorkingChanges,
    selected: bool,
    palette: &Palette,
    on_select: Option<Handler<()>>,
    cx: &App,
) -> AnyElement {
    let summary = summary(changes);
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
                        .child(UNCOMMITTED),
                )
                .child(
                    div()
                        .flex_none()
                        .xs_text()
                        .text_color(palette.muted)
                        .child(summary),
                ),
        )
        .when_some(on_select, |row, on_select| {
            row.on_click(move |_, window, cx| on_select(&(), window, cx))
        })
        .into_any_element()
}
