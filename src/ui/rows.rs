use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement as _, Pixels, RenderOnce, SharedString,
    Styled as _, Window,
    component::{ActiveTheme as _, StyledExt as _, h_flex, list::ListItem},
    div, px,
};

use super::text::thousands;

pub(crate) const ROW_HEIGHT: Pixels = px(28.);

pub(crate) fn list_row(id: impl Into<ElementId>, selected: bool) -> ListItem {
    ListItem::new(id)
        .selected(selected)
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
}

#[derive(IntoElement)]
pub(crate) struct SectionHeader {
    title: SharedString,
    count: Option<usize>,
    action: Option<AnyElement>,
}

impl SectionHeader {
    pub(crate) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            count: None,
            action: None,
        }
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
            .w_full()
            .h(ROW_HEIGHT)
            .px_3()
            .gap_2()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().muted_foreground)
            .child(self.title)
            .child(div().flex_1())
            .children(self.count.map(thousands))
            .children(self.action)
    }
}
