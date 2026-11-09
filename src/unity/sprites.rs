use super::yaml::Node;

const SINGLE_SPRITE: i64 = 21300000;
const DEFAULT_PIXELS_PER_UNIT: f64 = 100.;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SpriteImage {
    pub(crate) rgba: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pivot: (f64, f64),
    pub(crate) pixels_per_unit: f64,
    pub(crate) border: [f64; 4],
}

impl SpriteImage {
    pub(crate) fn size_in_units(&self) -> (f64, f64) {
        (f64::from(self.width) / self.pixels_per_unit, f64::from(self.height) / self.pixels_per_unit)
    }
}

struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn number(node: Option<&Node>, key: &str) -> Option<f64> {
    node?.get(key)?.number()
}

fn alignment_pivot(alignment: i64, custom: (f64, f64)) -> (f64, f64) {
    match alignment {
        1 => (0., 1.),
        2 => (0.5, 1.),
        3 => (1., 1.),
        4 => (0., 0.5),
        5 => (1., 0.5),
        6 => (0., 0.),
        7 => (0.5, 0.),
        8 => (1., 0.),
        9 => custom,
        _ => (0.5, 0.5),
    }
}

fn border_of(node: Option<&Node>) -> [f64; 4] {
    ["x", "y", "z", "w"].map(|side| number(node, side).unwrap_or(0.).max(0.))
}

fn pivot_of(entry: &Node) -> (f64, f64) {
    let alignment = entry.get("alignment").or_else(|| entry.get("spriteAlignment")).and_then(Node::number).unwrap_or(0.) as i64;
    let pivot = entry.get("pivot").or_else(|| entry.get("spritePivot"));
    let custom = (number(pivot, "x").unwrap_or(0.5), number(pivot, "y").unwrap_or(0.5));
    alignment_pivot(alignment, custom)
}

fn sheet_entry<'a>(importer: &'a Node, file_id: i64) -> Option<&'a Node> {
    let sprites = importer.at(&["spriteSheet", "sprites"]).map(Node::items).unwrap_or_default();
    if let Some(found) = sprites
        .iter()
        .find(|sprite| sprite.get("internalID").and_then(Node::integer) == Some(file_id))
    {
        return Some(found);
    }
    let named = importer
        .get("internalIDToNameTable")
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .find(|entry| {
            entry
                .get("first")
                .and_then(|first| first.entries().first())
                .and_then(|(_, id)| id.integer())
                == Some(file_id)
        })
        .and_then(|entry| entry.get("second").and_then(Node::text))?;
    sprites.iter().find(|sprite| sprite.get("name").and_then(Node::text) == Some(named))
}

pub(crate) fn crop(rgba: &[u8], width: u32, height: u32, meta: &Node, file_id: i64) -> Option<SpriteImage> {
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    let importer = meta.get("TextureImporter");
    let pixels_per_unit = importer
        .and_then(|importer| importer.get("spritePixelsToUnits"))
        .and_then(Node::number)
        .filter(|ppu| *ppu > 0.)
        .unwrap_or(DEFAULT_PIXELS_PER_UNIT);
    let entry = importer.filter(|_| file_id != SINGLE_SPRITE).and_then(|importer| sheet_entry(importer, file_id));
    let border = match entry {
        Some(entry) => border_of(entry.get("border")),
        None => border_of(importer.and_then(|importer| importer.get("spriteBorder"))),
    };
    let (rect, pivot) = match entry {
        Some(entry) => {
            let rect = entry.get("rect");
            let read = |key: &str| number(rect, key).unwrap_or(0.).max(0.) as u32;
            (
                Rect {
                    x: read("x"),
                    y: read("y"),
                    width: read("width"),
                    height: read("height"),
                },
                pivot_of(entry),
            )
        }
        None => (
            Rect {
                x: 0,
                y: 0,
                width,
                height,
            },
            importer.map_or((0.5, 0.5), pivot_of),
        ),
    };
    let right = rect.x.saturating_add(rect.width).min(width);
    let top_from_bottom = rect.y.saturating_add(rect.height).min(height);
    if right <= rect.x || top_from_bottom <= rect.y {
        return None;
    }
    let crop_width = right - rect.x;
    let crop_height = top_from_bottom - rect.y;
    let first_row = height - top_from_bottom;
    let mut pixels = Vec::with_capacity((crop_width * crop_height * 4) as usize);
    for row in first_row..first_row + crop_height {
        let start = ((row * width + rect.x) * 4) as usize;
        pixels.extend_from_slice(&rgba[start..start + (crop_width * 4) as usize]);
    }
    Some(SpriteImage {
        rgba: pixels,
        width: crop_width,
        height: crop_height,
        pivot,
        pixels_per_unit,
        border,
    })
}

#[cfg(test)]
mod tests {
    use super::crop;
    use crate::unity::yaml::plain;

    fn picture(width: u32, height: u32) -> Vec<u8> {
        (0..width * height).flat_map(|ix| [ix as u8, 0, 0, 255]).collect()
    }

    #[test]
    fn a_single_sprite_is_the_whole_picture_at_its_pixels_per_unit() {
        let meta = plain("guid: 1\nTextureImporter:\n  spritePixelsToUnits: 16\n  alignment: 7\n");
        let sprite = crop(&picture(4, 2), 4, 2, &meta, 21300000).expect("whole picture");
        assert_eq!((sprite.width, sprite.height), (4, 2));
        assert_eq!(sprite.pivot, (0.5, 0.));
        assert_eq!(sprite.size_in_units(), (0.25, 0.125));
    }

    #[test]
    fn a_sprite_from_a_sheet_is_cut_counting_rows_from_the_bottom() {
        let meta = plain(
            "TextureImporter:\n  spriteSheet:\n    sprites:\n    - name: top_right\n      rect:\n        x: 2\n        y: 1\n        width: 2\n        height: 1\n      alignment: 0\n      internalID: 555\n",
        );
        let sprite = crop(&picture(4, 2), 4, 2, &meta, 555).expect("one cell");
        assert_eq!((sprite.width, sprite.height), (2, 1));
        assert_eq!(sprite.rgba[0], 2);
    }
}
