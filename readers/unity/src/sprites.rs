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

pub(crate) struct Frame {
    pub(crate) rect: Option<[f64; 4]>,
    pub(crate) pivot: (f64, f64),
    pub(crate) pixels_per_unit: f64,
    pub(crate) border: [f64; 4],
}

pub(crate) fn frame(meta: &Node, file_id: i64) -> Frame {
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
    match entry {
        Some(entry) => {
            let rect = entry.get("rect");
            let read = |key: &str| number(rect, key).unwrap_or(0.).max(0.).floor();
            Frame {
                rect: Some([read("x"), read("y"), read("width"), read("height")]),
                pivot: pivot_of(entry),
                pixels_per_unit,
                border,
            }
        }
        None => Frame {
            rect: None,
            pivot: importer.map_or((0.5, 0.5), pivot_of),
            pixels_per_unit,
            border,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::frame;
    use crate::yaml::plain;

    #[test]
    fn a_single_sprite_is_the_whole_picture_at_its_pixels_per_unit() {
        let meta = plain("guid: 1
TextureImporter:
  spritePixelsToUnits: 16
  alignment: 7
");
        let found = frame(&meta, 21300000);
        assert_eq!(found.rect, None);
        assert_eq!(found.pivot, (0.5, 0.));
        assert_eq!(found.pixels_per_unit, 16.);
    }

    #[test]
    fn a_sprite_from_a_sheet_is_its_cell() {
        let meta = plain(
            "TextureImporter:
  spriteSheet:
    sprites:
    - name: top_right
      rect:
        x: 2
        y: 1
        width: 2
        height: 1
      alignment: 0
      internalID: 555
",
        );
        assert_eq!(frame(&meta, 555).rect, Some([2., 1., 2., 1.]));
    }
}
