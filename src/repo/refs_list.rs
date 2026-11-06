use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, UniformListScrollHandle, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, h_flex, menu::ContextMenuExt as _,
        scroll::Scrollbar, tooltip::Tooltip,
    },
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::{
    branch_menu::BranchMenu,
    pulls::{PullMarks, PullsSection},
    ref_tree::{RefGroup, RefTreeRow, Section, last_segment, ref_tree},
    stash_menu::StashMenu,
};
use crate::{
    git::{AheadBehind, History, RefKind, RefLabel, Stash},
    ui::{
        Folded, Handler, Look as _, PullMark, SectionHeader, counted, handler, is_double_click,
        list_row, pull_chip, thousands, tree_line,
    },
};

#[derive(IntoElement)]
pub(crate) struct RefsList {
    history: Arc<History>,
    rows: Vec<RefTreeRow>,
    selected: Option<ObjectId>,
    scroll: UniformListScrollHandle,
    on_reveal: Option<Handler<usize>>,
    on_reveal_stash: Option<Handler<ObjectId>>,
    on_check_out: Option<Handler<RefLabel>>,
    on_toggle: Option<Handler<RefGroup>>,
    branch_menu: Option<BranchMenu>,
    stash_menu: Option<StashMenu>,
    pulls: Option<PullsSection>,
    marks: Arc<PullMarks>,
}

impl RefsList {
    pub(crate) fn new(
        history: Arc<History>,
        folded: &Folded<RefGroup>,
        scroll: UniformListScrollHandle,
    ) -> Self {
        Self {
            rows: ref_tree(&history.refs, history.stashes.len(), folded),
            history,
            selected: None,
            scroll,
            on_reveal: None,
            on_reveal_stash: None,
            on_check_out: None,
            on_toggle: None,
            branch_menu: None,
            stash_menu: None,
            pulls: None,
            marks: Arc::default(),
        }
    }

    pub(crate) fn pulls(mut self, pulls: Option<PullsSection>, marks: Arc<PullMarks>) -> Self {
        self.pulls = pulls;
        self.marks = marks;
        self
    }

    pub(crate) fn branch_menu(mut self, menu: BranchMenu) -> Self {
        self.branch_menu = Some(menu);
        self
    }

    pub(crate) fn stash_menu(mut self, menu: StashMenu) -> Self {
        self.stash_menu = Some(menu);
        self
    }

    pub(crate) fn selected(mut self, commit: Option<ObjectId>) -> Self {
        self.selected = commit;
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
            selected,
            scroll,
            on_reveal,
            on_reveal_stash,
            on_check_out,
            on_toggle,
            branch_menu,
            stash_menu,
            pulls,
            marks,
        } = self;
        let pull_rows = pulls.as_ref().map_or(0, PullsSection::len);
        let handlers = RowHandlers {
            on_reveal,
            on_reveal_stash,
            on_check_out,
            on_toggle,
            branch_menu,
            stash_menu,
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
                            Some(ix) => ref_tree_row(&rows[ix], &history, selected, &marks, &handlers, cx),
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
    branch_menu: Option<BranchMenu>,
    stash_menu: Option<StashMenu>,
}

fn ref_tree_row(
    row: &RefTreeRow,
    history: &History,
    selected: Option<ObjectId>,
    marks: &PullMarks,
    handlers: &RowHandlers,
    cx: &App,
) -> AnyElement {
    let RowHandlers {
        on_reveal,
        on_reveal_stash,
        on_check_out,
        on_toggle,
        branch_menu,
        stash_menu,
    } = handlers;
    match row {
        &RefTreeRow::Section {
            section,
            count,
            open,
        } => section_header(section, count, open, on_toggle.clone()),
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
            selected,
            marks.get(&history.refs[ix].full_name()).copied(),
            RefClicks {
                on_reveal: on_reveal.clone(),
                on_check_out: on_check_out.clone(),
            },
            branch_menu.clone(),
            cx,
        ),
        &RefTreeRow::Stash { ix } => stash_row(
            &history.stashes[ix],
            ix,
            selected,
            on_reveal_stash.clone(),
            stash_menu.clone(),
            cx,
        ),
    }
}

fn section_title(section: Section) -> &'static str {
    match section {
        Section::Refs(RefKind::LocalBranch) => "Branches",
        Section::Refs(RefKind::RemoteBranch) => "Remotes",
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
) -> AnyElement {
    let group = RefGroup::Section(section);
    SectionHeader::new(section_title(section))
        .count(count)
        .open(open)
        .when_some(on_toggle, |header, on_toggle| {
            header.on_toggle(move |_, window, cx| on_toggle(&group, window, cx))
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
    selected: Option<ObjectId>,
    mark: Option<PullMark>,
    clicks: RefClicks,
    branch_menu: Option<BranchMenu>,
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
        selected == Some(label.target),
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
            .children(ahead_behind(label, cx)),
    )
    .on_click(move |event, window, cx| {
        if let Some(on_reveal) = &on_reveal {
            on_reveal(&ix, window, cx)
        }
        if let Some(on_check_out) = on_check_out.as_ref().filter(|_| is_double_click(event)) {
            on_check_out(&double_clicked, window, cx)
        }
    });
    match branch_menu.filter(|_| label.is_branch()) {
        Some(branch_menu) => {
            let right_clicked = label.clone();
            row.context_menu(move |menu, _, _| branch_menu.build(menu, &right_clicked))
                .into_any_element()
        }
        None => row.into_any_element(),
    }
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
    selected: Option<ObjectId>,
    on_reveal: Option<Handler<ObjectId>>,
    stash_menu: Option<StashMenu>,
    cx: &App,
) -> AnyElement {
    let id = stash.id;
    let row = list_row(SharedString::from(format!("stash/{ix}")), selected == Some(id))
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
