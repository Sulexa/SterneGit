use gpui_kit::{App, Hsla, SharedString, component::ActiveTheme as _};

pub(crate) const LANE_COLORS: usize = 6;

#[derive(Clone)]
pub(crate) struct Palette {
    pub(crate) lanes: [Hsla; LANE_COLORS],
    pub(crate) muted: Hsla,
    pub(crate) mono: SharedString,
}

impl Palette {
    pub(crate) fn new(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            lanes: [
                theme.blue,
                theme.magenta,
                theme.green,
                theme.yellow,
                theme.cyan,
                theme.red,
            ],
            muted: theme.muted_foreground,
            mono: theme.mono_font_family.clone(),
        }
    }

    pub(crate) fn lane(&self, lane: u16) -> Hsla {
        self.lanes[lane as usize % LANE_COLORS]
    }
}
