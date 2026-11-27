use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use sternegit_reader_protocol::Warning;

use super::{UnityCompare, UnityEvent};
use crate::ui::Look as _;

fn place_icon(name: &str) -> IconName {
    if name.ends_with(".unity") {
        IconName::Map
    } else if name.ends_with(".prefab") {
        IconName::Box
    } else {
        IconName::FileBox
    }
}

pub(super) fn warning(warning: Warning, cx: &mut Context<UnityCompare>) -> AnyElement {
    let theme = cx.theme();
    let places = warning.places.iter().map(|place| {
        let (folder, name) = place.path.rsplit_once('/').unwrap_or(("", place.path.as_str()));
        h_flex()
            .h(px(26.))
            .px_2()
            .gap_2()
            .rounded(px(4.))
            .bg(theme.foreground.opacity(0.03))
            .child(Icon::new(place_icon(name)).xsmall().text_color(theme.muted_foreground))
            .child(div().flex_none().sm_text().child(name.to_owned()))
            .child(div().flex_none().xs_text().text_color(theme.muted_foreground).child(folder.to_owned()))
            .child(div().flex_1())
            .child(div().xs_text().ellipsis().child(place.details.join(" · ")))
    });
    let buttons = warning.fixes.iter().enumerate().map(|(ix, offer)| {
        let id = offer.id.clone();
        Button::new(("reader-fix", ix))
            .small()
            .when(ix == 0, |button| button.primary())
            .label(offer.label.clone())
            .when(!offer.about.is_empty(), |button| button.tooltip(offer.about.clone()))
            .on_click(cx.listener(move |_, _, _, cx| cx.emit(UnityEvent::Fix(id.clone()))))
    });
    v_flex()
        .flex_none()
        .m_3()
        .p_3()
        .gap_1p5()
        .rounded(px(6.))
        .border_1()
        .border_color(theme.yellow.opacity(0.45))
        .bg(theme.yellow.opacity(0.07))
        .child(
            h_flex()
                .gap_2()
                .child(Icon::new(IconName::TriangleAlert).small().text_color(theme.yellow))
                .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(warning.title.clone())),
        )
        .child(div().xs_text().text_color(theme.muted_foreground).child(warning.about.clone()))
        .when(!warning.places.is_empty(), |this| this.child(v_flex().gap_0p5().children(places)))
        .when(!warning.fixes.is_empty(), |this| this.child(h_flex().gap_2().children(buttons)))
        .into_any_element()
}
