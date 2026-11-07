use gpui_kit::{App, DisplayId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opening {
    InFront,
    Aside,
}

impl Opening {
    pub(super) fn takes_focus(self) -> bool {
        self == Opening::InFront
    }

    pub(super) fn display(self, cx: &App) -> Option<DisplayId> {
        match self {
            Opening::InFront => None,
            Opening::Aside => rightmost_display(cx),
        }
    }
}

fn rightmost_display(cx: &App) -> Option<DisplayId> {
    cx.displays()
        .into_iter()
        .max_by_key(|display| display.bounds().origin.x)
        .map(|display| display.id())
}
