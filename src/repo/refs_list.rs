use std::{path::Path, rc::Rc, sync::Arc};

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, menu::ContextMenuExt as _,
        scroll::Scrollbar, tooltip::Tooltip,
    },
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::{
    branch_menu::BranchMenu,
    pulls::{PullMarks, PullsSection},
    ref_menus::RefMenus,
    ref_tree::{Listed, RefGroup, RefTreeRow, Section, last_segment, ref_tree},
    stash_menu::StashMenu,
    tag_menu::TagMenu,
    worktree_menu::WorktreeMenu,
};
use crate::{
    git::{AheadBehind, History, RefKind, RefLabel, Stash, Worktree},
    ui::{
        Folded, Handler, Look as _, Picked, PullMark, SectionHeader, checked_out_in, counted, handler,
        is_double_click, list_row, pull_chip, thousands, tree_line,
    },
};

const PANEL_TOOLTIP: &str = "Open the Worktrees panel to add and clean up worktrees";

#[derive(Clone)]
pub(crate) struct WorktreeSection {
    pub(crate) changed: Vec<usize>,
    pub(crate) selected: Option<Arc<Path>>,
    pub(crate) menu: WorktreeMenu,
    pub(crate) on_select: Handler<Arc<Path>>,
    pub(crate) on_open: Handler<Arc<Path>>,
    pub(crate) on_open_panel: Handler<()>,
}

#[derive(IntoElement)]
pub(crate) struct RefsList {
    history: Arc<History>,
    rows: Vec<RefTreeRow>,
    picked: Rc<Picked<ObjectId>>,
    scroll: UniformListScrollHandle,
    on_reveal: Option<Handler<usize>>,
    on_reveal_stash: Option<Handler<ObjectId>>,
    on_check_out: Option<Handler<RefLabel>>,
    on_toggle: Option<Handler<RefGroup>>,
    menus: RefMenus,
    stash_menu: Option<StashMenu>,
    pulls: Option<PullsSection>,
    worktrees: Option<WorktreeSection>,
    marks: Arc<PullMarks>,
}

impl RefsList {
    pub(crate) fn new(
        history: Arc<History>,
        folded: &Folded<RefGroup>,
        scroll: UniformListScrollHandle,
    ) -> Self {
        let listed = Listed {
            stashes: history.stashes.len(),
            worktrees: match history.worktrees.len() {
                0 | 1 => 0,
                listed => listed,
            },
        };
        Self {
            rows: ref_tree(&history.refs, listed, folded),
            history,
            picked: Rc::default(),
            scroll,
            on_reveal: None,
            on_reveal_stash: None,
            on_check_out: None,
            on_toggle: None,
            menus: RefMenus::default(),
            stash_menu: None,
            pulls: None,
            worktrees: None,
            marks: Arc::default(),
        }
    }

    pub(crate) fn worktrees(mut self, section: WorktreeSection) -> Self {
        self.worktrees = Some(section);
        self
    }

    pub(crate) fn pulls(mut self, pulls: Option<PullsSection>, marks: Arc<PullMarks>) -> Self {
        self.pulls = pulls;
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

    pub(crate) fn picked(mut self, picked: Rc<Picked<ObjectId>>) -> Self {
        self.picked = picked;
        self
    }

    pub(crate) fn on_reveal(
        mut self,
        callback: impl Fn(&usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reveal = Some(handler(callback));
        self
    }

    pub(crate) fn on_reveal_stash(
        mut self,
        callback: impl Fn(&ObjectId, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reveal_stash = Some(handler(callback));
        self
    }

    pub(crate) fn on_check_out(
        mut self,
        callback: impl Fn(&RefLabel, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_check_out = Some(handler(callback));
        self
    }

    pub(crate) fn on_toggle(
        mut self,
        callback: impl Fn(&RefGroup, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(handler(callback));
        self
    }
}

impl RenderOnce for RefsList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            history,
            rows,
            picked,
            scroll,
            on_reveal,
            on_reveal_stash,
            on_check_out,
            on_toggle,
            menus,
            stash_menu,
            pulls,
            worktrees,
            marks,
        } = self;
        let pull_rows = pulls.as_ref().map_or(0, PullsSection::len);
        let handlers = RowHandlers {
            on_reveal,
            on_reveal_stash,
            on_check_out,
            on_toggle,
            menus,
            stash_menu,
            worktrees,
        };
        div()
            .id("refs")
            .relative()
            .size_full()
            .min_h_0()
            .bg(cx.theme().sidebar)
            .child(
                uniform_list("ref-list", pull_rows + rows.len(), move |range, _, cx| {
                    range
                        .map(|ix| match ix.checked_sub(pull_rows) {
                            None => pulls
                                .as_ref()
                                .map_or_else(|| div().into_any_element(), |pulls| pulls.row(ix, cx)),
                            Some(ix) => ref_tree_row(&rows[ix], &history, &picked, &marks, &handlers, cx),
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll))
    }
}

struct RowHandlers {
    on_reveal: Option<Handler<usize>>,
    on_reveal_stash: Option<Handler<ObjectId>>,
    on_check_out: Option<Handler<RefLabel>>,
    on_toggle: Option<Handler<RefGroup>>,
    menus: RefMenus,
    stash_menu: Option<StashMenu>,
    worktrees: Option<WorktreeSection>,
}

fn ref_tree_row(
    row: &RefTreeRow,
    history: &History,
    picked: &Picked<ObjectId>,
    marks: &PullMarks,
    handlers: &RowHandlers,
    cx: &App,
) -> AnyElement {
    let RowHandlers {
        on_reveal,
        on_reveal_stash,
        on_check_out,
        on_toggle,
        menus,
        stash_menu,
        worktrees,
    } = handlers;
    match row {
        &RefTreeRow::Section {
            section,
            count,
            open,
        } => section_header(
            section,
            count,
            open,
            on_toggle.clone(),
            worktrees.as_ref().map(|section| section.on_open_panel.clone()),
        ),
        RefTreeRow::Folder {
            kind,
            path,
            depth,
            open,
        } => folder_row(*kind, path, *depth, *open, on_toggle.clone(), cx),
        &RefTreeRow::Ref { ix, depth } => ref_row(
            &history.refs[ix],
            ix,
            depth,
            picked,
            marks.get(&history.refs[ix].full_name()).copied(),
            RefClicks {
                on_reveal: on_reveal.clone(),
                on_check_out: on_check_out.clone(),
            },
            menus,
            cx,
        ),
        &RefTreeRow::Stash { ix } => stash_row(
            &history.stashes[ix],
            ix,
            picked,
            on_reveal_stash.clone(),
            stash_menu.clone(),
            cx,
        ),
        &RefTreeRow::Worktree { ix } => worktree_row(&history.worktrees[ix], ix, worktrees.as_ref(), cx),
    }
}

fn section_title(section: Section) -> &'static str {
    match section {
        Section::Refs(RefKind::LocalBranch) => "Branches",
        Section::Refs(RefKind::RemoteBranch) => "Remotes",
        Section::Worktrees => "Worktrees",
        Section::Stashes => "Stashes",
        Section::Refs(RefKind::Tag) => "Tags",
    }
}

fn folder_icon(kind: RefKind, depth: usize) -> IconName {
    match (kind, depth) {
        (RefKind::RemoteBranch, 0) => IconName::Cloud,
        _ => IconName::Folder,
    }
}

fn leaf_icon(kind: RefKind) -> IconName {
    match kind {
        RefKind::LocalBranch | RefKind::RemoteBranch => IconName::GitBranch,
        RefKind::Tag => IconName::Tag,
    }
}

fn section_header(
    section: Section,
    count: usize,
    open: bool,
    on_toggle: Option<Handler<RefGroup>>,
    on_open_panel: Option<Handler<()>>,
) -> AnyElement {
    let group = RefGroup::Section(section);
    SectionHeader::new(section_title(section))
        .count(count)
        .open(open)
        .when_some(on_toggle, |header, on_toggle| {
            header.on_toggle(move |_, window, cx| on_toggle(&group, window, cx))
        })
        .when_some(on_open_panel.filter(|_| section == Section::Worktrees), |header, on_open| {
            header.opens(PANEL_TOOLTIP, move |_, window, cx| on_open(&(), window, cx))
        })
        .into_any_element()
}

fn folder_row(
    kind: RefKind,
    path: &SharedString,
    depth: usize,
    open: bool,
    on_toggle: Option<Handler<RefGroup>>,
    cx: &App,
) -> AnyElement {
    let group = RefGroup::Folder(kind, path.clone());
    list_row(SharedString::from(format!("{kind:?}/{path}/")), false)
        .px_3()
        .child(tree_line(
            depth,
            Some(open),
            muted_icon(folder_icon(kind, depth), cx),
            last_segment(path),
            false,
            cx,
        ))
        .when_some(on_toggle, |row, on_toggle| {
            row.on_click(move |_, window, cx| on_toggle(&group, window, cx))
        })
        .into_any_element()
}

struct RefClicks {
    on_reveal: Option<Handler<usize>>,
    on_check_out: Option<Handler<RefLabel>>,
}

fn ref_row(
    label: &RefLabel,
    ix: usize,
    depth: usize,
    picked: &Picked<ObjectId>,
    mark: Option<PullMark>,
    clicks: RefClicks,
    menus: &RefMenus,
    cx: &App,
) -> AnyElement {
    let RefClicks {
        on_reveal,
        on_check_out,
    } = clicks;
    let on_check_out = on_check_out.filter(|_| label.can_be_checked_out());
    let double_clicked = label.clone();
    let row = list_row(
        SharedString::from(format!("{:?}/{}", label.kind, label.name)),
        picked.contains(&label.target),
    )
    .px_3()
    .child(
        h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(
                tree_line(
                    depth,
                    None,
                    muted_icon(leaf_icon(label.kind), cx),
                    last_segment(&label.name),
                    label.is_head,
                    cx,
                )
                .flex_1(),
            )
            .children(mark.map(|mark| pull_chip(mark, cx)))
            .children(ahead_behind(label, cx))
            .children(label.worktree.clone().map(|worktree| {
                let words = checked_out_in(&worktree);
                state_icon("elsewhere", IconName::FolderGit, cx.theme().muted_foreground, words)
            })),
    )
    .on_click(move |event, window, cx| {
        if let Some(on_reveal) = &on_reveal {
            on_reveal(&ix, window, cx)
        }
        if let Some(on_check_out) = on_check_out.as_ref().filter(|_| is_double_click(event)) {
            on_check_out(&double_clicked, window, cx)
        }
    });
    menus.attach(row, label)
}

fn ahead_behind(label: &RefLabel, cx: &App) -> Option<impl IntoElement> {
    let apart = label.from_upstream.filter(|apart| !apart.is_level())?;
    let words = apart_in_words(apart, label.upstream.as_deref()?);
    let muted = cx.theme().muted_foreground;
    let side = |count: usize, arrow: IconName| {
        (count > 0).then(|| {
            h_flex()
                .gap_px()
                .child(Icon::new(arrow).xsmall().text_color(muted))
                .child(thousands(count))
        })
    };
    Some(
        h_flex()
            .id("ahead-behind")
            .flex_none()
            .gap_1p5()
            .xs_text()
            .text_color(muted)
            .children(side(apart.behind, IconName::ArrowDown))
            .children(side(apart.ahead, IconName::ArrowUp))
            .tooltip(move |window, cx| Tooltip::new(words.clone()).build(window, cx)),
    )
}

fn apart_in_words(apart: AheadBehind, upstream: &str) -> String {
    let pull = format!(
        "{} to pull from {upstream}",
        counted(apart.behind, "commit", "commits")
    );
    let push = format!(
        "{} to push to {upstream}",
        counted(apart.ahead, "commit", "commits")
    );
    match apart {
        AheadBehind { ahead: 0, .. } => pull,
        AheadBehind { behind: 0, .. } => push,
        AheadBehind { ahead, .. } => format!("{pull}, {} to push", thousands(ahead)),
    }
}

fn stash_row(
    stash: &Stash,
    ix: usize,
    picked: &Picked<ObjectId>,
    on_reveal: Option<Handler<ObjectId>>,
    stash_menu: Option<StashMenu>,
    cx: &App,
) -> AnyElement {
    let id = stash.id;
    let row = list_row(SharedString::from(format!("stash/{ix}")), picked.contains(&id))
        .px_3()
        .child(tree_line(
            0,
            None,
            muted_icon(IconName::Archive, cx),
            &stash.message,
            false,
            cx,
        ))
        .when_some(on_reveal, |row, on_reveal| {
            row.on_click(move |_, window, cx| on_reveal(&id, window, cx))
        });
    match stash_menu {
        Some(stash_menu) => {
            let right_clicked = stash.clone();
            row.context_menu(move |menu, _, _| stash_menu.build(menu, &right_clicked))
                .into_any_element()
        }
        None => row.into_any_element(),
    }
}

fn worktree_row(
    worktree: &Worktree,
    ix: usize,
    section: Option<&WorktreeSection>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let (muted, yellow) = (theme.muted_foreground, theme.yellow);
    let changed = section
        .and_then(|section| section.changed.get(ix).copied())
        .unwrap_or_default();
    let selected = section
        .and_then(|section| section.selected.as_deref())
        .is_some_and(|selected| *selected == *worktree.location);
    let checked_out = worktree.checked_out();
    let state = if worktree.is_missing {
        let gone = format!("{} is gone. Forget this worktree to free its branch.", worktree.location.display());
        Some(state_icon("missing", IconName::TriangleAlert, yellow, gone))
    } else if let Some(reason) = &worktree.lock {
        let because = match reason.is_empty() {
            true => "Locked".to_owned(),
            false => format!("Locked: {reason}"),
        };
        Some(state_icon("locked", IconName::Lock, muted, because))
    } else {
        None
    };
    let dirty = (changed > 0).then(|| {
        let words = counted(changed, "changed file", "changed files");
        h_flex()
            .id("changed")
            .flex_none()
            .gap_1()
            .xs_text()
            .text_color(muted)
            .child(div().size(gpui_kit::px(6.)).rounded_full().bg(yellow))
            .child(thousands(changed))
            .tooltip(move |window, cx| Tooltip::new(words.clone()).build(window, cx))
    });
    let name = div()
        .flex_none()
        .max_w(gpui_kit::px(112.))
        .truncate()
        .sm_text()
        .when(worktree.is_here, |this| this.font_semibold())
        .when(worktree.is_missing, |this| this.text_color(muted))
        .child(worktree.name.clone());
    let row = list_row(SharedString::from(format!("worktree/{ix}")), selected)
        .px_3()
        .child(
            h_flex()
                .id("worktree")
                .w_full()
                .min_w_0()
                .gap_1p5()
                .child(div().flex_none().size_3())
                .child(muted_icon(IconName::FolderGit, cx))
                .child(name)
                .child(div().flex_1().min_w_0().truncate().xs_text().text_color(muted).child(checked_out))
                .children(state)
                .children(dirty),
        );
    let Some(section) = section else {
        return row.into_any_element();
    };
    let location = worktree.location.clone();
    let on_select = section.on_select.clone();
    let on_open = (!worktree.is_here && !worktree.is_missing).then(|| section.on_open.clone());
    let menu = section.menu.clone();
    let right_clicked = worktree.clone();
    row.on_click(move |event, window, cx| {
        on_select(&location, window, cx);
        if let Some(on_open) = on_open.as_ref().filter(|_| is_double_click(event)) {
            on_open(&location, window, cx)
        }
    })
    .context_menu(move |popup, _, _| menu.build(popup, &right_clicked))
    .into_any_element()
}

fn state_icon(id: &'static str, icon: IconName, color: gpui_kit::Hsla, words: String) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .child(Icon::new(icon).xsmall().text_color(color))
        .tooltip(move |window, cx| Tooltip::new(words.clone()).build(window, cx))
}

fn muted_icon(name: IconName, cx: &App) -> Icon {
    Icon::new(name)
        .small()
        .text_color(cx.theme().muted_foreground)
}

#[cfg(test)]
mod tests {
    use super::apart_in_words;
    use crate::git::AheadBehind;

    #[test]
    fn the_tooltip_says_what_to_pull_and_push_naming_the_upstream() {
        let words = |ahead, behind| apart_in_words(AheadBehind { ahead, behind }, "origin/main");
        assert_eq!(words(2, 0), "2 commits to push to origin/main");
        assert_eq!(words(0, 1), "1 commit to pull from origin/main");
        assert_eq!(words(3, 1_284), "1,284 commits to pull from origin/main, 3 to push");
    }
}
