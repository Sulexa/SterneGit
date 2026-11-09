use gpui_kit::{
    AnyElement, App, Hsla, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, h_flex},
    div,
    prelude::FluentBuilder as _,
    px, rgba,
};

use crate::{
    git::ChangeKind,
    repo::compare::Version,
    ui::{Look as _, change_badge},
    unity::{Glyph, Reference, State, Value, hex_color, short_number},
};

const SWATCH_WIDTH: f32 = 30.;
const SWATCH_HEIGHT: f32 = 16.;
const CELL_TINT: f32 = 0.09;
const WORD_TINT: f32 = 0.3;

pub(crate) fn side_color(version: Version, cx: &App) -> Hsla {
    match version {
        Version::Before => cx.theme().red,
        Version::After => cx.theme().green,
    }
}

pub(crate) fn cell_tint(version: Version, cx: &App) -> Hsla {
    side_color(version, cx).opacity(CELL_TINT)
}

pub(crate) fn state_badge(state: State, cx: &App) -> Option<AnyElement> {
    let kind = match state {
        State::Added => ChangeKind::Added,
        State::Changed => ChangeKind::Modified,
        State::Removed => ChangeKind::Deleted,
        State::Same => return None,
    };
    Some(change_badge(kind, cx))
}

pub(crate) fn glyph_icon(glyph: Glyph) -> IconName {
    match glyph {
        Glyph::Object | Glyph::Prefab => IconName::Box,
        Glyph::Asset => IconName::FileBox,
        Glyph::Script => IconName::FileCode,
        Glyph::Image => IconName::Image,
        Glyph::Sound => IconName::AudioLines,
        Glyph::Material => IconName::CircleDot,
        Glyph::Clip => IconName::Spline,
    }
}

fn marked(text: String, changed: bool, version: Version, cx: &App) -> AnyElement {
    div()
        .px(px(3.))
        .rounded(px(2.))
        .when(changed, |this| this.bg(side_color(version, cx).opacity(WORD_TINT)))
        .child(text)
        .into_any_element()
}

pub(crate) fn reference_chip(reference: &Reference, cx: &App) -> AnyElement {
    let theme = cx.theme();
    h_flex()
        .flex_none()
        .h(px(20.))
        .px_1p5()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .xs_text()
        .child(Icon::new(glyph_icon(reference.glyph)).xsmall().text_color(if reference.glyph == Glyph::Prefab { theme.blue } else { theme.muted_foreground }))
        .child(div().when(reference.missing, |this| this.text_color(theme.red)).child(reference.name.clone()))
        .child(div().text_color(theme.muted_foreground).child(format!("({})", reference.kind)))
        .into_any_element()
}

pub(crate) fn value_cell(value: &Value, other: Option<&Value>, version: Version, cx: &App) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let differs = other.is_some_and(|other| other != value);
    match value {
        Value::Text(text) | Value::Number(text) => marked(text.clone(), differs && !text.is_empty(), version, cx),
        Value::Flag(on) => {
            let theme = cx.theme();
            h_flex()
                .gap_1p5()
                .child(
                    div()
                        .flex_none()
                        .size(px(14.))
                        .rounded(px(3.))
                        .border_1()
                        .border_color(if *on { theme.foreground } else { theme.ring })
                        .when(*on, |this| this.bg(theme.foreground))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(*on, |this| this.child(Icon::new(IconName::Check).xsmall().text_color(theme.background))),
                )
                .child(marked(if *on { "On" } else { "Off" }.to_owned(), differs, version, cx))
                .into_any_element()
        }
        Value::Vector(axes) => {
            let others = match other {
                Some(Value::Vector(others)) => Some(others),
                _ => None,
            };
            h_flex()
                .gap_3()
                .children(axes.iter().map(|(axis, number)| {
                    let changed = others.is_some_and(|others| others.iter().find(|(name, _)| name == axis).map(|(_, value)| value) != Some(number));
                    h_flex()
                        .gap_1()
                        .child(div().text_color(muted).child(axis.to_uppercase()))
                        .child(marked(number.clone(), changed, version, cx))
                }))
                .into_any_element()
        }
        Value::Color { rgba: channels, hdr } => {
            let (other_rgba, other_hdr) = match other {
                Some(Value::Color { rgba, hdr }) => (Some(*rgba), Some(*hdr)),
                _ => (None, None),
            };
            let [r, g, b, _] = channels.map(|channel| (channel.clamp(0., 1.) * 255.).round() as u32);
            h_flex()
                .gap_2()
                .child(
                    div()
                        .flex_none()
                        .w(px(SWATCH_WIDTH))
                        .h(px(SWATCH_HEIGHT))
                        .rounded(px(3.))
                        .border_1()
                        .border_color(cx.theme().ring)
                        .bg(rgba((r << 24) | (g << 16) | (b << 8) | 0xff)),
                )
                .child(
                    div()
                        .font_family(cx.theme().mono_font_family.clone())
                        .child(marked(hex_color(*channels), other_rgba.is_some_and(|other| other != *channels), version, cx)),
                )
                .when_some(*hdr, |this, intensity| {
                    this.child(marked(
                        format!("Intensity {}", short_number(f64::from(intensity))),
                        other_hdr.is_some_and(|other| other != Some(intensity)),
                        version,
                        cx,
                    ))
                })
                .into_any_element()
        }
        Value::Reference(reference) => reference_chip(reference, cx),
        Value::Nothing(kind) => div().text_color(muted).child(format!("None ({kind})")).into_any_element(),
        Value::Count(_) => div().text_color(muted).child(value.text()).into_any_element(),
    }
}

pub(crate) fn hatched(cx: &App) -> gpui_kit::Background {
    gpui_kit::pattern_slash(cx.theme().foreground.opacity(0.05), 4., 4.)
}
