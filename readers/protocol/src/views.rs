use serde::{Deserialize, Serialize};

use crate::{Field, Value};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Curves {
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub events: Vec<(f64, String)>,
    pub length: f64,
    pub sample_rate: f64,
    #[serde(default)]
    pub settings: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub group: String,
    pub name: String,
    pub keys: Keys,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Keys {
    Curve(Vec<CurveKey>),
    Objects(Vec<ObjectKey>),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveKey {
    pub time: f64,
    pub value: f64,
    pub in_slope: f64,
    pub out_slope: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectKey {
    pub time: f64,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphKind {
    Script,
    Controller,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub kind: GraphKind,
    pub ports: bool,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub parameters: Vec<Declared>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Declared {
    pub name: String,
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub members: Vec<Field>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Entry,
    AnyState,
    Exit,
    DefaultState,
    State,
    Machine,
    Event,
    Flow,
    Value,
    Variable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortType {
    Flow,
    Float,
    Integer,
    Bool,
    Vector,
    Text,
    Object,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Port {
    pub key: String,
    pub label: String,
    pub kind: PortType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub literal: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub key: String,
    pub id: String,
    pub title: String,
    pub kind: NodeKind,
    pub at: (f64, f64),
    pub size: (f64, f64),
    #[serde(default)]
    pub inputs: Vec<Port>,
    #[serde(default)]
    pub outputs: Vec<Port>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_port: Option<String>,
    pub kind: PortType,
    #[serde(default)]
    pub fields: Vec<Field>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    Front,
    Above,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub facing: Facing,
    #[serde(default)]
    pub spots: Vec<Spot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spot {
    pub object: String,
    pub name: String,
    pub at: (f64, f64),
    #[serde(default)]
    pub sprites: Vec<PlacedSprite>,
    #[serde(default)]
    pub shapes: Vec<(String, Shape)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<f64>,
    pub marker: Marker,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f64; 4]>,
    #[serde(default)]
    pub graphics: Vec<Graphic>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedSprite {
    pub picture: String,
    pub at: (f64, f64),
    pub scale: (f64, f64),
    pub flip: (bool, bool),
    pub order: i64,
    pub alpha: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Circle { center: (f64, f64), radius: f64 },
    Box { center: (f64, f64), size: (f64, f64) },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Marker {
    Plain,
    Camera,
    Light,
    Canvas,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graphic {
    pub area: [f64; 4],
    pub color: [f64; 4],
    pub paint: Paint,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paint {
    Fill,
    Sprite { picture: String, sliced: bool, keep_aspect: bool, unit: f64 },
    Texture { picture: String },
    Text { text: String, size: f64, align: [f64; 2] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Picture {
    pub id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f64; 4]>,
    pub pivot: (f64, f64),
    pub pixels_per_unit: f64,
    pub border: [f64; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub line: u32,
    pub text: String,
}
