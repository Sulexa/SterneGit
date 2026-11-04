use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, RenderOnce,
    SharedString, Styled as _, UniformListScrollHandle, Window,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, scroll::Scrollbar},
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};

use super::ref_tree::{Folded, RefGroup, RefTreeRow, last_segment, ref_tree};
use crate::{
    git::{History, RefKind, RefLabel},
    ui::{Handler, Look as _, SectionHeader, fold_icon, handler, list_row},
};

const INDENT: Pixels = px(18.);

#[derive(IntoElement)]
pub(crate) struct RefsList {
    history: Arc<History>,
    rows: Vec<RefTreeRow>,
    selected: Option<ObjectId>,
    scroll: UniformListScrollHandle,
    on_reveal: Option<Handler<usize>>,
    on_toggle: Option<Handler<RefGroup>>,
}

impl RefsList {
    pub(crate) fn new(
        history: Arc<History>,
        folded: &Folded,
        scroll: UniformListScrollHandle,
    ) -> Self {
        Self {
            rows: ref_tree(&history.refs, folded),
            history,
            selected: None,
            scroll,
            on_reveal: None,
            on_toggle: None,
        }
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
            on_toggle,
        } = self;
        div()
            .id("refs")
            .relative()
            .size_full()
            .min_h_0()
            .bg(cx.theme().sidebar)
            .child(
                uniform_list("ref-list", rows.len(), move |range, _, cx| {
                    rows[range]
                        .iter()
                        .map(|row| match row {
                            &RefTreeRow::Section { kind, count, open } => {
                                section_header(kind, count, open, on_toggle.clone())
                            }
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
                                on_reveal.clone(),
                                cx,
                            ),
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll))
    }
}

fn section_title(kind: RefKind) -> &'static str {
    match kind {
        RefKind::LocalBranch => "Branches",
        RefKind::RemoteBranch => "Remotes",
        RefKind::Tag => "Tags",
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
    kind: RefKind,
    count: usize,
    open: bool,
    on_toggle: Option<Handler<RefGroup>>,
) -> AnyElement {
    let group = RefGroup::Section(kind);
    SectionHeader::new(section_title(kind))
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
            folder_icon(kind, depth),
            last_segment(path),
            false,
            cx,
        ))
        .when_some(on_toggle, |row, on_toggle| {
            row.on_click(move |_, window, cx| on_toggle(&group, window, cx))
        })
        .into_any_element()
}

fn ref_row(
    label: &RefLabel,
    ix: usize,
    depth: usize,
    selected: Option<ObjectId>,
    on_reveal: Option<Handler<usize>>,
    cx: &App,
) -> AnyElement {
    list_row(
        SharedString::from(format!("{:?}/{}", label.kind, label.name)),
        selected == Some(label.target),
    )
    .px_3()
    .child(tree_line(
        depth,
        None,
        leaf_icon(label.kind),
        last_segment(&label.name),
        label.is_head,
        cx,
    ))
    .when_some(on_reveal, |row, on_reveal| {
        row.on_click(move |_, window, cx| on_reveal(&ix, window, cx))
    })
    .into_any_element()
}

fn tree_line(
    depth: usize,
    open: Option<bool>,
    icon: IconName,
    name: &str,
    bold: bool,
    cx: &App,
) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    h_flex()
        .gap_1p5()
        .min_w_0()
        .pl(INDENT * depth as f32)
        .child(match open {
            Some(open) => fold_icon(open).text_color(muted).into_any_element(),
            None => div().flex_none().size_3().into_any_element(),
        })
        .child(Icon::new(icon).small().text_color(muted))
        .child(
            div()
                .flex_1()
                .ellipsis()
                .sm_text()
                .when(bold, |this| this.font_semibold())
                .child(SharedString::from(name.to_owned())),
        )
}
