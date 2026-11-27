use super::scene::Key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum View {
    Front,
    Above,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlacedSprite {
    pub(crate) picture: String,
    pub(crate) at: (f64, f64),
    pub(crate) scale: (f64, f64),
    pub(crate) flip: (bool, bool),
    pub(crate) order: i64,
    pub(crate) alpha: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Shape {
    Circle { center: (f64, f64), radius: f64 },
    Box { center: (f64, f64), size: (f64, f64) },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marker {
    Plain,
    Camera,
    Light,
    Canvas,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Paint {
    Fill,
    Sprite { picture: String, sliced: bool, keep_aspect: bool, unit: f64 },
    Texture { picture: String },
    Text { text: String, size: f64, align: [f64; 2] },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Graphic {
    pub(crate) area: [f64; 4],
    pub(crate) color: [f64; 4],
    pub(crate) paint: Paint,
}

impl Paint {
    pub(crate) fn picture(&self) -> Option<&str> {
        match self {
            Paint::Sprite { picture, .. } | Paint::Texture { picture } => Some(picture),
            Paint::Fill | Paint::Text { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spot {
    pub(crate) key: Key,
    pub(crate) name: String,
    pub(crate) at: (f64, f64),
    pub(crate) sprites: Vec<PlacedSprite>,
    pub(crate) shapes: Vec<(String, Shape)>,
    pub(crate) camera: Option<f64>,
    pub(crate) marker: Marker,
    pub(crate) rect: Option<[f64; 4]>,
    pub(crate) graphics: Vec<Graphic>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout {
    pub(crate) spots: Vec<Spot>,
}

impl Layout {
    pub(crate) fn spot(&self, key: Key) -> Option<&Spot> {
        self.spots.iter().find(|spot| spot.key == key)
    }
}
