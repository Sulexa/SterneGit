mod pixels;

use std::{
    ops::Range,
    sync::OnceLock,
    time::{Duration, Instant},
};

use gpui_kit::{
    App, Bounds, IntoElement, ParentElement as _, Pixels, Styled as _, Window, canvas,
    component::button::Button, div, fill, point, prelude::FluentBuilder as _, px, rgb,
};

const HEAD_ROWS: usize = 6;
const NOD: Duration = Duration::from_millis(400);
const DIPPED_FROM: Duration = Duration::from_millis(260);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    row: usize,
    columns: Range<usize>,
    color: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Head {
    Up,
    Dipped,
}

pub(crate) fn logo() -> impl IntoElement {
    tern(|_, _| Head::Up)
}

pub(crate) fn nodding_logo() -> impl IntoElement {
    tern(|window, cx| {
        if cx.reduce_motion() {
            return Head::Up;
        }
        window.request_animation_frame();
        head_at(nod_clock().elapsed())
    })
}

pub(crate) trait Nodding {
    fn nodding(self, busy: bool) -> Self;
}

impl Nodding for Button {
    fn nodding(self, busy: bool) -> Self {
        self.loading(busy).when(busy, |button| button.icon_element(nodding_logo()))
    }
}

fn tern(head: impl FnOnce(&mut Window, &mut App) -> Head + 'static) -> impl IntoElement {
    div().flex_none().size(px(pixels::SIDE as f32)).child(
        canvas(|_, _, _| {}, move |bounds, (), window, cx| {
            let head = head(window, cx);
            paint(bounds, head, window)
        })
        .size_full(),
    )
}

fn nod_clock() -> Instant {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    *STARTED.get_or_init(Instant::now)
}

fn head_at(elapsed: Duration) -> Head {
    match elapsed.as_millis() % NOD.as_millis() >= DIPPED_FROM.as_millis() {
        true => Head::Dipped,
        false => Head::Up,
    }
}

fn paint(bounds: Bounds<Pixels>, head: Head, window: &mut Window) {
    let scale = window.scale_factor();
    let cell = bounds.size.width / pixels::SIDE as f32;
    let edge =
        |start: Pixels, cells: usize| px((f32::from(start + cell * cells as f32) * scale).round() / scale);
    for run in runs(head) {
        let top_left = point(edge(bounds.left(), run.columns.start), edge(bounds.top(), run.row));
        let bottom_right = point(edge(bounds.left(), run.columns.end), edge(bounds.top(), run.row + 1));
        window.paint_quad(fill(Bounds::from_corners(top_left, bottom_right), rgb(run.color)));
    }
}

fn runs(head: Head) -> impl Iterator<Item = Run> {
    let lowered = usize::from(head == Head::Dipped);
    pixels::ROWS.iter().enumerate().flat_map(move |(row, line)| {
        let row = match row < HEAD_ROWS {
            true => row + lowered,
            false => row,
        };
        line.as_bytes()
            .chunk_by(|left, right| left == right)
            .scan(0, move |column, same| {
                let start = *column;
                *column += same.len();
                Some((row, start..*column, same[0]))
            })
            .filter_map(|(row, columns, pixel)| Some(Run { row, columns, color: pixels::rgb(pixel)? }))
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{HEAD_ROWS, Head, Run, head_at, pixels, runs};

    fn on_row(head: Head, row: usize) -> Vec<Run> {
        runs(head).filter(|run| run.row == row).collect()
    }

    #[test]
    fn every_row_is_as_wide_as_the_grid_and_uses_known_pixels() {
        for line in pixels::ROWS {
            assert_eq!(line.len(), pixels::SIDE);
            assert!(line.bytes().all(|pixel| pixel == b'.' || pixels::rgb(pixel).is_some()));
        }
    }

    #[test]
    fn nothing_is_painted_behind_the_bird() {
        assert_eq!(runs(Head::Up).filter(|run| run.row < 2 || run.row == 15).count(), 0);
    }

    #[test]
    fn a_dipped_head_moves_down_one_row_and_leaves_the_body_still() {
        assert!(on_row(Head::Dipped, 2).is_empty());
        assert_eq!(on_row(Head::Dipped, 3), on_row(Head::Up, 2).into_iter().map(|run| Run { row: 3, ..run }).collect::<Vec<_>>());
        for row in HEAD_ROWS + 1..pixels::SIDE {
            assert_eq!(on_row(Head::Dipped, row), on_row(Head::Up, row));
        }
    }

    #[test]
    fn the_head_dips_for_the_last_third_of_every_nod() {
        let at = |millis| head_at(Duration::from_millis(millis));
        assert_eq!([at(0), at(259), at(260), at(399), at(400), at(660)], [
            Head::Up,
            Head::Up,
            Head::Dipped,
            Head::Dipped,
            Head::Up,
            Head::Dipped,
        ]);
    }
}
