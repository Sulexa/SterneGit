use gpui_kit::{
    AnyElement, App, Context, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _, StyledExt as _, ThemeMode,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    parts::{Heading, boxed, page, row},
    settings_tab::SettingsTab,
};
use crate::{
    actions::{ResetZoom, WINDOW_CONTEXT, ZoomIn, ZoomOut},
    app::{
        theme::{current_theme, switch_theme},
        zoom::{Zoom, current_zoom, reset_zoom, zoom_in, zoom_out},
    },
    ui::Look as _,
};

const INTERFACE: &str = "Interface";
const INTERFACE_ABOUT: &str = "How SterneGit draws its window on this computer.";
const THEME: &str = "Theme";
const THEME_ABOUT: &str =
    "Dark graphite or light pearl, for the whole window: lists, the commit graph, comparisons, menus and dialogs.";
const DARK: &str = "Dark";
const LIGHT: &str = "Light";
const ZOOM: &str = "Zoom";
const ZOOM_ABOUT: &str = "Makes everything in the window bigger or smaller. Hold Ctrl and turn the mouse wheel \
     to zoom from anywhere; over a picture or a sound being compared, the wheel zooms that instead.";
const ZOOM_OUT: &str = "Zoom out";
const ZOOM_IN: &str = "Zoom in";
const RESET: &str = "Reset";
const BACK_TO_ACTUAL_SIZE: &str = "Back to 100 %";
const PERCENT_WIDTH: f32 = 52.;
const PICKER_RADIUS: f32 = 8.;

pub(crate) fn zoom_label(zoom: Zoom) -> String {
    format!("{} %", zoom.percent())
}

impl SettingsTab {
    pub(super) fn interface_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let zoom = current_zoom(cx);
        let theme_row = setting(THEME, THEME_ABOUT, cx).child(theme_picker(cx)).into_any_element();
        let zoom_row = setting(ZOOM, ZOOM_ABOUT, cx)
            .child(zoom_stepper(zoom))
            .child(
                Button::new("reset-zoom")
                    .ghost()
                    .small()
                    .label(RESET)
                    .disabled(zoom == Zoom::ACTUAL_SIZE)
                    .tooltip_with_action(BACK_TO_ACTUAL_SIZE, &ResetZoom, Some(WINDOW_CONTEXT))
                    .on_click(|_, window, cx| reset_zoom(window, cx)),
            )
            .into_any_element();
        page(
            v_flex()
                .gap_6()
                .child(Heading::new(INTERFACE).about(INTERFACE_ABOUT).render(cx))
                .child(boxed(vec![theme_row, zoom_row], cx)),
        )
        .into_any_element()
    }
}

fn setting(title: &'static str, about: &'static str, cx: &App) -> gpui_kit::Div {
    row().child(
        v_flex()
            .flex_1()
            .min_w_0()
            .gap_0p5()
            .child(div().sm_text().font_medium().child(title))
            .child(div().xs_text().text_color(cx.theme().muted_foreground).child(about)),
    )
}

fn theme_picker(cx: &App) -> impl IntoElement {
    let current = current_theme(cx);
    let picked = cx.theme().list_active;
    let choice = |id: &'static str, mode: ThemeMode, icon: IconName, label: &'static str| {
        Button::new(id)
            .ghost()
            .small()
            .icon(icon)
            .label(label)
            .selected(mode == current)
            .when(mode == current, |button| button.bg(picked))
            .on_click(move |_, _, cx| switch_theme(mode, cx))
    };
    h_flex()
        .flex_none()
        .gap_0p5()
        .p_0p5()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(PICKER_RADIUS))
        .child(choice("dark-theme", ThemeMode::Dark, IconName::Moon, DARK))
        .child(choice("light-theme", ThemeMode::Light, IconName::Sun, LIGHT))
}

fn zoom_stepper(zoom: Zoom) -> impl IntoElement {
    h_flex()
        .flex_none()
        .gap_1()
        .child(
            Button::new("zoom-out")
                .small()
                .icon(IconName::Minus)
                .disabled(zoom.smaller().is_none())
                .tooltip_with_action(ZOOM_OUT, &ZoomOut, Some(WINDOW_CONTEXT))
                .on_click(|_, window, cx| zoom_out(window, cx)),
        )
        .child(
            div()
                .w(px(PERCENT_WIDTH))
                .text_center()
                .sm_text()
                .child(zoom_label(zoom)),
        )
        .child(
            Button::new("zoom-in")
                .small()
                .icon(IconName::Plus)
                .disabled(zoom.bigger().is_none())
                .tooltip_with_action(ZOOM_IN, &ZoomIn, Some(WINDOW_CONTEXT))
                .on_click(|_, window, cx| zoom_in(window, cx)),
        )
}
