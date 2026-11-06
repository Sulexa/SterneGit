#[path = "../src/app/logo/pixels.rs"]
mod pixels;

use std::{fs, path::Path};

type Grid = Vec<Vec<Option<u32>>>;

#[test]
fn the_mockup_logo_shows_the_same_pixels_as_the_app() {
    let svg = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs").join("mockup").join("logo.svg"),
    )
    .expect("docs/mockup/logo.svg exists");
    assert_eq!(painted(&svg), drawn_by_the_app());
}

fn drawn_by_the_app() -> Grid {
    pixels::ROWS.iter().map(|row| row.bytes().map(pixels::rgb).collect()).collect()
}

fn painted(svg: &str) -> Grid {
    let mut grid = vec![vec![None; pixels::SIDE]; pixels::SIDE];
    for path in svg.split("<path ").skip(1) {
        let color = u32::from_str_radix(between(path, "fill=\"#"), 16).expect("fills are #rrggbb");
        for square in between(path, "d=\"").split('M').filter(|square| !square.is_empty()) {
            let numbers = square
                .split(|character: char| !character.is_ascii_digit())
                .filter(|number| !number.is_empty())
                .map(|number| number.parse::<usize>().expect("digits parse"))
                .collect::<Vec<_>>();
            let [x, y, width, ..] = numbers[..] else {
                panic!("each square is drawn as Mx yhwv1h-wz, not {square}");
            };
            grid[y][x..x + width].fill(Some(color));
        }
    }
    grid
}

fn between<'a>(text: &'a str, opening: &str) -> &'a str {
    let start = text.find(opening).expect("the attribute is there") + opening.len();
    let length = text[start..].find('"').expect("the attribute is closed");
    &text[start..start + length]
}
