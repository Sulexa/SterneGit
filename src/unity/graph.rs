use std::collections::{HashMap, HashSet};

use serde_json::Value as Json;

use super::{
    compare::{Options, Row, State, changed_rows},
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
    pub(crate) was: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphNode {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) kind: NodeKind,
    pub(crate) at: (f64, f64),
    pub(crate) size: (f64, f64),
    pub(crate) state: State,
    pub(crate) inputs: Vec<Port>,
    pub(crate) outputs: Vec<Port>,
    pub(crate) was_at: Option<(f64, f64)>,
    pub(crate) variable: Option<String>,
}

impl GraphNode {
    pub(crate) fn rows(&self) -> usize {
        self.inputs.len().max(self.outputs.len())
    }

    pub(crate) fn input_y(&self, key: &str) -> f64 {
        let row = self.inputs.iter().position(|port| port.key == key).unwrap_or(0);
        self.at.1 + UNIT_HEADER + (row as f64 + 0.5) * PORT_ROW
    }

    pub(crate) fn output_y(&self, key: &str) -> f64 {
        let row = self.outputs.iter().position(|port| port.key == key).unwrap_or(0);
        self.at.1 + UNIT_HEADER + (row as f64 + 0.5) * PORT_ROW
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphEdge {
    pub(crate) id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) from_port: Option<String>,
    pub(crate) to_port: Option<String>,
    pub(crate) state: State,
    pub(crate) kind: PortType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChangeTarget {
    Node(usize),
    Edge(usize),
    Parameter(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphParameter {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) value: Option<String>,
    pub(crate) was: Option<String>,
    pub(crate) was_kind: Option<String>,
    pub(crate) state: State,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Declared {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) value: String,
    pub(crate) members: Vec<Field>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ParameterLook {
    pub(crate) value_label: &'static str,
    pub(crate) value_shown: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GraphChange {
    pub(crate) state: State,
    pub(crate) kind: &'static str,
    pub(crate) name: String,
    pub(crate) detail: String,
    pub(crate) target: ChangeTarget,
    pub(crate) rows: Vec<Row>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct GraphComparison {
    pub(crate) nodes: Vec<GraphNode>,
    pub(crate) edges: Vec<GraphEdge>,
    pub(crate) parameters: Vec<GraphParameter>,
    pub(crate) parameters_title: &'static str,
    pub(crate) changes: Vec<GraphChange>,
    pub(crate) layers: Vec<String>,
    pub(crate) has_ports: bool,
    pub(crate) hidden: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Shown {
    Before,
    #[default]
    Both,
    After,
}

impl Shown {
    pub(crate) fn hides(self, state: State) -> bool {
        matches!((self, state), (Shown::Before, State::Added) | (Shown::After, State::Removed))
    }
}

impl GraphComparison {
    pub(crate) fn as_shown(&self, shown: Shown) -> GraphComparison {
        let mut drawn = self.clone();
        if shown != Shown::Before {
            return drawn;
        }
        for node in &mut drawn.nodes {
            if let Some(at) = node.was_at {
                node.at = at;
            }
            for port in &mut node.inputs {
                if let Some(was) = &port.was {
                    port.literal = (!was.is_empty()).then(|| was.clone());
                }
            }
        }
        for parameter in &mut drawn.parameters {
            if let Some(was) = &parameter.was {
                parameter.value = (!was.is_empty()).then(|| was.clone());
            }
            if let Some(kind) = &parameter.was_kind {
                parameter.kind = kind.clone();
            }
        }
        drawn
    }

    pub(crate) fn users_of(&self, parameter: usize) -> impl Iterator<Item = usize> + '_ {
        let name = self.parameters.get(parameter).map(|parameter| parameter.name.as_str());
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(_, node)| name.is_some() && node.variable.as_deref() == name)
            .map(|(ix, _)| ix)
    }
}

fn declared_fields(declared: &Declared, look: ParameterLook) -> Vec<Field> {
    let mut fields = vec![Field {
        path: "type".to_owned(),
        label: "Type".to_owned(),
        depth: 0,
        value: Value::Text(declared.kind.clone()),
    }];
    if !declared.value.is_empty() || !declared.members.is_empty() {
        fields.push(Field {
            path: "value".to_owned(),
            label: look.value_label.to_owned(),
            depth: 0,
            value: Value::Text(declared.value.clone()),
        });
    }
    fields.extend(declared.members.iter().cloned());
    fields
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

fn declared_text(declared: &Declared) -> String {
    format!("{} {}", declared.kind, declared.value).trim().to_owned()
}

pub(crate) fn compare_parameters(
    old: &[Declared],
    new: &[Declared],
    kind: &'static str,
    look: ParameterLook,
    options: Options,
) -> (Vec<GraphParameter>, Vec<GraphChange>) {
    let mut parameters = Vec::new();
    let mut changes = Vec::new();
    let shown_value = |declared: &Declared| (look.value_shown && !declared.value.is_empty()).then(|| declared.value.clone());
    for declared in new {
        let previous = old.iter().find(|previous| previous.name == declared.name);
        let after = declared_fields(declared, look);
        let before = previous.map(|previous| declared_fields(previous, look));
        let (rows, _) = changed_rows(before.as_deref(), Some(&after), options);
        let state = match previous {
            None => State::Added,
            Some(previous) if previous == declared => State::Same,
            Some(_) => State::Changed,
        };
        let detail = match state {
            State::Changed => rows.iter().filter(|row| row.state != State::Same).map(|row| row.label.clone()).collect::<Vec<_>>().join(", "),
            _ => declared_text(declared),
        };
        if state != State::Same {
            changes.push(GraphChange {
                state,
                kind,
                name: declared.name.clone(),
                detail,
                target: ChangeTarget::Parameter(parameters.len()),
                rows,
            });
        }
        parameters.push(GraphParameter {
            name: declared.name.clone(),
            kind: declared.kind.clone(),
            value: shown_value(declared),
            was: previous.filter(|previous| look.value_shown && previous.value != declared.value).map(|previous| previous.value.clone()),
            was_kind: previous.filter(|previous| previous.kind != declared.kind).map(|previous| previous.kind.clone()),
            state,
        });
    }
    for declared in old {
        if !new.iter().any(|kept| kept.name == declared.name) {
            let (rows, _) = changed_rows(Some(&declared_fields(declared, look)), None, options);
            changes.push(GraphChange {
                state: State::Removed,
                kind,
                name: declared.name.clone(),
                detail: declared_text(declared),
                target: ChangeTarget::Parameter(parameters.len()),
                rows,
            });
            parameters.push(GraphParameter {
                name: declared.name.clone(),
                kind: declared.kind.clone(),
                value: shown_value(declared),
                was: None,
                was_kind: None,
                state: State::Removed,
            });
        }
    }
    (parameters, changes)
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
        was: None,
    };
    let value = |key: &str, kind: PortType| Port {
        key: key.to_owned(),
        label: port_label(key),
        kind,
        literal: None,
        was: None,
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
            state: State::Same,
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
                        was: None,
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
                    was: None,
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
                    was: None,
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
                state: State::Same,
                inputs,
                outputs,
                was_at: None,
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

pub(crate) fn compare_script_graphs(before: Option<&Json>, after: Option<&Json>, options: Options) -> GraphComparison {
    let empty = Json::Null;
    let old = read_side(before.unwrap_or(&empty));
    let new = read_side(after.unwrap_or(&empty));
    let old_nodes: HashMap<&str, &GraphNode> = old.units.iter().map(|(key, node)| (key.as_str(), node)).collect();
    let new_nodes: HashMap<&str, &GraphNode> = new.units.iter().map(|(key, node)| (key.as_str(), node)).collect();
    let mut nodes = Vec::new();
    let mut changes = Vec::new();
    let mut hidden = 0;
    for (key, node) in &new.units {
        let previous = old_nodes.get(key.as_str()).copied();
        let mut shown = node.clone();
        let after_fields = node_fields(node, &new.edges, &new_nodes);
        let before_fields = previous.map(|previous| node_fields(previous, &old.edges, &old_nodes));
        let (rows, noise) = changed_rows(before_fields.as_deref(), Some(&after_fields), options);
        hidden += noise;
        shown.state = match previous {
            None => State::Added,
            Some(_) if rows.iter().any(|row| row.state != State::Same) => State::Changed,
            Some(_) => State::Same,
        };
        if let Some(previous) = previous {
            if previous.at != node.at {
                shown.was_at = Some(previous.at);
            }
            for port in &mut shown.inputs {
                let was = previous.inputs.iter().find(|old| old.key == port.key).and_then(|old| old.literal.clone());
                if was != port.literal {
                    port.was = Some(was.unwrap_or_default());
                }
            }
        }
        if shown.state != State::Same {
            let detail = rows
                .iter()
                .filter(|row| row.state != State::Same && row.label != "Node")
                .map(|row| row.label.clone())
                .collect::<Vec<_>>()
                .join(", ");
            changes.push(GraphChange {
                state: shown.state,
                kind: "Node",
                name: shown.title.clone(),
                detail,
                target: ChangeTarget::Node(nodes.len()),
                rows,
            });
        }
        nodes.push(shown);
    }
    for (key, node) in &old.units {
        if !new_nodes.contains_key(key.as_str()) {
            let mut shown = node.clone();
            shown.state = State::Removed;
            let (rows, _) = changed_rows(Some(&node_fields(node, &old.edges, &old_nodes)), None, options);
            changes.push(GraphChange {
                state: State::Removed,
                kind: "Node",
                name: shown.title.clone(),
                detail: shown.inputs.iter().find_map(|port| port.literal.clone()).unwrap_or_default(),
                target: ChangeTarget::Node(nodes.len()),
                rows,
            });
            nodes.push(shown);
        }
    }
    let titles: HashMap<String, String> = nodes.iter().map(|node| (node.id.clone(), node.title.clone())).collect();
    let mut edges = Vec::new();
    let old_edges: HashSet<&str> = old.edges.iter().map(|edge| edge.id.as_str()).collect();
    let new_edges: HashSet<&str> = new.edges.iter().map(|edge| edge.id.as_str()).collect();
    let describe = |edge: &GraphEdge| {
        let from = titles.get(&edge.from).cloned().unwrap_or_default();
        let to = edge
            .to_port
            .as_deref()
            .map(port_label)
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| titles.get(&edge.to).cloned().unwrap_or_default());
        format!("{from} → {to}")
    };
    for edge in &new.edges {
        let mut shown = edge.clone();
        if !old_edges.contains(edge.id.as_str()) {
            shown.state = State::Added;
            changes.push(edge_change(&shown, describe(&shown), edges.len()));
        }
        edges.push(shown);
    }
    for edge in &old.edges {
        if !new_edges.contains(edge.id.as_str()) {
            let mut shown = edge.clone();
            shown.state = State::Removed;
            changes.push(edge_change(&shown, describe(&shown), edges.len()));
            edges.push(shown);
        }
    }
    let look = ParameterLook {
        value_label: "Value",
        value_shown: true,
    };
    let (parameters, mut variable_changes) = compare_parameters(&old.variables, &new.variables, "Variable", look, options);
    variable_changes.append(&mut changes);
    let states_graph = before.or(after).is_some_and(is_state_graph);
    GraphComparison {
        nodes,
        edges,
        parameters,
        parameters_title: "Graph variables",
        changes: variable_changes,
        layers: Vec::new(),
        has_ports: !states_graph,
        hidden,
    }
}

fn edge_change(edge: &GraphEdge, name: String, at: usize) -> GraphChange {
    GraphChange {
        state: edge.state,
        kind: if edge.kind == PortType::Flow && edge.from_port.is_none() { "Transition" } else { "Connection" },
        name,
        detail: match edge.kind {
            PortType::Flow => "Flow",
            PortType::Float => "Float",
            PortType::Integer => "Integer",
            PortType::Bool => "Boolean",
            PortType::Vector => "Vector",
            PortType::Text => "String",
            PortType::Object => "Value",
        }
        .to_owned(),
        target: ChangeTarget::Edge(at),
        rows: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{NodeKind, PortType, Shown, compare_script_graphs};
    use crate::unity::{
        compare::{Options, State},
        value::Value,
    };
    use serde_json::json;

    fn graph(duration: f64, with_multiply: bool, with_log: bool) -> serde_json::Value {
        let mut elements = vec![
            json!({"$type": "Unity.VisualScripting.Update", "$id": "1", "guid": "u-update", "position": {"x": 0, "y": 0}, "defaultValues": {}}),
            json!({"$type": "Unity.VisualScripting.Cooldown", "$id": "2", "guid": "u-cooldown", "position": {"x": 200, "y": 0},
                "defaultValues": {"duration": {"$content": duration, "$type": "System.Single"}}}),
            json!({"$type": "Unity.VisualScripting.InvokeMember", "$id": "3", "guid": "u-vector", "position": {"x": 300, "y": 200},
                "member": {"name": ".ctor", "targetType": "UnityEngine.Vector2"},
                "defaultValues": {"%x": {"$content": 0.0, "$type": "System.Single"}, "%y": {"$content": 6.0, "$type": "System.Single"}}}),
            json!({"$type": "Unity.VisualScripting.ControlConnection", "guid": "c-1", "sourceUnit": {"$ref": "1"}, "sourceKey": "trigger",
                "destinationUnit": {"$ref": "2"}, "destinationKey": "enter"}),
        ];
        if with_multiply {
            elements.push(json!({"$type": "Unity.VisualScripting.ScalarMultiply", "$id": "4", "guid": "u-multiply", "position": {"x": 100, "y": 300},
                "defaultValues": {"a": {"$content": 7.5, "$type": "System.Single"}, "b": {"$content": 1.0, "$type": "System.Single"}}}));
            elements.push(json!({"$type": "Unity.VisualScripting.ValueConnection", "guid": "v-1", "sourceUnit": {"$ref": "4"}, "sourceKey": "product",
                "destinationUnit": {"$ref": "3"}, "destinationKey": "%y"}));
        }
        if with_log {
            elements.push(json!({"$type": "Unity.VisualScripting.InvokeMember", "$id": "5", "guid": "u-log", "position": {"x": 400, "y": 0},
                "member": {"name": "Log", "targetType": "UnityEngine.Debug"},
                "defaultValues": {"%message": {"$content": "jump", "$type": "System.String"}}}));
            elements.push(json!({"$type": "Unity.VisualScripting.ControlConnection", "guid": "c-2", "sourceUnit": {"$ref": "2"}, "sourceKey": "ready",
                "destinationUnit": {"$ref": "5"}, "destinationKey": "enter"}));
        }
        json!({"graph": {"variables": {"Kind": "Flow", "collection": {"$content": [
            {"name": "Speed", "value": {"$content": 3, "$type": "System.Single"}, "typeHandle": {"Identification": "System.Single"}}
        ]}}, "elements": elements}})
    }

    #[test]
    fn nodes_and_connections_that_came_or_went_are_listed_with_changed_values() {
        let before = graph(0.8, false, true);
        let after = graph(0.6, true, false);
        let comparison = compare_script_graphs(Some(&before), Some(&after), Options::default());
        let changes: Vec<(&str, &str, State)> = comparison
            .changes
            .iter()
            .map(|change| (change.kind, change.name.as_str(), change.state))
            .collect();
        assert!(changes.contains(&("Node", "Cooldown", State::Changed)));
        assert!(changes.contains(&("Node", "Multiply", State::Added)));
        assert!(changes.contains(&("Node", "Debug: Log", State::Removed)));
        assert!(changes.contains(&("Connection", "Multiply → Y", State::Added)));
        assert!(changes.contains(&("Node", "Create Vector 2", State::Changed)));
    }

    #[test]
    fn a_value_that_is_now_connected_reads_as_coming_from_its_node() {
        let comparison = compare_script_graphs(Some(&graph(0.8, false, false)), Some(&graph(0.8, true, false)), Options::default());
        let vector = comparison.changes.iter().find(|change| change.name == "Create Vector 2").map(|change| &change.rows);
        let y = vector.and_then(|rows| rows.iter().find(|row| row.label == "Y"));
        assert_eq!(y.and_then(|row| row.before.as_ref()).map(|value| value.text()), Some("6".to_owned()));
        assert_eq!(y.and_then(|row| row.after.as_ref()).map(|value| value.text()), Some("← Multiply (Product)".to_owned()));
    }

    #[test]
    fn the_version_before_reads_old_values_and_hides_what_was_added() {
        let comparison = compare_script_graphs(Some(&graph(0.8, false, true)), Some(&graph(0.6, true, false)), Options::default());
        let before = comparison.as_shown(Shown::Before);
        let cooldown = before.nodes.iter().find(|node| node.title == "Cooldown");
        let duration = cooldown.and_then(|node| node.inputs.iter().find(|port| port.key == "duration"));
        assert_eq!(duration.and_then(|port| port.literal.clone()), Some("0.8".to_owned()));
        let shown: Vec<&str> = before.nodes.iter().filter(|node| !Shown::Before.hides(node.state)).map(|node| node.title.as_str()).collect();
        assert!(shown.contains(&"Debug: Log"));
        assert!(!shown.contains(&"Multiply"));
        let after: Vec<&str> = comparison.nodes.iter().filter(|node| !Shown::After.hides(node.state)).map(|node| node.title.as_str()).collect();
        assert!(after.contains(&"Multiply"));
        assert!(!after.contains(&"Debug: Log"));
    }

    fn with_variables(mut graph: serde_json::Value, variables: &[(&str, serde_json::Value, &str)]) -> serde_json::Value {
        let declared: Vec<serde_json::Value> = variables
            .iter()
            .map(|(name, value, kind)| json!({"name": name, "value": value, "typeHandle": {"Identification": format!("{kind}, mscorlib, Version=4.0.0.0, Culture=neutral")}}))
            .collect();
        graph["graph"]["variables"]["collection"]["$content"] = json!(declared);
        if let Some(elements) = graph["graph"]["elements"].as_array_mut() {
            elements.push(json!({"$type": "Unity.VisualScripting.GetVariable", "$id": "9", "guid": "u-get-speed", "kind": "Graph",
                "position": {"x": 0, "y": 400}, "defaultValues": {"name": {"$content": "Speed", "$type": "System.String"}}}));
            elements.push(json!({"$type": "Unity.VisualScripting.SetVariable", "$id": "10", "guid": "u-set-speed", "kind": "Object",
                "position": {"x": 200, "y": 400}, "defaultValues": {"name": {"$content": "Speed", "$type": "System.String"}}}));
        }
        graph
    }

    #[test]
    fn graph_variables_come_first_with_their_values_before_and_after() {
        let int = |value: i64| json!({"$content": value, "$type": "System.Int32"});
        let before = with_variables(graph(0.8, false, false), &[("Speed", int(10), "System.Int32"), ("Lives", int(3), "System.Int32")]);
        let after = with_variables(
            graph(0.8, false, false),
            &[("Speed", int(6), "System.Int32"), ("Drops", json!({"$content": [], "$type": "System.Collections.Generic.List`1[[Item, Game]]"}), "System.Collections.Generic.List`1[[Item, Game]]")],
        );
        let comparison = compare_script_graphs(Some(&before), Some(&after), Options::default());
        let firsts: Vec<(&str, &str, State, &str)> = comparison
            .changes
            .iter()
            .take(3)
            .map(|change| (change.kind, change.name.as_str(), change.state, change.detail.as_str()))
            .collect();
        assert_eq!(
            firsts,
            vec![
                ("Variable", "Speed", State::Changed, "Value"),
                ("Variable", "Drops", State::Added, "List of Item 0 items"),
                ("Variable", "Lives", State::Removed, "Integer 3"),
            ]
        );
        let value = comparison.changes[0].rows.iter().find(|row| row.label == "Value");
        assert_eq!(value.and_then(|row| row.before.as_ref()).map(|value| value.text()), Some("10".to_owned()));
        assert_eq!(value.and_then(|row| row.after.as_ref()).map(|value| value.text()), Some("6".to_owned()));
        let speed = comparison.parameters.iter().find(|parameter| parameter.name == "Speed");
        assert_eq!(speed.map(|speed| (speed.kind.as_str(), speed.value.as_deref(), speed.was.as_deref())), Some(("Integer", Some("6"), Some("10"))));
        let before_speed = comparison.as_shown(Shown::Before).parameters.into_iter().find(|parameter| parameter.name == "Speed");
        assert_eq!(before_speed.and_then(|speed| speed.value), Some("10".to_owned()));
    }

    #[test]
    fn a_variable_holding_an_object_compares_its_fields() {
        let modifier = |value: i64| json!({"StatType": "OFFER_COST", "ModifierType": "ADD", "Value": value, "$type": "PassiveStatModifier"});
        let before = with_variables(graph(0.8, false, false), &[("OffersPrice", modifier(-20), "PassiveStatModifier")]);
        let after = with_variables(graph(0.8, false, false), &[("OffersPrice", modifier(-15), "PassiveStatModifier")]);
        let comparison = compare_script_graphs(Some(&before), Some(&after), Options::default());
        let change = comparison.changes.iter().find(|change| change.name == "OffersPrice");
        assert_eq!(change.map(|change| (change.state, change.detail.as_str())), Some((State::Changed, "Value")));
        let rows: Vec<(usize, &str, Option<String>, Option<String>)> = change
            .map(|change| change.rows.iter().map(|row| (row.depth, row.label.as_str(), row.before.as_ref().map(Value::text), row.after.as_ref().map(Value::text))).collect())
            .unwrap_or_default();
        assert_eq!(rows, vec![(0, "Value", Some(String::new()), Some(String::new())), (1, "Value", Some("-20".to_owned()), Some("-15".to_owned()))]);
        let added = compare_script_graphs(None, Some(&after), Options::default());
        let fields: Vec<&str> = added.changes.iter().find(|change| change.name == "OffersPrice").map(|change| change.rows.iter().map(|row| row.label.as_str()).collect()).unwrap_or_default();
        assert_eq!(fields, vec!["Type", "Value", "Stat Type", "Modifier Type", "Value"]);
        let offers = comparison.parameters.iter().find(|parameter| parameter.name == "OffersPrice");
        assert_eq!(offers.map(|offers| (offers.state, offers.value.clone())), Some((State::Changed, None)));
    }

    #[test]
    fn a_picked_graph_variable_points_at_the_nodes_that_use_it() {
        let int = json!({"$content": 3, "$type": "System.Int32"});
        let both = with_variables(graph(0.8, false, false), &[("Speed", int, "System.Int32")]);
        let comparison = compare_script_graphs(Some(&both), Some(&both), Options::default());
        let speed = comparison.parameters.iter().position(|parameter| parameter.name == "Speed").unwrap_or(usize::MAX);
        let users: Vec<&str> = comparison.users_of(speed).map(|ix| comparison.nodes[ix].id.as_str()).collect();
        assert_eq!(users, vec!["u-get-speed"]);
    }

    #[test]
    fn events_have_a_flow_output_and_values_get_their_type() {
        let comparison = compare_script_graphs(None, Some(&graph(0.8, true, false)), Options::default());
        let update = comparison.nodes.iter().find(|node| node.title == "On Update");
        assert_eq!(update.map(|node| node.kind), Some(NodeKind::Event));
        let cooldown = comparison.nodes.iter().find(|node| node.title == "Cooldown");
        let duration = cooldown.and_then(|node| node.inputs.iter().find(|port| port.key == "duration"));
        assert_eq!(duration.map(|port| port.kind), Some(PortType::Float));
        assert_eq!(duration.and_then(|port| port.literal.clone()), Some("0.8".to_owned()));
    }
}
