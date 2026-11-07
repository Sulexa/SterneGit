use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div, px,
};

use super::{
    parts::{Heading, boxed, page, row},
    settings_tab::SettingsTab,
};
use crate::{
    actions::{ResetZoom, WINDOW_CONTEXT, ZoomIn, ZoomOut},
    app::zoom::{Zoom, current_zoom, reset_zoom, zoom_in, zoom_out},
    ui::Look as _,
};

const INTERFACE: &str = "Interface";
const INTERFACE_ABOUT: &str = "How SterneGit draws its window on this computer.";
const ZOOM: &str = "Zoom";
const ZOOM_ABOUT: &str = "Makes everything in the window bigger or smaller. Hold Ctrl and turn the mouse wheel \
     to zoom from anywhere; over a picture or a sound being compared, the wheel zooms that instead.";
const ZOOM_OUT: &str = "Zoom out";
const ZOOM_IN: &str = "Zoom in";
const RESET: &str = "Reset";
const BACK_TO_ACTUAL_SIZE: &str = "Back to 100 %";
const PERCENT_WIDTH: f32 = 52.;

pub(crate) fn zoom_label(zoom: Zoom) -> String {
    format!("{} %", zoom.percent())
}

impl SettingsTab {
    pub(super) fn interface_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let zoom = current_zoom(cx);
        let muted = cx.theme().muted_foreground;
        let zoom_row = row()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(div().sm_text().font_medium().child(ZOOM))
                    .child(div().xs_text().text_color(muted).child(ZOOM_ABOUT)),
            )
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
                .child(boxed(vec![zoom_row], cx)),
        )
        .into_any_element()
    }
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
