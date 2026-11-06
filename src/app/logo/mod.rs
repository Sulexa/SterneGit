mod pixels;

use std::ops::Range;

use gpui_kit::{
    Bounds, IntoElement, ParentElement as _, Pixels, Styled as _, Window, canvas, div, fill,
    point, px, rgb,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Run {
    row: usize,
    columns: Range<usize>,
    color: u32,
}

pub(crate) fn logo() -> impl IntoElement {
    div().flex_none().size(px(pixels::SIDE as f32)).child(
        canvas(|_, _, _| {}, |bounds, (), window, _| paint(bounds, window)).size_full(),
    )
}

fn paint(bounds: Bounds<Pixels>, window: &mut Window) {
    let scale = window.scale_factor();
    let cell = bounds.size.width / pixels::SIDE as f32;
    let edge =
        |start: Pixels, cells: usize| px((f32::from(start + cell * cells as f32) * scale).round() / scale);
    for run in runs() {
        let top_left = point(edge(bounds.left(), run.columns.start), edge(bounds.top(), run.row));
        let bottom_right = point(edge(bounds.left(), run.columns.end), edge(bounds.top(), run.row + 1));
        window.paint_quad(fill(Bounds::from_corners(top_left, bottom_right), rgb(run.color)));
    }
}

fn runs() -> impl Iterator<Item = Run> {
    pixels::ROWS.iter().enumerate().flat_map(|(row, line)| {
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
    use super::{Run, pixels, runs};

    #[test]
    fn every_row_is_as_wide_as_the_grid_and_uses_known_pixels() {
        for line in pixels::ROWS {
            assert_eq!(line.len(), pixels::SIDE);
            assert!(line.bytes().all(|pixel| pixel == b'.' || pixels::rgb(pixel).is_some()));
        }
    }

    #[test]
    fn the_bill_row_is_cut_into_sky_cap_orange_bill_and_black_tip() {
        let bill_row = runs().filter(|run| run.row == 6).collect::<Vec<_>>();
        let run = |columns, color| Run { row: 6, columns, color };
        assert_eq!(
            bill_row,
            [
                run(0..2, 0x8ec3e6),
                run(2..11, 0x111111),
                run(11..14, 0xf26b1d),
                run(14..15, 0x111111),
                run(15..16, 0x8ec3e6),
            ]
        );
    }

    #[test]
    fn transparent_corners_are_left_unpainted() {
        let first_row = runs().filter(|run| run.row == 0).collect::<Vec<_>>();
        assert_eq!(first_row, [Run { row: 0, columns: 3..13, color: 0x8ec3e6 }]);
    }
}
