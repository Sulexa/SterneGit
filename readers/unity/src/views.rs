use std::collections::{BTreeSet, HashMap};

use sternegit_reader_sdk::{
    protocol::{
        self, CurveKey, Curves, Declared, Edge, Facing, Graph, GraphKind, Graphic, Keys, Layer, Layout, Marker, Node,
        NodeKind, Note, ObjectKey, Paint, Picture, PlacedSprite, Port, PortType, Shape, Spot, Track,
    },
    read_text,
};

use crate::{
    animator, clip, convert, graph, hints::Scopes, layout, names::UnityKind, scene, sprites, value::Lookup,
    yaml::{Document, plain},
};

const FILE_ID: &str = "fileID: ";

pub(crate) fn picture_id(guid: &str, file_id: i64) -> String {
    format!("{guid}:{file_id}")
}

pub(crate) fn curves(docs: &[Document], lookup: &Lookup) -> Option<Curves> {
    let clip = clip::read_clip(docs, lookup)?;
    Some(Curves {
        tracks: clip
            .tracks
            .iter()
            .map(|track| Track {
                id: track.id.clone(),
                group: track.group.clone(),
                name: track.name.clone(),
                keys: match &track.keys {
                    clip::Keys::Curve(keys) => Keys::Curve(
                        keys.iter()
                            .map(|key| CurveKey {
                                time: key.time,
                                value: key.value,
                                in_slope: key.in_slope,
                                out_slope: key.out_slope,
                            })
                            .collect(),
                    ),
                    clip::Keys::Objects(keys) => Keys::Objects(
                        keys.iter()
                            .map(|key| ObjectKey {
                                time: key.time,
                                value: convert::value(&key.value),
                                picture: key.sprite.as_ref().map(|(guid, file_id)| picture_id(guid, *file_id)),
                            })
                            .collect(),
                    ),
                },
            })
            .collect(),
        events: clip.events,
        length: clip.length,
        sample_rate: clip.sample_rate,
        settings: clip.settings,
    })
}

fn node_kind(kind: graph::NodeKind) -> NodeKind {
    match kind {
        graph::NodeKind::Entry => NodeKind::Entry,
        graph::NodeKind::AnyState => NodeKind::AnyState,
        graph::NodeKind::Exit => NodeKind::Exit,
        graph::NodeKind::DefaultState => NodeKind::DefaultState,
        graph::NodeKind::State => NodeKind::State,
        graph::NodeKind::Machine => NodeKind::Machine,
        graph::NodeKind::Event => NodeKind::Event,
        graph::NodeKind::Flow => NodeKind::Flow,
        graph::NodeKind::Value => NodeKind::Value,
        graph::NodeKind::Variable => NodeKind::Variable,
    }
}

fn port_type(kind: graph::PortType) -> PortType {
    match kind {
        graph::PortType::Flow => PortType::Flow,
        graph::PortType::Float => PortType::Float,
        graph::PortType::Integer => PortType::Integer,
        graph::PortType::Bool => PortType::Bool,
        graph::PortType::Vector => PortType::Vector,
        graph::PortType::Text => PortType::Text,
        graph::PortType::Object => PortType::Object,
    }
}

fn ports(ports: &[graph::Port]) -> Vec<Port> {
    ports
        .iter()
        .map(|port| Port {
            key: port.key.clone(),
            label: port.label.clone(),
            kind: port_type(port.kind),
            literal: port.literal.clone(),
        })
        .collect()
}

fn node(key: String, node: &graph::GraphNode, fields: &[crate::value::Field]) -> Node {
    Node {
        key,
        id: node.id.clone(),
        title: node.title.clone(),
        kind: node_kind(node.kind),
        at: node.at,
        size: node.size,
        inputs: ports(&node.inputs),
        outputs: ports(&node.outputs),
        variable: node.variable.clone(),
        fields: convert::tree(fields),
    }
}

fn edge(edge: &graph::GraphEdge, fields: &[crate::value::Field]) -> Edge {
    Edge {
        id: edge.id.clone(),
        from: edge.from.clone(),
        to: edge.to.clone(),
        from_port: edge.from_port.clone(),
        to_port: edge.to_port.clone(),
        kind: port_type(edge.kind),
        fields: convert::tree(fields),
    }
}

fn declared(declared: &graph::Declared) -> Declared {
    Declared {
        name: declared.name.clone(),
        kind: declared.kind.clone(),
        value: declared.value.clone(),
        members: convert::tree(&declared.members),
    }
}

pub(crate) fn controller(docs: &[Document], lookup: &Lookup) -> Graph {
    Graph {
        kind: GraphKind::Controller,
        ports: false,
        layers: animator::drawn_layers(docs, lookup)
            .into_iter()
            .map(|(name, (nodes, edges))| Layer {
                name,
                nodes: nodes.iter().map(|(shown, fields)| node(shown.id.clone(), shown, fields)).collect(),
                edges: edges.iter().map(|(shown, fields)| edge(shown, fields)).collect(),
            })
            .collect(),
        parameters: animator::declared(docs).iter().map(declared).collect(),
    }
}

pub(crate) fn script_graph(docs: &[Document]) -> Option<Graph> {
    let json = graph::graph_json(docs)?;
    let (units, edges, variables) = graph::script_side(&json);
    Some(Graph {
        kind: GraphKind::Script,
        ports: !graph::is_state_graph(&json),
        layers: vec![Layer {
            name: String::new(),
            nodes: units.iter().map(|(key, shown, fields)| node(key.clone(), shown, fields)).collect(),
            edges: edges.iter().map(|shown| edge(shown, &[])).collect(),
        }],
        parameters: variables.iter().map(declared).collect(),
    })
}

fn shape(shape: &layout::Shape) -> Shape {
    match shape {
        layout::Shape::Circle { center, radius } => Shape::Circle {
            center: *center,
            radius: *radius,
        },
        layout::Shape::Box { center, size } => Shape::Box {
            center: *center,
            size: *size,
        },
    }
}

fn paint(paint: &layout::Paint) -> Paint {
    let picture = || paint.sprite().map(|(guid, file_id)| picture_id(&guid, file_id)).unwrap_or_default();
    match paint {
        layout::Paint::Fill => Paint::Fill,
        layout::Paint::Sprite {
            sliced,
            keep_aspect,
            unit,
            ..
        } => Paint::Sprite {
            picture: picture(),
            sliced: *sliced,
            keep_aspect: *keep_aspect,
            unit: *unit,
        },
        layout::Paint::Texture { .. } => Paint::Texture { picture: picture() },
        layout::Paint::Text { text, size, align } => Paint::Text {
            text: text.clone(),
            size: *size,
            align: *align,
        },
    }
}

pub(crate) fn layout(
    docs: &[Document],
    model: &scene::Model,
    library: &HashMap<String, Vec<Document>>,
    lookup: &Lookup,
) -> Layout {
    let view = layout::view_for(docs);
    let placed = layout::layout(docs, model, library, lookup, view);
    Layout {
        facing: match view {
            layout::View::Front => Facing::Front,
            layout::View::Above => Facing::Above,
        },
        spots: placed
            .spots
            .iter()
            .map(|spot| Spot {
                object: spot.key.to_string(),
                name: spot.name.clone(),
                at: spot.at,
                sprites: spot
                    .sprites
                    .iter()
                    .map(|sprite| PlacedSprite {
                        picture: picture_id(&sprite.guid, sprite.file_id),
                        at: sprite.at,
                        scale: sprite.scale,
                        flip: sprite.flip,
                        order: sprite.order,
                        alpha: sprite.alpha,
                    })
                    .collect(),
                shapes: spot.shapes.iter().map(|(name, found)| (name.clone(), shape(found))).collect(),
                camera: spot.camera,
                marker: match spot.marker {
                    layout::Marker::Plain => Marker::Plain,
                    layout::Marker::Camera => Marker::Camera,
                    layout::Marker::Light => Marker::Light,
                    layout::Marker::Canvas => Marker::Canvas,
                },
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
            })
            .collect(),
    }
}

pub(crate) fn pictures(model: &protocol::Model, lookup: &Lookup) -> Vec<Picture> {
    let mut wanted = BTreeSet::new();
    if let Some(layout) = &model.layout {
        for spot in &layout.spots {
            wanted.extend(spot.sprites.iter().map(|sprite| sprite.picture.clone()));
            wanted.extend(spot.graphics.iter().filter_map(|graphic| match &graphic.paint {
                Paint::Sprite { picture, .. } | Paint::Texture { picture } => Some(picture.clone()),
                Paint::Fill | Paint::Text { .. } => None,
            }));
        }
    }
    if let Some(curves) = &model.curves {
        for track in &curves.tracks {
            if let Keys::Objects(keys) = &track.keys {
                wanted.extend(keys.iter().filter_map(|key| key.picture.clone()));
            }
        }
    }
    let mut metas: HashMap<String, Option<crate::yaml::Node>> = HashMap::new();
    wanted
        .into_iter()
        .filter_map(|id| {
            let (guid, file_id) = id.rsplit_once(':')?;
            let file_id: i64 = file_id.parse().ok()?;
            let path = lookup.asset_path(guid)?.to_owned();
            let meta = metas
                .entry(path.clone())
                .or_insert_with(|| read_text(&format!("{path}.meta")).map(|text| plain(&text)))
                .clone()
                .unwrap_or(crate::yaml::Node::Map(Vec::new()));
            let frame = sprites::frame(&meta, file_id);
            Some(Picture {
                id,
                path,
                rect: frame.rect,
                pivot: frame.pivot,
                pixels_per_unit: frame.pixels_per_unit,
                border: frame.border,
            })
        })
        .collect()
}

pub(crate) fn scopes_and_notes(text: &str, docs: &[Document], lookup: &Lookup) -> (Vec<protocol::Scope>, Vec<Note>) {
    let scopes = docs
        .iter()
        .filter_map(|document| {
            let (name, kind) = lookup.objects.get(&document.file_id)?;
            Some(protocol::Scope {
                lines: (document.first_line as u32, document.last_line as u32),
                label: if kind == "GameObject" || name == kind { name.clone() } else { format!("{name} › {kind}") },
                object: Some(document.file_id.to_string()),
            })
        })
        .collect();
    let hinter = Scopes::new(docs, docs, lookup);
    let notes = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(FILE_ID))
        .filter_map(|(ix, line)| {
            Some(Note {
                line: ix as u32 + 1,
                text: hinter.hint(line)?,
            })
        })
        .collect();
    (scopes, notes)
}

pub(crate) fn readable_graph(docs: &[Document]) -> Option<String> {
    graph::graph_json(docs)
        .and_then(|json| serde_json::to_string_pretty(&json).ok())
        .map(|text| text + "
")
}

pub(crate) fn shaped(kind: UnityKind, docs: &[Document]) -> bool {
    kind == UnityKind::Asset && graph::graph_json(docs).is_some()
}
