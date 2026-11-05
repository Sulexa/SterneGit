use gpui_kit::{
    App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    Direction,
    details::{Version, version_dot},
};
use crate::{
    actions::{LoopChange, MEDIA_CONTEXT, PlayPause, SwitchVersion},
    ui::{Handler, Look as _},
};

const HEIGHT: f32 = 44.;
const PLAY_SIZE: f32 = 28.;

pub(crate) fn clock(seconds: f64, digits: usize) -> String {
    let seconds = seconds.max(0.);
    let minutes = (seconds / 60.).floor();
    let rest = seconds - minutes * 60.;
    let width = if digits == 0 { 2 } else { digits + 3 };
    format!("{}:{rest:0width$.digits$}", minutes as u64)
}

#[derive(IntoElement)]
pub(crate) struct Transport {
    position: f64,
    length: f64,
    playing: bool,
    repeating: bool,
    heard: Option<Version>,
    trouble: Option<SharedString>,
    on_step: Option<Handler<Direction>>,
    on_play: Handler<()>,
    on_repeat: Handler<()>,
    on_hear: Handler<Version>,
}

impl Transport {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        position: f64,
        length: f64,
        playing: bool,
        repeating: bool,
        heard: Option<Version>,
        on_play: Handler<()>,
        on_repeat: Handler<()>,
        on_hear: Handler<Version>,
    ) -> Self {
        Self {
            position,
            length,
            playing,
            repeating,
            heard,
            trouble: None,
            on_step: None,
            on_play,
            on_repeat,
            on_hear,
        }
    }

    pub(crate) fn frame_steps(mut self, on_step: Handler<Direction>) -> Self {
        self.on_step = Some(on_step);
        self
    }

    pub(crate) fn trouble(mut self, trouble: Option<SharedString>) -> Self {
        self.trouble = trouble;
        self
    }
}

impl RenderOnce for Transport {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let on_play = self.on_play.clone();
        let on_repeat = self.on_repeat.clone();
        let play = Button::new("play")
            .primary()
            .icon(if self.playing { IconName::Pause } else { IconName::Play })
            .w(px(PLAY_SIZE))
            .h(px(PLAY_SIZE))
            .rounded_full()
            .tooltip_with_action(
                if self.playing { "Pause" } else { "Play" },
                &PlayPause,
                Some(MEDIA_CONTEXT),
            )
            .on_click(move |_, window, cx| on_play(&(), window, cx));
        let hear = |version: Version| {
            let on_hear = self.on_hear.clone();
            Button::new(match version {
                Version::Before => "hear-before",
                Version::After => "hear-after",
            })
            .ghost()
            .xsmall()
            .selected(self.heard == Some(version))
            .child(
                h_flex()
                    .gap_1p5()
                    .child(version_dot(version, cx))
                    .child(version.label()),
            )
            .tooltip_with_action(
                match version {
                    Version::Before => "Hear the version before the change",
                    Version::After => "Hear the changed version",
                },
                &SwitchVersion,
                Some(MEDIA_CONTEXT),
            )
            .on_click(move |_, window, cx| on_hear(&version, window, cx))
        };
        let step = |id: &'static str, icon: IconName, direction: Direction, tooltip: &'static str| {
            self.on_step.clone().map(|on_step| {
                Button::new(id)
                    .ghost()
                    .xsmall()
                    .icon(icon)
                    .tooltip(tooltip)
                    .on_click(move |_, window, cx| on_step(&direction, window, cx))
            })
        };
        h_flex()
            .flex_none()
            .w_full()
            .h(px(HEIGHT))
            .px_3()
            .gap_1p5()
            .border_t_1()
            .border_color(theme.border)
            .children(step("frame-back", IconName::StepBack, Direction::Back, "Previous frame"))
            .child(play)
            .children(step("frame-forward", IconName::StepForward, Direction::Forward, "Next frame"))
            .child(
                h_flex()
                    .mx_2()
                    .xs_text()
                    .font_family(theme.mono_font_family.clone())
                    .text_color(theme.muted_foreground)
                    .child(div().text_color(theme.foreground).child(clock(self.position, 2)))
                    .child(format!(" / {}", clock(self.length, 2))),
            )
            .child(
                Button::new("loop-change")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Repeat)
                    .label("Loop change")
                    .selected(self.repeating)
                    .tooltip_with_action("Play the current change over and over", &LoopChange, Some(MEDIA_CONTEXT))
                    .on_click(move |_, window, cx| on_repeat(&(), window, cx)),
            )
            .when_some(self.trouble, |this, trouble| {
                this.child(div().ellipsis().xs_text().text_color(theme.red).child(trouble))
            })
            .child(div().flex_1())
            .child(
                h_flex()
                    .gap_0p5()
                    .child(
                        h_flex()
                            .mr_1p5()
                            .gap_1p5()
                            .xs_text()
                            .text_color(theme.muted_foreground)
                            .child(Icon::new(IconName::Headphones).xsmall())
                            .child("Hear"),
                    )
                    .child(hear(Version::Before))
                    .child(hear(Version::After)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::clock;

    #[test]
    fn times_read_as_minutes_and_seconds() {
        assert_eq!(clock(78.4, 2), "1:18.40");
        assert_eq!(clock(161.3, 1), "2:41.3");
        assert_eq!(clock(5., 0), "0:05");
        assert_eq!(clock(-1., 2), "0:00.00");
    }
}
