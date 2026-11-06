use gpui_kit::{
    Bounds, Hsla, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, canvas, component::tooltip::Tooltip, div,
    fill, hsla, point, px,
};

const GRID: usize = 5;
const PAINTED_COLUMNS: usize = GRID.div_ceil(2);
const PATTERN_SHIFT: u32 = 9;
const PATTERN_BITS: u64 = (1 << (GRID * PAINTED_COLUMNS)) - 1;
const MARGIN_CELLS: f32 = 0.5;
const SPAN_CELLS: f32 = GRID as f32 + 2. * MARGIN_CELLS;
const CORNER_SHARE: f32 = 0.2;
const HUES: u64 = 360;
const INK: (f32, f32) = (0.6, 0.62);
const GROUND: (f32, f32) = (0.3, 0.22);

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;
const MURMUR_MIX_FIRST: u64 = 0xff51_afd7_ed55_8ccd;
const MURMUR_MIX_SECOND: u64 = 0xc4ce_b9fe_1a85_ec53;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identicon(u64);

impl Identicon {
    fn of(email: &str) -> Self {
        let normalized = email.trim().chars().flat_map(char::to_lowercase);
        Self(murmur_finish(fnv1a(normalized)))
    }

    fn hue_degrees(self) -> u64 {
        self.0 % HUES
    }

    fn pattern(self) -> u64 {
        (self.0 >> PATTERN_SHIFT) & PATTERN_BITS
    }

    fn color(self, (saturation, lightness): (f32, f32)) -> Hsla {
        hsla(self.hue_degrees() as f32 / HUES as f32, saturation, lightness, 1.)
    }

    fn is_on(self, column: usize, row: usize) -> bool {
        let painted = column.min(GRID - 1 - column);
        (self.pattern() >> (row * PAINTED_COLUMNS + painted)) & 1 == 1
    }

    fn cells(self) -> impl Iterator<Item = (usize, usize)> {
        (0..GRID)
            .flat_map(|row| (0..GRID).map(move |column| (column, row)))
            .filter(move |&(column, row)| self.is_on(column, row))
    }
}

fn fnv1a(text: impl Iterator<Item = char>) -> u64 {
    let mut utf8 = [0; 4];
    text.fold(FNV_OFFSET, |hash, character| {
        character
            .encode_utf8(&mut utf8)
            .bytes()
            .fold(hash, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME))
    })
}

fn murmur_finish(mut hash: u64) -> u64 {
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(MURMUR_MIX_FIRST);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(MURMUR_MIX_SECOND);
    hash ^ (hash >> 33)
}

pub(crate) fn identicon(seed: &str, size: Pixels) -> impl IntoElement {
    let identicon = Identicon::of(seed);
    div().flex_none().size(size).child(
        canvas(
            |_, _, _| {},
            move |bounds, (), window, _| paint(identicon, bounds, window),
        )
        .size_full(),
    )
}

pub(crate) fn author_avatar(
    name: &SharedString,
    email: &SharedString,
    size: Pixels,
) -> impl IntoElement {
    let identicon = Identicon::of(email);
    let (name, email) = (name.clone(), email.clone());
    div()
        .id("author-avatar")
        .flex_none()
        .size(size)
        .tooltip(move |window, cx| Tooltip::new(format!("{name} <{email}>")).build(window, cx))
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, _| paint(identicon, bounds, window),
            )
            .size_full(),
        )
}

fn paint(identicon: Identicon, bounds: Bounds<Pixels>, window: &mut Window) {
    let scale = window.scale_factor();
    let cell = bounds.size.width / SPAN_CELLS;
    let edge = |start: Pixels, cells: f32| {
        px((f32::from(start + cell * (MARGIN_CELLS + cells)) * scale).round() / scale)
    };
    window.paint_quad(
        fill(bounds, identicon.color(GROUND)).corner_radii(bounds.size.width * CORNER_SHARE),
    );
    let ink = identicon.color(INK);
    for (column, row) in identicon.cells() {
        let (column, row) = (column as f32, row as f32);
        let top_left = point(edge(bounds.left(), column), edge(bounds.top(), row));
        let bottom_right = point(edge(bounds.left(), column + 1.), edge(bounds.top(), row + 1.));
        window.paint_quad(fill(Bounds::from_corners(top_left, bottom_right), ink));
    }
}

#[cfg(test)]
mod tests {
    use super::{GRID, Identicon};

    #[test]
    fn the_same_email_in_another_case_or_with_spaces_gives_the_same_picture() {
        assert_eq!(
            Identicon::of(" Robin@FastGit.dev "),
            Identicon::of("robin@fastgit.dev")
        );
    }

    #[test]
    fn colors_and_patterns_match_the_mockup() {
        let drawn = |email| {
            let identicon = Identicon::of(email);
            (identicon.hue_degrees(), identicon.pattern())
        };
        assert_eq!(drawn("alex@fastgit.dev"), (305, 0b111110110000100));
        assert_eq!(drawn("sam@fastgit.dev"), (53, 0b101111000010011));
        assert_eq!(drawn("robin@fastgit.dev"), (103, 0b110100100100111));
    }

    #[test]
    fn the_right_columns_mirror_the_left_ones() {
        let identicon = Identicon::of("sulexa95@gmail.com");
        for row in 0..GRID {
            for column in 0..GRID {
                assert_eq!(
                    identicon.is_on(column, row),
                    identicon.is_on(GRID - 1 - column, row)
                );
            }
        }
    }

    #[test]
    fn a_pattern_bit_lights_a_left_cell_and_its_mirror() {
        let identicon = Identicon::of("sam@fastgit.dev");
        let top_row = identicon.cells().filter(|&(_, row)| row == 0).collect::<Vec<_>>();
        assert_eq!(top_row, [(0, 0), (1, 0), (3, 0), (4, 0)]);
    }
}
