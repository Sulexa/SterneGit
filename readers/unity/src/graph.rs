use std::collections::HashMap;

use serde_json::Value as Json;

use super::{
    names::{nicify, short_type},
    value::{Field, Value, short_number},
    yaml::Document,
};

const PORT_ROW: f64 = 20.;
const UNIT_HEADER: f64 = 26.;
const UNIT_FOOT: f64 = 6.;
const LETTER: f64 = 6.6;
const UNIT_SIDE_PADDING: f64 = 34.;
const SMALLEST_UNIT: f64 = 104.;
const EVENTS: [&str; 22] = [
    "Start",
    "Update",
    "FixedUpdate",
    "LateUpdate",
    "OnEnable",
    "OnDisable",
    "OnDestroy",
    "OnTriggerEnter",
    "OnTriggerExit",
    "OnTriggerStay",
    "OnTriggerEnter2D",
    "OnTriggerExit2D",
    "OnTriggerStay2D",
    "OnCollisionEnter",
    "OnCollisionExit",
    "OnCollisionEnter2D",
    "OnCollisionExit2D",
    "OnMouseDown",
    "OnMouseUp",
    "OnBecameVisible",
    "OnBecameInvisible",
    "CustomEvent",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PortType {
    Flow,
    Float,
    Integer,
    Bool,
    Vector,
    Text,
    Object,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Port {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: PortType,
    pub(crate) literal: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphNode {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) kind: NodeKind,
    pub(crate) at: (f64, f64),
    pub(crate) size: (f64, f64),
    pub(crate) inputs: Vec<Port>,
    pub(crate) outputs: Vec<Port>,
    pub(crate) variable: Option<String>,
}

impl GraphNode {
    pub(crate) fn rows(&self) -> usize {
        self.inputs.len().max(self.outputs.len())
    }

}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphEdge {
    pub(crate) id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) from_port: Option<String>,
    pub(crate) to_port: Option<String>,
    pub(crate) kind: PortType,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Declared {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) value: String,
    pub(crate) members: Vec<Field>,
}

fn member_fields(value: &Json, path: &str, depth: usize) -> Vec<Field> {
    let members: Vec<(String, String, &Json)> = match value.get("$content").unwrap_or(value) {
        Json::Array(items) => items
            .iter()
            .enumerate()
            .map(|(ix, item)| (format!("{path}[{ix}]"), format!("Element {ix}"), item))
            .collect(),
        Json::Object(map) if !map.contains_key("x") => map
            .iter()
            .filter(|(key, _)| !key.starts_with('$'))
            .map(|(key, inner)| (format!("{path}.{key}"), nicify(key), inner))
            .collect(),
        _ => Vec::new(),
    };
    members
        .into_iter()
        .flat_map(|(path, label, inner)| {
            let nested = member_fields(inner, &path, depth + 1);
            let shown = if nested.is_empty() { literal(inner, "value").unwrap_or_else(|| "None".to_owned()) } else { String::new() };
            std::iter::once(Field {
                path,
                label,
                depth,
                value: Value::Text(shown),
            })
            .chain(nested)
        })
        .collect()
}

pub(crate) fn graph_json(documents: &[Document]) -> Option<Json> {
    documents.iter().find_map(|document| {
        let text = document.body.at(&["_data", "_json"]).and_then(|node| node.text())?;
        let json: Json = serde_json::from_str(text).ok()?;
        json.get("graph").is_some().then_some(json)
    })
}

pub(crate) fn is_state_graph(json: &Json) -> bool {
    elements(json).iter().any(|element| type_of(element).ends_with("State") || type_of(element).ends_with("StateTransition"))
}

fn elements(json: &Json) -> &[Json] {
    json.pointer("/graph/elements").and_then(Json::as_array).map_or(&[], Vec::as_slice)
}

fn type_of(element: &Json) -> &str {
    element.get("$type").and_then(Json::as_str).unwrap_or_default()
}

fn last_segment(full: &str) -> &str {
    full.rsplit('.').next().unwrap_or(full)
}

fn is_connection(element: &Json) -> bool {
    let kind = type_of(element);
    kind.ends_with("Connection") || kind.ends_with("Transition")
}

fn unit_key(element: &Json) -> Option<String> {
    element
        .get("guid")
        .and_then(Json::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| element.get("$id").and_then(Json::as_str).map(|id| format!("id:{id}")))
}

fn literal(value: &Json, key: &str) -> Option<String> {
    let content = match value {
        Json::Object(map) if map.contains_key("$content") => &map["$content"],
        Json::Object(map) if map.contains_key("x") => value,
        Json::Null => {
            return (key == "target").then(|| "This".to_owned());
        }
        other => other,
    };
    Some(match content {
        Json::Null => return (key == "target").then(|| "This".to_owned()),
        Json::Bool(on) => if *on { "On" } else { "Off" }.to_owned(),
        Json::Number(number) => short_number(number.as_f64().unwrap_or(0.)),
        Json::String(text) => match value.get("$type").and_then(Json::as_str) {
            Some(kind) if kind.contains("String") => format!("\"{text}\""),
            _ => text.clone(),
        },
        Json::Object(map) => ["x", "y", "z", "w"]
            .iter()
            .filter_map(|axis| Some(format!("{} {}", axis.to_uppercase(), short_number(map.get(*axis)?.as_f64()?))))
            .collect::<Vec<_>>()
            .join(" "),
        Json::Array(items) => format!("{} items", items.len()),
    })
}

fn port_type(value: Option<&Json>) -> PortType {
    let Some(kind) = value.and_then(|value| value.get("$type")).and_then(Json::as_str) else {
        return PortType::Object;
    };
    match last_segment(kind) {
        "Single" | "Double" => PortType::Float,
        "Int32" | "Int64" => PortType::Integer,
        "Boolean" => PortType::Bool,
        "Vector2" | "Vector3" | "Vector4" | "Quaternion" => PortType::Vector,
        "String" => PortType::Text,
        _ => PortType::Object,
    }
}

fn port_label(key: &str) -> String {
    match key {
        "enter" | "exit" | "trigger" | "%enter" => String::new(),
        "ifTrue" => "True".to_owned(),
        "ifFalse" => "False".to_owned(),
        "a" | "b" | "%a" | "%b" => key.trim_start_matches('%').to_uppercase(),
        _ => nicify(key.trim_start_matches('%')),
    }
}

fn unit_title(element: &Json) -> (String, NodeKind) {
    let full = type_of(element);
    let short = last_segment(full);
    if EVENTS.contains(&short) {
        let name = if short == "CustomEvent" { "Custom Event".to_owned() } else { format!("On {}", nicify(short.trim_start_matches("On"))) };
        return (name, NodeKind::Event);
    }
    let member = element.get("member");
    let member_title = member.and_then(|member| {
        let name = member.get("name").and_then(Json::as_str)?;
        let target = member
            .get("targetTypeName")
            .or_else(|| member.get("targetType"))
            .and_then(Json::as_str)
            .map(short_type)
            .unwrap_or_default();
        Some(if name == ".ctor" { format!("Create {target}") } else if target.is_empty() { nicify(name) } else { format!("{target}: {}", nicify(name)) })
    });
    let title = match short {
        "If" | "Branch" => "If".to_owned(),
        "ScalarMultiply" | "GenericMultiply" | "Vector2Multiply" | "Vector3Multiply" => "Multiply".to_owned(),
        "ScalarSum" | "GenericSum" | "Vector2Sum" | "Vector3Sum" => "Add".to_owned(),
        "ScalarSubtract" | "GenericSubtract" => "Subtract".to_owned(),
        "ScalarDivide" | "GenericDivide" => "Divide".to_owned(),
        "GetVariable" => "Get Variable".to_owned(),
        "SetVariable" => "Set Variable".to_owned(),
        "Literal" => element
            .get("type")
            .and_then(Json::as_str)
            .map_or_else(|| "Literal".to_owned(), short_type),
        "InvokeMember" | "GetMember" | "SetMember" => member_title.unwrap_or_else(|| nicify(short)),
        _ => nicify(short),
    };
    let kind = match short {
        "GetVariable" | "SetVariable" => NodeKind::Variable,
        _ => NodeKind::Flow,
    };
    (title, kind)
}

fn known_ports(element: &Json) -> (Vec<Port>, Vec<Port>) {
    let short = last_segment(type_of(element));
    let flow = |key: &str| Port {
        key: key.to_owned(),
        label: port_label(key),
        kind: PortType::Flow,
        literal: None,
    };
    let value = |key: &str, kind: PortType| Port {
        key: key.to_owned(),
        label: port_label(key),
        kind,
        literal: None,
    };
    if EVENTS.contains(&short) {
        return (Vec::new(), vec![flow("trigger")]);
    }
    match short {
        "If" | "Branch" => (vec![flow("enter"), value("condition", PortType::Bool)], vec![flow("ifTrue"), flow("ifFalse")]),
        "Cooldown" => (vec![flow("enter"), value("duration", PortType::Float)], vec![flow("ready"), flow("notReady")]),
        "InvokeMember" | "SetMember" | "SetVariable" => (vec![flow("enter")], vec![flow("exit")]),
        _ => (Vec::new(), Vec::new()),
    }
}

struct Side {
    units: Vec<(String, GraphNode)>,
    edges: Vec<GraphEdge>,
    variables: Vec<Declared>,
}

fn graph_variable(element: &Json) -> Option<String> {
    if element.get("kind").and_then(Json::as_str) != Some("Graph") {
        return None;
    }
    element.pointer("/defaultValues/name/$content").and_then(Json::as_str).map(ToOwned::to_owned)
}

fn read_side(json: &Json) -> Side {
    let elements = elements(json);
    let ids: HashMap<&str, String> = elements
        .iter()
        .filter(|element| !is_connection(element))
        .filter_map(|element| Some((element.get("$id")?.as_str()?, unit_key(element)?)))
        .collect();
    let reference = |element: &Json, field: &str| -> Option<String> {
        let target = element.get(field)?;
        let id = target.get("$ref").or_else(|| target.get("$id")).and_then(Json::as_str)?;
        ids.get(id).cloned()
    };
    let mut edges = Vec::new();
    for element in elements.iter().filter(|element| is_connection(element)) {
        let kind = type_of(element);
        let (Some(from), Some(to)) = (
            reference(element, "sourceUnit").or_else(|| reference(element, "source")),
            reference(element, "destinationUnit").or_else(|| reference(element, "destination")),
        ) else {
            continue;
        };
        let from_port = element.get("sourceKey").and_then(Json::as_str).map(ToOwned::to_owned);
        let to_port = element.get("destinationKey").and_then(Json::as_str).map(ToOwned::to_owned);
        let id = element.get("guid").and_then(Json::as_str).map_or_else(
            || format!("{from}:{}>{to}:{}", from_port.clone().unwrap_or_default(), to_port.clone().unwrap_or_default()),
            ToOwned::to_owned,
        );
        let flow = kind.ends_with("ControlConnection") || kind.ends_with("Transition");
        edges.push(GraphEdge {
            id,
            from,
            to,
            from_port,
            to_port,
            kind: if flow { PortType::Flow } else { PortType::Object },
        });
    }
    let mut units = Vec::new();
    for element in elements.iter().filter(|element| !is_connection(element)) {
        let Some(key) = unit_key(element) else {
            continue;
        };
        let (mut title, mut kind) = unit_title(element);
        if type_of(element).ends_with("State") {
            kind = if element.get("isStart").and_then(Json::as_bool) == Some(true) { NodeKind::DefaultState } else { NodeKind::State };
            title = element
                .get("nest")
                .and_then(|nest| nest.get("embed"))
                .and_then(|embed| embed.get("title"))
                .and_then(Json::as_str)
                .filter(|title| !title.is_empty())
                .map(ToOwned::to_owned)
                .or_else(|| element.get("title").and_then(Json::as_str).map(ToOwned::to_owned))
                .unwrap_or_else(|| if type_of(element).ends_with("AnyState") { "Any State".to_owned() } else { nicify(last_segment(type_of(element))) });
            if type_of(element).ends_with("AnyState") {
                kind = NodeKind::AnyState;
            }
        }
        let (mut inputs, mut outputs) = known_ports(element);
        let defaults = element.get("defaultValues").and_then(Json::as_object);
        if let Some(defaults) = defaults {
            for (port, value) in defaults {
                if port == "name" && matches!(kind, NodeKind::Variable) {
                    title = format!("{title}: {}", literal(value, port).unwrap_or_default().trim_matches('"'));
                    continue;
                }
                let found = inputs.iter_mut().find(|known| known.key == *port);
                match found {
                    Some(known) => {
                        known.literal = literal(value, port);
                        if known.kind != PortType::Flow {
                            known.kind = port_type(Some(value)).max_with(known.kind);
                        }
                    }
                    None => inputs.push(Port {
                        key: port.clone(),
                        label: port_label(port),
                        kind: port_type(Some(value)),
                        literal: literal(value, port),
                    }),
                }
            }
        }
        for edge in &edges {
            if edge.to == key
                && let Some(port) = &edge.to_port
                && !inputs.iter().any(|known| &known.key == port)
            {
                inputs.push(Port {
                    key: port.clone(),
                    label: port_label(port),
                    kind: edge.kind,
                    literal: None,
                });
            }
            if edge.from == key
                && let Some(port) = &edge.from_port
                && !outputs.iter().any(|known| &known.key == port)
            {
                outputs.push(Port {
                    key: port.clone(),
                    label: port_label(port),
                    kind: edge.kind,
                    literal: None,
                });
            }
        }
        inputs.sort_by_key(|port| port.kind != PortType::Flow);
        outputs.sort_by_key(|port| port.kind != PortType::Flow);
        let at = element
            .get("position")
            .map(|position| {
                (
                    position.get("x").and_then(Json::as_f64).unwrap_or(0.),
                    position.get("y").and_then(Json::as_f64).unwrap_or(0.),
                )
            })
            .unwrap_or_default();
        let is_state = matches!(kind, NodeKind::State | NodeKind::DefaultState | NodeKind::AnyState);
        let flowing = inputs.iter().chain(&outputs).any(|port| port.kind == PortType::Flow);
        if !is_state && !matches!(kind, NodeKind::Event | NodeKind::Variable) && !flowing {
            kind = NodeKind::Value;
        }
        units.push((
            key.clone(),
            GraphNode {
                id: key,
                title,
                kind,
                at,
                size: (0., 0.),
                inputs,
                outputs,
                variable: graph_variable(element),
            },
        ));
    }
    let unit_ports: HashMap<String, Vec<Port>> = units.iter().map(|(key, node)| (key.clone(), node.inputs.clone())).collect();
    for edge in &mut edges {
        if edge.kind != PortType::Flow
            && let Some(inputs) = unit_ports.get(&edge.to)
            && let Some(port) = inputs.iter().find(|port| Some(&port.key) == edge.to_port.as_ref())
        {
            edge.kind = port.kind;
        }
    }
    for (_, node) in &mut units {
        let widest = node
            .inputs
            .iter()
            .map(|port| port.label.len() + port.literal.as_ref().map_or(0, |literal| literal.len() + 2))
            .max()
            .unwrap_or(0)
            + node.outputs.iter().map(|port| port.label.len()).max().unwrap_or(0);
        let width = (node.title.len().max(widest) as f64 * LETTER + UNIT_SIDE_PADDING).max(SMALLEST_UNIT);
        let height = if matches!(node.kind, NodeKind::State | NodeKind::DefaultState | NodeKind::AnyState) {
            40.
        } else {
            UNIT_HEADER + node.rows() as f64 * PORT_ROW + UNIT_FOOT
        };
        node.size = (width, height);
    }
    let variables = json
        .pointer("/graph/variables/collection/$content")
        .and_then(Json::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|variable| {
                    Some(Declared {
                        name: variable.get("name")?.as_str()?.to_owned(),
                        kind: variable
                            .pointer("/typeHandle/Identification")
                            .and_then(Json::as_str)
                            .map(short_type)
                            .unwrap_or_default(),
                        value: variable.get("value").and_then(|value| literal(value, "value")).unwrap_or_else(|| "None".to_owned()),
                        members: variable.get("value").map(|value| member_fields(value, "value", 1)).unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Side { units, edges, variables }
}

trait Strongest {
    fn max_with(self, other: PortType) -> PortType;
}

impl Strongest for PortType {
    fn max_with(self, other: PortType) -> PortType {
        if self == PortType::Object { other } else { self }
    }
}

fn input_text(node: &GraphNode, port: &Port, edges: &[GraphEdge], nodes: &HashMap<&str, &GraphNode>) -> String {
    let fed = edges
        .iter()
        .find(|edge| edge.to == node.id && edge.to_port.as_deref() == Some(port.key.as_str()));
    match fed {
        Some(edge) => {
            let source = nodes.get(edge.from.as_str()).map_or_else(String::new, |source| source.title.clone());
            let out = edge.from_port.as_deref().map(port_label).filter(|label| !label.is_empty());
            match out {
                Some(out) => format!("← {source} ({out})"),
                None => format!("← {source}"),
            }
        }
        None => port.literal.clone().unwrap_or_default(),
    }
}

fn node_fields(node: &GraphNode, edges: &[GraphEdge], nodes: &HashMap<&str, &GraphNode>) -> Vec<Field> {
    let mut fields = vec![Field {
        path: "title".to_owned(),
        label: "Node".to_owned(),
        depth: 0,
        value: Value::Text(node.title.clone()),
    }];
    for port in node.inputs.iter().filter(|port| port.kind != PortType::Flow) {
        fields.push(Field {
            path: format!("in:{}", port.key),
            label: if port.label.is_empty() { nicify(&port.key) } else { port.label.clone() },
            depth: 0,
            value: Value::Text(input_text(node, port, edges, nodes)),
        });
    }
    fields
}

pub(crate) type ScriptSide = (Vec<(String, GraphNode, Vec<Field>)>, Vec<GraphEdge>, Vec<Declared>);

pub(crate) fn script_side(json: &Json) -> ScriptSide {
    let side = read_side(json);
    let nodes: HashMap<&str, &GraphNode> = side.units.iter().map(|(key, node)| (key.as_str(), node)).collect();
    let units = side
        .units
        .iter()
        .map(|(key, node)| (key.clone(), node.clone(), node_fields(node, &side.edges, &nodes)))
        .collect();
    (units, side.edges.clone(), side.variables.clone())
}
