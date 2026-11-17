use std::ops::Range;

use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    StatefulInteractiveElement as _, Styled as _, Window,
    component::{ActiveTheme as _, diff::DiffChangeMark, orange_400},
    div, px, relative,
};

use crate::ui::Handler;

const WIDTH: f32 = 12.;
const MARK_INSET: f32 = 3.;
const SHORTEST_MARK: f32 = 3.;
const SHOWN_FILL: f32 = 0.07;
const SHOWN_EDGE: f32 = 0.18;
const FOUND_HEIGHT: f32 = 2.;
const FOUND_PLACES: usize = 512;

#[derive(IntoElement)]
pub(crate) struct ChangeRuler {
    marks: Vec<DiffChangeMark>,
    rows: usize,
    shown: Range<usize>,
    found: Vec<usize>,
    on_jump: Handler<usize>,
}

impl ChangeRuler {
    pub(crate) fn new(
        marks: Vec<DiffChangeMark>,
        rows: usize,
        shown: Range<usize>,
        on_jump: Handler<usize>,
    ) -> Self {
        Self {
            marks,
            rows,
            shown,
            found: Vec::new(),
            on_jump,
        }
    }

    pub(crate) fn found(mut self, rows: Vec<usize>) -> Self {
        self.found = rows;
        self
    }
}

impl RenderOnce for ChangeRuler {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let rows = self.rows.max(1) as f32;
        let fraction = |row: usize| relative(row as f32 / rows);
        let shown = div()
            .absolute()
            .left_0()
            .right_0()
            .top(fraction(self.shown.start))
            .h(fraction(self.shown.len()))
            .bg(theme.foreground.opacity(SHOWN_FILL))
            .border_y_1()
            .border_color(theme.foreground.opacity(SHOWN_EDGE));
        let marks = self.marks.iter().enumerate().map(|(ix, mark)| {
            let color = match (mark.adds, mark.removes) {
                (true, true) => theme.yellow,
                (true, false) => theme.green,
                _ => theme.red,
            };
            let on_jump = self.on_jump.clone();
            div()
                .id(("change-mark", ix))
                .absolute()
                .left(px(MARK_INSET))
                .right(px(MARK_INSET))
                .top(fraction(mark.first_row))
                .h(fraction(mark.rows))
                .min_h(px(SHORTEST_MARK))
                .rounded(px(1.))
                .bg(color)
                .cursor_pointer()
                .on_click(move |_, window, cx| on_jump(&ix, window, cx))
        });
        let mut places: Vec<usize> = self
            .found
            .iter()
            .map(|row| row * FOUND_PLACES / self.rows.max(1))
            .collect();
        places.sort_unstable();
        places.dedup();
        let found = places.into_iter().map(|place| {
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(relative(place as f32 / FOUND_PLACES as f32))
                .h(px(FOUND_HEIGHT))
                .bg(orange_400())
        });
        div()
            .relative()
            .flex_none()
            .w(px(WIDTH))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.25))
            .child(shown)
            .children(marks)
            .children(found)

    }
}
