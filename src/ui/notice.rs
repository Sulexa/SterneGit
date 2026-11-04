use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _,
    Window,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, spinner::Spinner, v_flex},
    div, px,
};

use super::look::Look as _;

pub(crate) fn loading(message: &'static str) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(Spinner::new())
        .child(div().sm_text().child(message))
        .into_any_element()
}

pub(crate) fn hint(message: &'static str, cx: &App) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .sm_text()
        .text_color(cx.theme().muted_foreground)
        .child(message)
        .into_any_element()
}

pub(crate) fn failure(title: &'static str, reason: SharedString, cx: &App) -> AnyElement {
    v_flex()
        .size_full()
        .p_4()
        .gap_2()
        .child(div().sm_text().font_medium().child(title))
        .child(
            div()
                .sm_text()
                .text_color(cx.theme().muted_foreground)
                .child(reason),
        )
        .into_any_element()
}

#[derive(IntoElement)]
pub(crate) struct Notice {
    title: SharedString,
    description: SharedString,
    action: AnyElement,
}

impl Notice {
    pub(crate) fn new(
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        action: impl IntoElement,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            action: action.into_any_element(),
        }
    }
}

impl RenderOnce for Notice {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(Icon::new(IconName::GitMerge).large().text_color(muted))
            .child(div().lg_text().font_medium().child(self.title))
            .child(
                div()
                    .max_w(px(520.))
                    .sm_text()
                    .text_center()
                    .text_color(muted)
                    .child(self.description),
            )
            .child(self.action)
    }
}
