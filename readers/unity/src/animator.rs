use std::collections::HashMap;

use super::{
    graph::{Declared, GraphEdge, GraphNode, NodeKind, PortType},
    value::{Field, Lookup, Value, fields, short_number},
    yaml::{Document, Node},
};

const STATE_SIZE: (f64, f64) = (200., 40.);
const STATE_FIELDS_SKIPPED: [&str; 10] = [
    "m_ObjectHideFlags",
    "m_CorrespondingSourceObject",
    "m_PrefabInstance",
    "m_PrefabAsset",
    "m_Transitions",
    "m_StateMachineBehaviours",
    "m_Position",
    "serializedVersion",
    "m_IKOnFeet",
    "m_Name",
];
const TRANSITION_FIELDS_SKIPPED: [&str; 9] = [
    "m_ObjectHideFlags",
    "m_CorrespondingSourceObject",
    "m_PrefabInstance",
    "m_PrefabAsset",
    "m_Conditions",
    "m_DstState",
    "m_DstStateMachine",
    "serializedVersion",
    "m_Name",
];
const ENTRY: &str = "entry";
const ANY: &str = "any";
const EXIT: &str = "exit";

struct Machine<'a> {
    docs: HashMap<i64, &'a Document>,
    lookup: &'a Lookup,
    parameter_types: HashMap<String, i64>,
}

#[derive(Clone)]
struct Drawn {
    nodes: Vec<(GraphNode, Vec<Field>)>,
    edges: Vec<(GraphEdge, Vec<Field>)>,
}

fn file_id(node: Option<&Node>) -> Option<i64> {
    node?.get("fileID")?.integer().filter(|&id| id != 0)
}

fn position(node: Option<&Node>) -> (f64, f64) {
    let part = |axis: &str| node.and_then(|node| node.get(axis)).and_then(Node::number).unwrap_or(0.);
    (part("x"), part("y"))
}

pub(crate) fn layers(documents: &[Document]) -> Vec<String> {
    controller(documents)
        .and_then(|body| body.get("m_AnimatorLayers"))
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .map(|layer| layer.get("m_Name").and_then(Node::text).unwrap_or("Layer").to_owned())
        .collect()
}

fn controller(documents: &[Document]) -> Option<&Node> {
    documents.iter().find(|document| document.type_name == "AnimatorController").map(|document| &document.body)
}

fn parameters(documents: &[Document]) -> Vec<(String, String, i64, String)> {
    controller(documents)
        .and_then(|body| body.get("m_AnimatorParameters"))
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .filter_map(|parameter| {
            let name = parameter.get("m_Name")?.text()?.to_owned();
            let kind = parameter.get("m_Type").and_then(Node::number).unwrap_or(1.) as i64;
            let default = match kind {
                1 => parameter.get("m_DefaultFloat").and_then(Node::number).map(short_number).unwrap_or_default(),
                3 => parameter.get("m_DefaultInt").and_then(Node::text).unwrap_or_default().to_owned(),
                4 => if parameter.get("m_DefaultBool").and_then(Node::text) == Some("1") { "true" } else { "false" }.to_owned(),
                _ => String::new(),
            };
            Some((name, type_name(kind).to_owned(), kind, default))
        })
        .collect()
}

fn type_name(kind: i64) -> &'static str {
    match kind {
        1 => "Float",
        3 => "Int",
        4 => "Bool",
        9 => "Trigger",
        _ => "Parameter",
    }
}

impl<'a> Machine<'a> {
    fn new(documents: &'a [Document], lookup: &'a Lookup) -> Self {
        Self {
            docs: documents.iter().map(|document| (document.file_id, document)).collect(),
            lookup,
            parameter_types: parameters(documents).into_iter().map(|(name, _, kind, _)| (name, kind)).collect(),
        }
    }

    fn condition(&self, condition: &Node) -> String {
        let parameter = condition.get("m_ConditionEvent").and_then(Node::text).unwrap_or_default();
        let mode = condition.get("m_ConditionMode").and_then(Node::number).unwrap_or(1.) as i64;
        let threshold = condition.get("m_EventTreshold").and_then(Node::number).map(short_number).unwrap_or_default();
        let trigger = self.parameter_types.get(parameter) == Some(&9);
        match mode {
            1 if trigger => parameter.to_owned(),
            1 => format!("{parameter} is true"),
            2 => format!("{parameter} is false"),
            3 => format!("{parameter} > {threshold}"),
            4 => format!("{parameter} < {threshold}"),
            6 => format!("{parameter} = {threshold}"),
            7 => format!("{parameter} ≠ {threshold}"),
            _ => parameter.to_owned(),
        }
    }

    fn transition_fields(&self, body: &Node) -> Vec<Field> {
        let conditions = body
            .get("m_Conditions")
            .map(Node::items)
            .unwrap_or_default()
            .iter()
            .map(|condition| self.condition(condition))
            .collect::<Vec<_>>()
            .join(", ");
        let mut out = vec![Field {
            path: "conditions".to_owned(),
            label: "Conditions".to_owned(),
            depth: 0,
            value: Value::Text(if conditions.is_empty() { "None".to_owned() } else { conditions }),
        }];
        out.extend(fields(body, &TRANSITION_FIELDS_SKIPPED, self.lookup));
        out
    }

    fn draw(&self, layer: usize) -> Drawn {
        let mut drawn = Drawn { nodes: Vec::new(), edges: Vec::new() };
        let Some(root) = self
            .docs
            .values()
            .find(|document| document.type_name == "AnimatorController")
            .and_then(|document| document.body.get("m_AnimatorLayers"))
            .and_then(|layers| layers.items().get(layer))
            .and_then(|layer| file_id(layer.get("m_StateMachine")))
            .and_then(|id| self.docs.get(&id))
        else {
            return drawn;
        };
        let machine = &root.body;
        let special = |id: &str, title: &str, kind: NodeKind, at: (f64, f64)| {
            (
                GraphNode {
                    id: id.to_owned(),
                    title: title.to_owned(),
                    kind,
                    at,
                    size: STATE_SIZE,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    variable: None,
                },
                Vec::new(),
            )
        };
        drawn.nodes.push(special(ENTRY, "Entry", NodeKind::Entry, position(machine.get("m_EntryPosition"))));
        drawn.nodes.push(special(ANY, "Any State", NodeKind::AnyState, position(machine.get("m_AnyStatePosition"))));
        drawn.nodes.push(special(EXIT, "Exit", NodeKind::Exit, position(machine.get("m_ExitPosition"))));
        let default = file_id(machine.get("m_DefaultState"));
        for child in machine.get("m_ChildStates").map(Node::items).unwrap_or_default() {
            let Some(state_id) = file_id(child.get("m_State")) else {
                continue;
            };
            let Some(state) = self.docs.get(&state_id) else {
                continue;
            };
            let name = state.body.get("m_Name").and_then(Node::text).unwrap_or("State").to_owned();
            drawn.nodes.push((
                GraphNode {
                    id: state_id.to_string(),
                    title: name,
                    kind: if Some(state_id) == default { NodeKind::DefaultState } else { NodeKind::State },
                    at: position(child.get("m_Position")),
                    size: STATE_SIZE,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    variable: None,
                },
                fields(&state.body, &STATE_FIELDS_SKIPPED, self.lookup),
            ));
            for transition in state.body.get("m_Transitions").map(Node::items).unwrap_or_default() {
                self.edge(&mut drawn, &state_id.to_string(), file_id(Some(transition)));
            }
        }
        for child in machine.get("m_ChildStateMachines").map(Node::items).unwrap_or_default() {
            let Some(id) = file_id(child.get("m_StateMachine")) else {
                continue;
            };
            let name = self
                .docs
                .get(&id)
                .and_then(|document| document.body.get("m_Name"))
                .and_then(Node::text)
                .unwrap_or("Sub-state machine")
                .to_owned();
            drawn.nodes.push((
                GraphNode {
                    id: id.to_string(),
                    title: name,
                    kind: NodeKind::Machine,
                    at: position(child.get("m_Position")),
                    size: STATE_SIZE,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    variable: None,
                },
                Vec::new(),
            ));
        }
        for transition in machine.get("m_AnyStateTransitions").map(Node::items).unwrap_or_default() {
            self.edge(&mut drawn, ANY, file_id(Some(transition)));
        }
        for transition in machine.get("m_EntryTransitions").map(Node::items).unwrap_or_default() {
            self.edge(&mut drawn, ENTRY, file_id(Some(transition)));
        }
        if let Some(default) = default {
            drawn.edges.push((
                GraphEdge {
                    id: "default".to_owned(),
                    from: ENTRY.to_owned(),
                    to: default.to_string(),
                    from_port: None,
                    to_port: None,
                    kind: PortType::Flow,
                },
                Vec::new(),
            ));
        }
        drawn
    }

    fn edge(&self, drawn: &mut Drawn, from: &str, transition: Option<i64>) {
        let Some(transition) = transition else {
            return;
        };
        let Some(document) = self.docs.get(&transition) else {
            return;
        };
        let body = &document.body;
        let to = if body.get("m_IsExit").and_then(Node::text) == Some("1") {
            EXIT.to_owned()
        } else {
            match file_id(body.get("m_DstState")).or_else(|| file_id(body.get("m_DstStateMachine"))) {
                Some(to) => to.to_string(),
                None => EXIT.to_owned(),
            }
        };
        drawn.edges.push((
            GraphEdge {
                id: transition.to_string(),
                from: from.to_owned(),
                to,
                from_port: None,
                to_port: None,
                kind: PortType::Flow,
            },
            self.transition_fields(body),
        ));
    }
}

pub(crate) type Drawing = (Vec<(GraphNode, Vec<Field>)>, Vec<(GraphEdge, Vec<Field>)>);

pub(crate) fn drawn_layers(documents: &[Document], lookup: &Lookup) -> Vec<(String, Drawing)> {
    let machine = Machine::new(documents, lookup);
    layers(documents)
        .into_iter()
        .enumerate()
        .map(|(ix, name)| {
            let drawn = machine.draw(ix);
            (name, (drawn.nodes, drawn.edges))
        })
        .collect()
}

pub(crate) fn declared(documents: &[Document]) -> Vec<Declared> {
    parameters(documents)
        .into_iter()
        .map(|(name, kind, _, value)| Declared {
            name,
            kind,
            value,
            members: Vec::new(),
        })
        .collect()
}
