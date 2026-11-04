use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, scroll::Scrollbar},
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use crate::{
    git::{History, RefKind, RefLabel},
    ui::{Handler, Look as _, SectionHeader, SectionRow, Sections, handler, list_row, ref_icon},
};

#[derive(IntoElement)]
pub(crate) struct RefsList {
    history: Arc<History>,
    selected: Option<ObjectId>,
    scroll: UniformListScrollHandle,
    on_reveal: Option<Handler<usize>>,
}

impl RefsList {
    pub(crate) fn new(history: Arc<History>, scroll: UniformListScrollHandle) -> Self {
        Self {
            history,
            selected: None,
            scroll,
            on_reveal: None,
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
}

impl RenderOnce for RefsList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            history,
            selected,
            scroll,
            on_reveal,
        } = self;
        let sections = ref_sections(&history.refs);
        div()
            .id("refs")
            .relative()
            .size_full()
            .min_h_0()
            .bg(cx.theme().sidebar)
            .child(
                uniform_list("ref-list", sections.rows(), move |range, _, cx| {
                    range
                        .filter_map(|ix| sections.row(ix))
                        .map(|row| match row {
                            SectionRow::Header((kind, _), count) => {
                                SectionHeader::new(section_title(kind))
                                    .count(count)
                                    .into_any_element()
                            }
                            SectionRow::Item((_, first), offset) => {
                                let ix = first + offset;
                                ref_row(&history.refs[ix], ix, selected, on_reveal.clone(), cx)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll))
    }
}

fn ref_sections(refs: &[RefLabel]) -> Sections<(RefKind, usize)> {
    let mut first = 0;
    let mut sections = Vec::new();
    for kind in RefKind::ALL {
        let count = refs[first..].partition_point(|label| label.kind == kind);
        if count > 0 {
            sections.push(((kind, first), count));
        }
        first += count;
    }
    Sections::new(sections)
}

fn section_title(kind: RefKind) -> &'static str {
    match kind {
        RefKind::LocalBranch => "Branches",
        RefKind::RemoteBranch => "Remotes",
        RefKind::Tag => "Tags",
    }
}

fn ref_row(
    label: &RefLabel,
    ix: usize,
    selected: Option<ObjectId>,
    on_reveal: Option<Handler<usize>>,
    cx: &App,
) -> AnyElement {
    list_row(
        SharedString::from(format!("{:?}/{}", label.kind, label.name)),
        selected == Some(label.target),
    )
    .px_3()
    .child(
        h_flex()
            .gap_2()
            .min_w_0()
            .child(
                Icon::new(ref_icon(label.kind))
                    .small()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .flex_1()
                    .ellipsis()
                    .text_sm()
                    .when(label.is_head, |this| this.font_semibold())
                    .child(label.name.clone()),
            ),
    )
    .when_some(on_reveal, |row, on_reveal| {
        row.on_click(move |_, window, cx| on_reveal(&ix, window, cx))
    })
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::ref_sections;
    use crate::{
        git::{RefKind, RefLabel, fixtures::id},
        ui::SectionRow,
    };

    fn label(name: &str, kind: RefKind) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind,
            target: id(1),
            is_head: false,
        }
    }

    #[test]
    fn sections_skip_empty_kinds_and_point_at_each_ref() {
        let refs = [
            label("main", RefKind::LocalBranch),
            label("topic", RefKind::LocalBranch),
            label("v1", RefKind::Tag),
        ];
        let sections = ref_sections(&refs);
        let rows: Vec<_> = (0..sections.rows())
            .filter_map(|ix| sections.row(ix))
            .collect();
        assert_eq!(
            rows,
            [
                SectionRow::Header((RefKind::LocalBranch, 0), 2),
                SectionRow::Item((RefKind::LocalBranch, 0), 0),
                SectionRow::Item((RefKind::LocalBranch, 0), 1),
                SectionRow::Header((RefKind::Tag, 2), 1),
                SectionRow::Item((RefKind::Tag, 2), 0),
            ]
        );
    }
}
