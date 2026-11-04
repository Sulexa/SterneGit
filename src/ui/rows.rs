use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, list::ListItem},
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    handler::{Handler, handler},
    look::Look as _,
    text::thousands,
};

pub(crate) const ROW_HEIGHT: Pixels = px(28.);

pub(crate) fn list_row(id: impl Into<ElementId>, selected: bool) -> ListItem {
    ListItem::new(id)
        .selected(selected)
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
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
    count: Option<usize>,
    action: Option<AnyElement>,
    open: Option<bool>,
    on_toggle: Option<Handler<()>>,
}

impl SectionHeader {
    pub(crate) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            count: None,
            action: None,
            open: None,
            on_toggle: None,
        }
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
            .text_color(cx.theme().muted_foreground)
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
