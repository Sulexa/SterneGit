use std::collections::HashMap;

use super::{
    names::{file_name, file_stem, nicify},
    yaml::Node,
};

const AXES: [&str; 4] = ["x", "y", "z", "w"];
const CHANNELS: [&str; 4] = ["r", "g", "b", "a"];
const SAME_NUMBER: f64 = 1e-5;
const BUILT_IN_EXTRA: &str = "0000000000000000e000000000000000";
const BUILT_IN_DEFAULT: &str = "0000000000000000f000000000000000";
const FLAG_PREFIXES: [&str; 25] = [
    "is", "has", "use", "enable", "enabled", "can", "should", "show", "allow", "auto", "loop", "play", "mute",
    "flip", "invert", "lock", "keep", "ignore", "apply", "cast", "receive", "bypass", "preload", "force", "generate",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glyph {
    Object,
    Asset,
    Script,
    Image,
    Sound,
    Material,
    Prefab,
    Clip,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reference {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) glyph: Glyph,
    pub(crate) guid: Option<String>,
    pub(crate) missing: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Text(String),
    Number(String),
    Flag(bool),
    Vector(Vec<(&'static str, String)>),
    Color { rgba: [f32; 4], hdr: Option<f32> },
    Reference(Reference),
    Nothing(String),
    Count(usize),
}

impl Value {
    pub(crate) fn same_as(&self, other: &Value) -> Sameness {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => numbers(a, b),
            (Value::Vector(a), Value::Vector(b)) if a.len() == b.len() => a
                .iter()
                .zip(b)
                .map(|((axis, x), (other_axis, y))| if axis == other_axis { numbers(x, y) } else { Sameness::Different })
                .fold(Sameness::Equal, Sameness::and),
            (Value::Color { rgba: a, .. }, Value::Color { rgba: b, .. }) => a
                .iter()
                .zip(b)
                .map(|(x, y)| float_sameness(f64::from(*x), f64::from(*y)))
                .fold(Sameness::Equal, Sameness::and),
            (a, b) if a == b => Sameness::Equal,
            _ => Sameness::Different,
        }
    }

    pub(crate) fn text(&self) -> String {
        match self {
            Value::Text(text) | Value::Number(text) => text.clone(),
            Value::Flag(on) => if *on { "On" } else { "Off" }.to_owned(),
            Value::Vector(axes) => axes
                .iter()
                .map(|(axis, value)| format!("{} {value}", axis.to_uppercase()))
                .collect::<Vec<_>>()
                .join("  "),
            Value::Color { rgba, hdr } => {
                let hex = hex_color(*rgba);
                match hdr {
                    Some(intensity) => format!("{hex} · Intensity {}", short_number(f64::from(*intensity))),
                    None => hex,
                }
            }
            Value::Reference(reference) => format!("{} ({})", reference.name, reference.kind),
            Value::Nothing(kind) => format!("None ({kind})"),
            Value::Count(count) => format!("{count} {}", if *count == 1 { "item" } else { "items" }),
        }
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sameness {
    Equal,
    Rounding,
    Different,
}

impl Sameness {
    fn and(self, other: Sameness) -> Sameness {
        match (self, other) {
            (Sameness::Different, _) | (_, Sameness::Different) => Sameness::Different,
            (Sameness::Rounding, _) | (_, Sameness::Rounding) => Sameness::Rounding,
            _ => Sameness::Equal,
        }
    }
}

fn numbers(a: &str, b: &str) -> Sameness {
    if a == b {
        return Sameness::Equal;
    }
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => float_sameness(x, y),
        _ => Sameness::Different,
    }
}

fn float_sameness(x: f64, y: f64) -> Sameness {
    if x == y {
        Sameness::Equal
    } else if (x - y).abs() <= SAME_NUMBER * x.abs().max(y.abs()).max(1.) {
        Sameness::Rounding
    } else {
        Sameness::Different
    }
}

pub(crate) fn short_number(value: f64) -> String {
    let rounded = (value * 1000.).round() / 1000.;
    let text = format!("{rounded}");
    if text == "-0" { "0".to_owned() } else { text }
}

pub(crate) fn hex_color(rgba: [f32; 4]) -> String {
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u8;
    let [r, g, b, a] = rgba.map(byte);
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Lookup {
    pub(crate) objects: HashMap<i64, (String, String)>,
    pub(crate) assets: HashMap<String, String>,
    pub(crate) targets: HashMap<(String, i64), Target>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) type_name: String,
    pub(crate) object: String,
    pub(crate) script: Option<String>,
}

impl Lookup {
    pub(crate) fn asset_path(&self, guid: &str) -> Option<&str> {
        self.assets.get(guid).map(String::as_str)
    }

    pub(crate) fn script_name(&self, guid: &str) -> Option<String> {
        self.asset_path(guid)
            .map(|path| file_stem(path).to_owned())
            .or_else(|| known_script(guid).map(str::to_owned))
    }

    pub(crate) fn reference(&self, node: &Node) -> Option<Value> {
        let file_id = node.get("fileID")?.number()? as i64;
        let guid = node.get("guid").and_then(Node::text).filter(|guid| !guid.is_empty());
        if file_id == 0 && guid.is_none() {
            return Some(Value::Nothing("Object".to_owned()));
        }
        let Some(guid) = guid else {
            return Some(Value::Reference(match self.objects.get(&file_id) {
                Some((name, kind)) => Reference {
                    name: name.clone(),
                    kind: kind.clone(),
                    glyph: Glyph::Object,
                    guid: None,
                    missing: false,
                },
                None => Reference {
                    name: "Missing".to_owned(),
                    kind: format!("fileID {file_id}"),
                    glyph: Glyph::Object,
                    guid: None,
                    missing: true,
                },
            }));
        };
        if guid == BUILT_IN_EXTRA || guid == BUILT_IN_DEFAULT {
            return Some(Value::Reference(Reference {
                name: built_in_name(file_id),
                kind: "Built-in".to_owned(),
                glyph: Glyph::Asset,
                guid: Some(guid.to_owned()),
                missing: false,
            }));
        }
        let (kind, glyph) = asset_kind(file_id, self.asset_path(guid));
        let reference = match self.asset_path(guid) {
            Some(path) => Reference {
                name: if kind == "Script" { file_stem(path).to_owned() } else { file_stem(file_name(path)).to_owned() },
                kind: kind.to_owned(),
                glyph,
                guid: Some(guid.to_owned()),
                missing: false,
            },
            None => match known_script(guid) {
                Some(name) => Reference {
                    name: name.to_owned(),
                    kind: "Script".to_owned(),
                    glyph: Glyph::Script,
                    guid: Some(guid.to_owned()),
                    missing: false,
                },
                None => Reference {
                    name: format!("Missing {}", &guid[..guid.len().min(8)]),
                    kind: kind.to_owned(),
                    glyph,
                    guid: Some(guid.to_owned()),
                    missing: true,
                },
            },
        };
        Some(Value::Reference(reference))
    }
}

fn built_in_name(file_id: i64) -> String {
    match file_id {
        10303 | 10302 => "Default-Material".to_owned(),
        10754 => "Sprites-Default".to_owned(),
        10202 => "Cube".to_owned(),
        10207 => "Sphere".to_owned(),
        10208 => "Capsule".to_owned(),
        10209 => "Plane".to_owned(),
        10210 => "Quad".to_owned(),
        10102 => "Arial".to_owned(),
        _ => format!("Built-in {file_id}"),
    }
}

fn asset_kind(file_id: i64, path: Option<&str>) -> (&'static str, Glyph) {
    match file_id {
        21300000 => ("Sprite", Glyph::Image),
        2800000 => ("Texture", Glyph::Image),
        2100000 => ("Material", Glyph::Material),
        11500000 => ("Script", Glyph::Script),
        7400000 => ("Animation Clip", Glyph::Clip),
        9100000 => ("Animator Controller", Glyph::Clip),
        8300000 => ("Audio Clip", Glyph::Sound),
        100100000 => ("Prefab", Glyph::Prefab),
        4300000 => ("Mesh", Glyph::Asset),
        4800000 => ("Shader", Glyph::Asset),
        11400000 => ("Asset", Glyph::Asset),
        _ => match path.map(|path| path.rsplit_once('.').map_or("", |(_, extension)| extension)) {
            Some("png" | "jpg" | "jpeg" | "psd" | "tga" | "tif" | "tiff" | "exr" | "bmp" | "gif") => {
                ("Sprite", Glyph::Image)
            }
            Some("wav" | "ogg" | "mp3" | "aif" | "aiff" | "flac") => ("Audio Clip", Glyph::Sound),
            Some("fbx" | "obj" | "blend") => ("Model", Glyph::Prefab),
            Some("prefab") => ("Prefab", Glyph::Prefab),
            Some("mat") => ("Material", Glyph::Material),
            Some("anim") => ("Animation Clip", Glyph::Clip),
            _ => ("Asset", Glyph::Asset),
        },
    }
}

fn known_script(guid: &str) -> Option<&'static str> {
    Some(match guid {
        "fe87c0e1cc204ed48ad3b37840f39efc" => "Image",
        "5f7201a12d95ffc409449d95f23cf332" => "Text",
        "4e29b1a8efbd4b44bb3f3716e73f07ff" => "Button",
        "0cd44c1031e13a943bb63640046fad76" => "Canvas Scaler",
        "dc42784cf147c0c48a680349fa168899" => "Graphic Raycaster",
        "76c392e42b5098c458856cdf6ecaaaa1" => "Event System",
        "4f231c4fb786f3946a6b90b886c48677" => "Standalone Input Module",
        "f4688fdb7df04437aeb418b961361dc5" => "TextMeshPro UGUI",
        "9541d86e2fd84c1d9990edf0852d74ab" => "TextMeshPro",
        "073797afb82c5a1438f328866b10b3f0" => "Light 2D",
        "a79441f348de89743a2939f4d699eac1" => "Universal Additional Camera Data",
        "172515602e62fb746b5d573b38a5fe58" => "Volume",
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Field {
    pub(crate) path: String,
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) value: Value,
}

pub(crate) fn fields(body: &Node, skipped: &[&str], lookup: &Lookup) -> Vec<Field> {
    let mut out = Vec::new();
    for (key, node) in body.entries() {
        if skipped.contains(&key.as_str()) {
            continue;
        }
        flatten(key, &nicify(key), key, node, 0, lookup, &mut out);
    }
    out
}

pub(crate) fn single(key: &str, node: &Node, lookup: &Lookup) -> Value {
    leaf(key, node, lookup).unwrap_or_else(|| Value::Text(String::new()))
}

fn flatten(key: &str, label: &str, path: &str, node: &Node, depth: usize, lookup: &Lookup, out: &mut Vec<Field>) {
    if let Some(value) = leaf(key, node, lookup) {
        out.push(Field {
            path: path.to_owned(),
            label: label.to_owned(),
            depth,
            value,
        });
        return;
    }
    match node {
        Node::List(items) => {
            out.push(Field {
                path: path.to_owned(),
                label: label.to_owned(),
                depth,
                value: Value::Count(items.len()),
            });
            let mut seen = std::collections::HashSet::new();
            for (ix, item) in items.iter().enumerate() {
                let named = leaf(key, item, lookup).is_none().then(|| single_entry(item)).flatten();
                match named {
                    Some((name, inner)) if seen.insert(name) => {
                        flatten(name, &nicify(name), &format!("{path}.{name}"), inner, depth + 1, lookup, out)
                    }
                    Some((name, inner)) => {
                        flatten(name, &nicify(name), &format!("{path}[{ix}].{name}"), inner, depth + 1, lookup, out)
                    }
                    None => flatten(key, &format!("Element {ix}"), &format!("{path}[{ix}]"), item, depth + 1, lookup, out),
                }
            }
        }
        Node::Map(entries) => {
            out.push(Field {
                path: path.to_owned(),
                label: label.to_owned(),
                depth,
                value: Value::Text(String::new()),
            });
            for (name, inner) in entries {
                flatten(name, &nicify(name), &format!("{path}.{name}"), inner, depth + 1, lookup, out);
            }
        }
        Node::Scalar(_) => {}
    }
}

fn single_entry(item: &Node) -> Option<(&str, &Node)> {
    match item.entries() {
        [(name, inner)] => Some((name.as_str(), inner)),
        _ => None,
    }
}

fn leaf(key: &str, node: &Node, lookup: &Lookup) -> Option<Value> {
    match node {
        Node::Scalar(text) => Some(scalar(key, text)),
        Node::Map(_) if node.get("fileID").is_some() => lookup.reference(node),
        Node::Map(_) if is_vector(node) => Some(vector(node)),
        Node::Map(_) if is_color(node) => Some(color(node)),
        Node::List(items) if items.is_empty() => Some(Value::Count(0)),
        _ => None,
    }
}

fn scalar(key: &str, text: &str) -> Value {
    let looks_numeric = text.parse::<f64>().is_ok();
    if (text == "0" || text == "1") && is_flag_name(key) {
        Value::Flag(text == "1")
    } else if looks_numeric {
        Value::Number(text.to_owned())
    } else {
        Value::Text(text.to_owned())
    }
}

pub(crate) fn is_flag_name(key: &str) -> bool {
    let bare = key.trim_start_matches("m_").trim_start_matches('_');
    let lower = bare.to_ascii_lowercase();
    if matches!(lower.as_str(), "enabled" | "isactive" | "istrigger" | "simulated" | "flipx" | "flipy" | "mute" | "loop") {
        return true;
    }
    FLAG_PREFIXES.iter().any(|prefix| {
        lower.starts_with(prefix)
            && bare[prefix.len()..]
                .chars()
                .next()
                .is_none_or(|next| next.is_uppercase() || next.is_ascii_digit())
    })
}

fn keys_within(node: &Node, allowed: &[&str]) -> bool {
    let entries = node.entries();
    !entries.is_empty()
        && entries.len() <= allowed.len()
        && entries
            .iter()
            .all(|(key, value)| allowed.contains(&key.as_str()) && value.number().is_some())
}

fn is_vector(node: &Node) -> bool {
    keys_within(node, &AXES) && node.get("x").is_some() && node.get("y").is_some()
}

fn is_color(node: &Node) -> bool {
    keys_within(node, &CHANNELS) && node.entries().len() >= 3
}

fn vector(node: &Node) -> Value {
    Value::Vector(
        AXES.iter()
            .filter_map(|axis| Some((*axis, node.get(axis)?.text()?.to_owned())))
            .collect(),
    )
}

fn color(node: &Node) -> Value {
    let channel = |name: &str, default: f32| node.get(name).and_then(Node::number).map_or(default, |value| value as f32);
    let rgba = [channel("r", 0.), channel("g", 0.), channel("b", 0.), channel("a", 1.)];
    let brightest = rgba[0].max(rgba[1]).max(rgba[2]);
    if brightest > 1.0001 {
        let scaled = [rgba[0] / brightest, rgba[1] / brightest, rgba[2] / brightest, rgba[3]];
        Value::Color {
            rgba: scaled,
            hdr: Some(brightest.log2()),
        }
    } else {
        Value::Color { rgba, hdr: None }
    }
}

pub(crate) fn quaternion_degrees(node: &Node) -> Option<[f64; 3]> {
    let part = |name: &str| node.get(name).and_then(Node::number);
    let (x, y, z, w) = (part("x")?, part("y")?, part("z")?, part("w")?);
    Some(euler_degrees(x, y, z, w))
}

pub(crate) fn euler_degrees(x: f64, y: f64, z: f64, w: f64) -> [f64; 3] {
    let sin_pitch = 2. * (w * x - y * z);
    let pitch = if sin_pitch.abs() >= 1. { std::f64::consts::FRAC_PI_2.copysign(sin_pitch) } else { sin_pitch.asin() };
    let yaw = (2. * (w * y + x * z)).atan2(1. - 2. * (x * x + y * y));
    let roll = (2. * (w * z + x * y)).atan2(1. - 2. * (x * x + z * z));
    let degrees = |radians: f64| {
        let value = radians.to_degrees();
        if value.abs() < 1e-4 { 0. } else { value }
    };
    [degrees(pitch), degrees(yaw), degrees(roll)]
}

pub(crate) fn rotation_value(node: &Node) -> Option<Value> {
    let [x, y, z] = quaternion_degrees(node)?;
    Some(Value::Vector(vec![("x", short_number(x)), ("y", short_number(y)), ("z", short_number(z))]))
}

#[cfg(test)]
mod tests {
    use super::{Lookup, Sameness, Value, euler_degrees, fields, hex_color, is_flag_name};
    use crate::unity::yaml::Node;

    fn map(entries: &[(&str, Node)]) -> Node {
        Node::Map(entries.iter().map(|(key, value)| ((*key).to_owned(), value.clone())).collect())
    }

    fn text(value: &str) -> Node {
        Node::Scalar(value.to_owned())
    }

    #[test]
    fn floats_that_only_round_differently_are_not_a_real_change() {
        let one = Value::Number("1".to_owned());
        assert_eq!(one.same_as(&Value::Number("0.99999994".to_owned())), Sameness::Rounding);
        assert_eq!(one.same_as(&Value::Number("1.4".to_owned())), Sameness::Different);
        assert_eq!(one.same_as(&Value::Number("1".to_owned())), Sameness::Equal);
    }

    #[test]
    fn a_map_of_axes_reads_as_a_vector_and_channels_as_a_color() {
        let body = map(&[
            ("m_LocalPosition", map(&[("x", text("0")), ("y", text("0.62")), ("z", text("0"))])),
            ("m_Color", map(&[("r", text("1")), ("g", text("1")), ("b", text("1")), ("a", text("1"))])),
        ]);
        let found = fields(&body, &[], &Lookup::default());
        assert_eq!(found[0].label, "Local Position");
        assert!(matches!(&found[0].value, Value::Vector(axes) if axes.len() == 3));
        assert_eq!(found[1].value.text(), "#FFFFFF");
    }

    #[test]
    fn lists_count_their_items_and_list_each_one_below() {
        let body = map(&[(
            "drops",
            Node::List(vec![
                map(&[("fileID", text("0"))]),
                map(&[("fileID", text("11400000")), ("guid", text("c3a1e9f07b2d4c5e8f6a9b0c1d2e3f4a")), ("type", text("2"))]),
            ]),
        )]);
        let found = fields(&body, &[], &Lookup::default());
        assert_eq!(found[0].value, Value::Count(2));
        assert_eq!(found[2].label, "Element 1");
        assert_eq!(found[2].depth, 1);
    }

    #[test]
    fn a_reference_inside_the_file_reads_as_the_name_of_its_object() {
        let mut lookup = Lookup::default();
        lookup.objects.insert(42, ("Crown".to_owned(), "Transform".to_owned()));
        lookup.assets.insert("aa".repeat(16), "Assets/Art/crown-icon.png".to_owned());
        let inside = lookup.reference(&map(&[("fileID", text("42"))]));
        assert_eq!(inside.map(|value| value.text()), Some("Crown (Transform)".to_owned()));
        let sprite = lookup.reference(&map(&[("fileID", text("21300000")), ("guid", text(&"aa".repeat(16))), ("type", text("3"))]));
        assert_eq!(sprite.map(|value| value.text()), Some("crown-icon (Sprite)".to_owned()));
        let empty = lookup.reference(&map(&[("fileID", text("0"))]));
        assert_eq!(empty, Some(Value::Nothing("Object".to_owned())));
    }

    #[test]
    fn zero_or_one_is_a_check_box_only_for_names_that_read_as_yes_or_no() {
        assert!(is_flag_name("m_IsTrigger"));
        assert!(is_flag_name("m_Enabled"));
        assert!(is_flag_name("useGravity"));
        assert!(!is_flag_name("m_Layer"));
        assert!(!is_flag_name("island"));
    }

    #[test]
    fn a_quarter_turn_around_z_is_ninety_degrees() {
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let [x, y, z] = euler_degrees(0., 0., half, half);
        assert!(x.abs() < 1e-6 && y.abs() < 1e-6 && (z - 90.).abs() < 1e-6);
        assert_eq!(hex_color([0.29, 0.87, 0.5, 1.]), "#4ADE80");
    }
}
