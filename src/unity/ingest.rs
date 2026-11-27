use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash as _, Hasher as _},
};

use sternegit_reader_protocol as protocol;

use super::{
    animator::{Drawn, Machine},
    clip::{Clip, CurveKey, Keys, ObjectKey, Track},
    compare::State,
    graph::{Declared, GraphEdge, GraphNode, NodeKind, Port, PortType, ScriptSide},
    layout::{Graphic, Layout, Marker, Paint, PlacedSprite, Shape, Spot, View},
    scene::{Item, Key, Model, Part, Role},
    value::{Digest, Field, Glyph, Reference, Value},
};

const NOISE: &str = "noise";
const AXES: [&str; 8] = ["x", "y", "z", "w", "r", "g", "b", "a"];

pub(crate) fn key_of(id: &str) -> Key {
    id.parse().unwrap_or_else(|_| {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        hasher.finish() as Key
    })
}

pub(crate) fn model(read: &protocol::Model) -> Model {
    let items: Vec<Item> = read.objects.iter().map(item).collect();
    let keys: Vec<Key> = items.iter().map(|item| item.key).collect();
    let mut children: HashMap<Key, Vec<Key>> = HashMap::new();
    let mut roots = Vec::new();
    for item in &items {
        match item.parent.filter(|parent| keys.contains(parent)) {
            Some(parent) => children.entry(parent).or_default().push(item.key),
            None if matches!(item.role, Role::GameObject { .. } | Role::Instance { .. }) => roots.push(item.key),
            None => {}
        }
    }
    Model { items, roots, children }
}

fn item(object: &protocol::Object) -> Item {
    Item {
        key: key_of(&object.id),
        name: object.name.clone(),
        role: match &object.role {
            protocol::Role::Node { root_of } => Role::GameObject {
                root_of: root_of.clone(),
            },
            protocol::Role::Instance { source } => Role::Instance { source: source.clone() },
            protocol::Role::Asset { type_name, detail } => Role::Asset {
                type_name: type_name.clone(),
                detail: detail.clone(),
            },
        },
        parent: object.parent.as_deref().map(key_of),
        parts: object.parts.iter().map(part).collect(),
    }
}

fn part(part: &protocol::Part) -> Part {
    let mut fields = Vec::new();
    flatten(&part.fields, "", 0, &mut fields);
    Part {
        key: part.id.clone(),
        title: part.title.clone(),
        script: part.script.clone(),
        overrides: part.overrides,
        fields,
    }
}

pub(crate) fn fields(tree: &[protocol::Field]) -> Vec<Field> {
    let mut out = Vec::new();
    flatten(tree, "", 0, &mut out);
    out
}

fn flatten(tree: &[protocol::Field], parent: &str, depth: usize, out: &mut Vec<Field>) {
    for field in tree {
        let path = field.path_under(parent);
        out.push(Field {
            path: path.clone(),
            label: field.label.clone(),
            depth,
            value: value(&field.value),
            noise: field.tags.iter().any(|tag| tag == NOISE),
        });
        flatten(&field.children, &path, depth + 1, out);
    }
}

fn value(value: &protocol::Value) -> Value {
    match value {
        protocol::Value::Text(text) => Value::Text(text.clone()),
        protocol::Value::Empty => Value::Text(String::new()),
        protocol::Value::Long { start, more, digest } => Value::Long {
            start: start.clone(),
            more: *more,
            digest: Digest::from_value(*digest),
        },
        protocol::Value::Data { bytes, digest } => Value::Data {
            bytes: *bytes,
            digest: Digest::from_value(*digest),
        },
        protocol::Value::Number(text) => Value::Number(text.clone()),
        protocol::Value::Flag(on) => Value::Flag(*on),
        protocol::Value::Vector(parts) => Value::Vector(
            parts
                .iter()
                .map(|(axis, text)| (AXES.iter().find(|known| *known == axis).copied().unwrap_or("x"), text.clone()))
                .collect(),
        ),
        protocol::Value::Color { rgba, hdr } => Value::Color { rgba: *rgba, hdr: *hdr },
        protocol::Value::Reference(reference) => Value::Reference(Reference {
            name: reference.name.clone(),
            kind: reference.kind.clone(),
            glyph: glyph(reference.icon),
            guid: reference.target.clone(),
            missing: reference.missing,
        }),
        protocol::Value::Nothing(what) => Value::Nothing(what.clone()),
        protocol::Value::Count(count) => Value::Count(*count),
    }
}

fn glyph(icon: protocol::Icon) -> Glyph {
    match icon {
        protocol::Icon::Asset => Glyph::Asset,
        protocol::Icon::Script => Glyph::Script,
        protocol::Icon::Image => Glyph::Image,
        protocol::Icon::Sound => Glyph::Sound,
        protocol::Icon::Material => Glyph::Material,
        protocol::Icon::Prefab => Glyph::Prefab,
        protocol::Icon::Clip => Glyph::Clip,
        protocol::Icon::Object | protocol::Icon::Unknown => Glyph::Object,
    }
}

pub(crate) fn clip(curves: &protocol::Curves) -> Clip {
    Clip {
        tracks: curves
            .tracks
            .iter()
            .map(|track| Track {
                id: track.id.clone(),
                group: track.group.clone(),
                name: track.name.clone(),
                keys: match &track.keys {
                    protocol::Keys::Curve(keys) => Keys::Curve(
                        keys.iter()
                            .map(|key| CurveKey {
                                time: key.time,
                                value: key.value,
                                in_slope: key.in_slope,
                                out_slope: key.out_slope,
                            })
                            .collect(),
                    ),
                    protocol::Keys::Objects(keys) => Keys::Objects(
                        keys.iter()
                            .map(|key| ObjectKey {
                                time: key.time,
                                value: value(&key.value),
                                picture: key.picture.clone(),
                            })
                            .collect(),
                    ),
                },
            })
            .collect(),
        events: curves.events.clone(),
        length: curves.length,
        sample_rate: curves.sample_rate,
        settings: curves.settings.clone(),
    }
}

fn node_kind(kind: protocol::NodeKind) -> NodeKind {
    match kind {
        protocol::NodeKind::Entry => NodeKind::Entry,
        protocol::NodeKind::AnyState => NodeKind::AnyState,
        protocol::NodeKind::Exit => NodeKind::Exit,
        protocol::NodeKind::DefaultState => NodeKind::DefaultState,
        protocol::NodeKind::State => NodeKind::State,
        protocol::NodeKind::Machine => NodeKind::Machine,
        protocol::NodeKind::Event => NodeKind::Event,
        protocol::NodeKind::Flow => NodeKind::Flow,
        protocol::NodeKind::Value => NodeKind::Value,
        protocol::NodeKind::Variable => NodeKind::Variable,
    }
}

fn port_type(kind: protocol::PortType) -> PortType {
    match kind {
        protocol::PortType::Flow => PortType::Flow,
        protocol::PortType::Float => PortType::Float,
        protocol::PortType::Integer => PortType::Integer,
        protocol::PortType::Bool => PortType::Bool,
        protocol::PortType::Vector => PortType::Vector,
        protocol::PortType::Text => PortType::Text,
        protocol::PortType::Object => PortType::Object,
    }
}

fn ports(ports: &[protocol::Port]) -> Vec<Port> {
    ports
        .iter()
        .map(|port| Port {
            key: port.key.clone(),
            label: port.label.clone(),
            kind: port_type(port.kind),
            literal: port.literal.clone(),
            was: None,
        })
        .collect()
}

fn graph_node(node: &protocol::Node) -> GraphNode {
    GraphNode {
        id: node.id.clone(),
        title: node.title.clone(),
        kind: node_kind(node.kind),
        at: node.at,
        size: node.size,
        state: State::Same,
        inputs: ports(&node.inputs),
        outputs: ports(&node.outputs),
        was_at: None,
        variable: node.variable.clone(),
    }
}

fn graph_edge(edge: &protocol::Edge) -> GraphEdge {
    GraphEdge {
        id: edge.id.clone(),
        from: edge.from.clone(),
        to: edge.to.clone(),
        from_port: edge.from_port.clone(),
        to_port: edge.to_port.clone(),
        state: State::Same,
        kind: port_type(edge.kind),
    }
}

fn declared(declared: &protocol::Declared) -> Declared {
    Declared {
        name: declared.name.clone(),
        kind: declared.kind.clone(),
        value: declared.value.clone(),
        members: {
            let mut members = Vec::new();
            flatten(&declared.members, "", 1, &mut members);
            members
        },
    }
}

pub(crate) fn script_side(graph: Option<&protocol::Graph>) -> ScriptSide {
    let Some(graph) = graph else {
        return ScriptSide::default();
    };
    let layer = graph.layers.first();
    ScriptSide {
        units: layer
            .map(|layer| {
                layer
                    .nodes
                    .iter()
                    .map(|node| (node.key.clone(), graph_node(node), fields(&node.fields)))
                    .collect()
            })
            .unwrap_or_default(),
        edges: layer.map(|layer| layer.edges.iter().map(graph_edge).collect()).unwrap_or_default(),
        variables: graph.parameters.iter().map(declared).collect(),
    }
}

pub(crate) fn machine(graph: Option<&protocol::Graph>) -> Machine {
    let Some(graph) = graph else {
        return Machine::default();
    };
    Machine {
        layers: graph
            .layers
            .iter()
            .map(|layer| {
                (
                    layer.name.clone(),
                    Drawn {
                        nodes: layer.nodes.iter().map(|node| (graph_node(node), fields(&node.fields))).collect(),
                        edges: layer.edges.iter().map(|edge| (graph_edge(edge), fields(&edge.fields))).collect(),
                    },
                )
            })
            .collect(),
        parameters: graph.parameters.iter().map(declared).collect(),
    }
}

pub(crate) fn view(layout: &protocol::Layout) -> View {
    match layout.facing {
        protocol::Facing::Front => View::Front,
        protocol::Facing::Above => View::Above,
    }
}

fn shape(shape: &protocol::Shape) -> Shape {
    match shape {
        protocol::Shape::Circle { center, radius } => Shape::Circle {
            center: *center,
            radius: *radius,
        },
        protocol::Shape::Box { center, size } => Shape::Box {
            center: *center,
            size: *size,
        },
    }
}

fn marker(marker: protocol::Marker) -> Marker {
    match marker {
        protocol::Marker::Plain => Marker::Plain,
        protocol::Marker::Camera => Marker::Camera,
        protocol::Marker::Light => Marker::Light,
        protocol::Marker::Canvas => Marker::Canvas,
    }
}

fn paint(paint: &protocol::Paint) -> Paint {
    match paint {
        protocol::Paint::Fill => Paint::Fill,
        protocol::Paint::Sprite {
            picture,
            sliced,
            keep_aspect,
            unit,
        } => Paint::Sprite {
            picture: picture.clone(),
            sliced: *sliced,
            keep_aspect: *keep_aspect,
            unit: *unit,
        },
        protocol::Paint::Texture { picture } => Paint::Texture { picture: picture.clone() },
        protocol::Paint::Text { text, size, align } => Paint::Text {
            text: text.clone(),
            size: *size,
            align: *align,
        },
    }
}

fn spot(spot: &protocol::Spot) -> Spot {
    Spot {
        key: key_of(&spot.object),
        name: spot.name.clone(),
        at: spot.at,
        sprites: spot
            .sprites
            .iter()
            .map(|sprite| PlacedSprite {
                picture: sprite.picture.clone(),
                at: sprite.at,
                scale: sprite.scale,
                flip: sprite.flip,
                order: sprite.order,
                alpha: sprite.alpha,
            })
            .collect(),
        shapes: spot.shapes.iter().map(|(name, found)| (name.clone(), shape(found))).collect(),
        camera: spot.camera,
        marker: marker(spot.marker),
        rect: spot.rect,
        graphics: spot
            .graphics
            .iter()
            .map(|graphic| Graphic {
                area: graphic.area,
                color: graphic.color,
                paint: paint(&graphic.paint),
            })
            .collect(),
    }
}

pub(crate) fn layout(layout: &protocol::Layout) -> Layout {
    Layout {
        spots: layout.spots.iter().map(spot).collect(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        ffi::OsString,
        path::Path,
        sync::{Arc, OnceLock, atomic::AtomicBool},
    };

    use sternegit_reader_protocol::{self as protocol, Found, ReadInput, Search, Side};

    use crate::{
        git::{Revision, scratch::ScratchRepo},
        read::{Reader, Tree},
        unity::{ByReader, Source, read},
    };

    struct Files(Vec<(String, String)>);

    impl Tree for Files {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0.iter().find(|(name, _)| name == path).map(|(_, text)| text.as_bytes().to_vec())
        }

        fn search(&self, search: &Search) -> Vec<Found> {
            let regex = fancy_regex::Regex::new(&format!("(?m){}", search.regex)).unwrap();
            let suffix = search.glob.trim_start_matches("**/").trim_start_matches('*');
            self.0
                .iter()
                .filter(|(name, _)| name.ends_with(suffix))
                .flat_map(|(name, text)| {
                    regex.find_iter(text).filter_map(Result::ok).map(move |found| Found {
                        path: name.clone(),
                        text: found.as_str().to_owned(),
                    })
                })
                .collect()
        }
    }

    pub(crate) fn read_files(path: &str, files: &[(&str, &str)]) -> protocol::Model {
        let tree: Arc<dyn Tree> = Arc::new(Files(
            files.iter().map(|(name, text)| ((*name).to_owned(), (*text).to_owned())).collect(),
        ));
        let input = ReadInput {
            kind: String::new(),
            path: path.to_owned(),
            side: Side::After,
            options: Default::default(),
        };
        bundled_unity().read(tree, input, &AtomicBool::new(false)).unwrap()
    }

    pub(crate) fn read_text(path: &str, text: &str) -> protocol::Model {
        read_files(path, &[(path, text)])
    }

    pub(crate) fn model_of(path: &str, text: &str) -> crate::unity::scene::Model {
        super::model(&read_text(path, text))
    }

    pub(crate) fn bundled_unity() -> Arc<Reader> {
        static UNITY: OnceLock<Arc<Reader>> = OnceLock::new();
        Arc::clone(UNITY.get_or_init(|| {
            let wasm = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("readers/unity/dist/unity.wasm")).unwrap();
            Arc::new(Reader::compile(&wasm).unwrap())
        }))
    }

    const PREFAB: &str = "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_ObjectHideFlags: 0
  serializedVersion: 6
  m_Component:
  - component: {fileID: 400}
  - component: {fileID: 500}
  m_Layer: 0
  m_Name: Slime
  m_IsActive: 1
--- !u!4 &400
Transform:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 100}
  m_LocalRotation: {x: 0, y: 0.38268343, z: 0, w: 0.9238795}
  m_LocalPosition: {x: 0, y: 0, z: 0}
  m_LocalScale: {x: 1, y: 1, z: 1}
  m_Children:
  - {fileID: 410}
  m_Father: {fileID: 0}
--- !u!114 &500
MonoBehaviour:
  m_ObjectHideFlags: 0
  m_GameObject: {fileID: 100}
  m_Enabled: 1
  m_Script: {fileID: 11500000, guid: 4f1d0c7a2b9e4e3f8a6b5c4d3e2f1a09, type: 3}
  m_Name:
  moveSpeed: 2.5
  colour: {r: 1, g: 0.5, b: 0, a: 1}
  crown: {fileID: 410}
  waypoints:
  - {x: 1, y: 2}
  - {x: 3, y: 4}
--- !u!1 &110
GameObject:
  m_Component:
  - component: {fileID: 410}
  m_Name: Crown
--- !u!4 &410
Transform:
  m_GameObject: {fileID: 110}
  m_LocalPosition: {x: 0, y: 0.62, z: 0}
  m_Children: []
  m_Father: {fileID: 400}
";

    #[test]
    fn a_working_file_and_its_meta_read_through_the_unity_reader() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("unity-reader-parity")?;
        scratch.write("Slime.prefab", PREFAB)?;
        scratch.write(
            "SlimeController.cs.meta",
            "fileFormatVersion: 2\nguid: 4f1d0c7a2b9e4e3f8a6b5c4d3e2f1a09\n",
        )?;
        scratch.write("Slime.prefab.meta", "fileFormatVersion: 2\nguid: 0123456789abcdef0123456789abcdef\nPrefabImporter:\n  userData: \n")?;
        let (repo, _) = scratch.open()?;
        for (path, kind) in [("Slime.prefab", "prefab"), ("Slime.prefab.meta", "meta")] {
            let source = || {
                Some(Source {
                    revision: Revision::WorkTree,
                    path: OsString::from(path),
                })
            };
            let by = ByReader {
                reader: bundled_unity(),
                kind: kind.to_owned(),
            };
            assert!(read(&repo, path, None, source(), None).is_err(), "{path} needs a reader");
            let files = read(&repo, path, None, source(), Some(&by))?;
            let model = &files.after.as_ref().unwrap().model;
            let names: Vec<&str> = model.items.iter().map(|item| item.name.as_str()).collect();
            match kind {
                "prefab" => {
                    assert_eq!(names, ["Slime", "Crown"]);
                    assert_eq!(model.roots, [100]);
                    assert_eq!(model.children.get(&100), Some(&vec![110]));
                    let titles: Vec<&str> = model.items[0].parts.iter().map(|part| part.title.as_str()).collect();
                    assert_eq!(titles, ["GameObject", "Transform", "Slime Controller"]);
                }
                _ => {
                    assert_eq!(names, ["Slime.prefab"]);
                    let guid = model.items[0].parts[0].fields.iter().find(|field| field.path == "guid");
                    assert!(guid.is_some_and(|field| field.label == "GUID"));
                }
            }
        }
        Ok(())
    }
}
