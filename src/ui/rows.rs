use gpui_kit::{
    AnyElement, App, Div, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, list::ListItem},
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    badges::change_badge,
    handler::{Handler, handler},
    look::Look as _,
    text::thousands,
};
use crate::git::ChangeKind;

pub(crate) const ROW_HEIGHT: Pixels = px(28.);

const TREE_INDENT: Pixels = px(18.);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilePlace {
    Listed,
    InTree { depth: usize },
}

pub(crate) fn list_row(id: impl Into<ElementId>, selected: bool) -> ListItem {
    ListItem::new(id)
        .selected(selected)
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
}

pub(crate) fn file_row(
    id: impl Into<ElementId>,
    kind: ChangeKind,
    path: &str,
    place: FilePlace,
    selected: bool,
    trailing: impl IntoElement,
    cx: &App,
) -> ListItem {
    let (name, folder) = match path.rsplit_once('/') {
        Some((folder, name)) => (name, Some(folder)),
        None => (path, None),
    };
    let label = match place {
        FilePlace::Listed => h_flex()
            .flex_1()
            .min_w_0()
            .gap_2()
            .child(change_badge(kind, cx))
            .child(name_and_folder(name, folder, cx)),
        FilePlace::InTree { depth } => {
            tree_line(depth, None, change_badge(kind, cx), name, false, cx).flex_1()
        }
    };
    list_row(id, selected).pl_3().pr_1().child(
        h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(label)
            .child(trailing),
    )
}

fn name_and_folder(name: &str, folder: Option<&str>, cx: &App) -> Div {
    h_flex()
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .overflow_hidden()
        .child(
            div()
                .flex_none()
                .max_w_full()
                .truncate()
                .sm_text()
                .child(name.to_owned()),
        )
        .when_some(folder, |this, folder| {
            this.child(
                div()
                    .ellipsis()
                    .xs_text()
                    .text_color(cx.theme().muted_foreground)
                    .child(folder.to_owned()),
            )
        })
}

pub(crate) fn folder_row(
    id: impl Into<ElementId>,
    name: &str,
    depth: usize,
    open: bool,
    trailing: Option<AnyElement>,
    cx: &App,
) -> ListItem {
    let folder = Icon::new(IconName::Folder)
        .small()
        .text_color(cx.theme().muted_foreground);
    list_row(id, false).pl_3().pr_1().child(
        h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(tree_line(depth, Some(open), folder, name, false, cx).flex_1())
            .children(trailing),
    )
}

pub(crate) fn tree_line(
    depth: usize,
    open: Option<bool>,
    lead: impl IntoElement,
    name: &str,
    bold: bool,
    cx: &App,
) -> Div {
    let muted = cx.theme().muted_foreground;
    h_flex()
        .gap_1p5()
        .min_w_0()
        .pl(TREE_INDENT * depth as f32)
        .child(match open {
            Some(open) => fold_icon(open).text_color(muted).into_any_element(),
            None => div().flex_none().size_3().into_any_element(),
        })
        .child(lead)
        .child(
            div()
                .flex_1()
                .ellipsis()
                .sm_text()
                .when(bold, |this| this.font_semibold())
                .child(SharedString::from(name.to_owned())),
        )
}

pub(crate) fn fold_icon(open: bool) -> Icon {
    Icon::new(if open {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    })
    .xsmall()
}

#[derive(IntoElement)]
pub(crate) struct SectionHeader {
    title: SharedString,
    color: Option<Hsla>,
    count: Option<usize>,
    action: Option<AnyElement>,
    open: Option<bool>,
    on_toggle: Option<Handler<()>>,
}

impl SectionHeader {
    pub(crate) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            color: None,
            count: None,
            action: None,
            open: None,
            on_toggle: None,
        }
    }

    pub(crate) fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub(crate) fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub(crate) fn on_toggle(
        mut self,
        callback: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(handler(callback));
        self
    }

    pub(crate) fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .id(self.title.clone())
            .w_full()
            .h(ROW_HEIGHT)
            .px_3()
            .gap_2()
            .xs_text()
            .font_medium()
            .text_color(self.color.unwrap_or(cx.theme().muted_foreground))
            .when_some(self.open, |this, open| {
                this.gap_1p5().child(fold_icon(open))
            })
            .child(self.title)
            .child(div().flex_1())
            .children(self.count.map(thousands))
            .children(self.action)
            .when_some(self.on_toggle, |this, on_toggle| {
                this.on_click(move |_, window, cx| on_toggle(&(), window, cx))
            })
    }
}
