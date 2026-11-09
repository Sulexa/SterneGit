use std::collections::HashMap;

use super::{
    compare::{Options, State, changed_rows},
    graph::{ChangeTarget, GraphChange, GraphComparison, GraphEdge, GraphNode, NodeKind, PortType},
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
    node?.get("fileID")?.number().map(|id| id as i64).filter(|&id| id != 0)
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
                    state: State::Same,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
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
                    state: State::Same,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
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
                    state: State::Same,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
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
                    state: State::Same,
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
                state: State::Same,
                kind: PortType::Flow,
            },
            self.transition_fields(body),
        ));
    }
}

pub(crate) fn compare_controllers(
    before: Option<&[Document]>,
    after: Option<&[Document]>,
    lookup: &Lookup,
    layer: usize,
    options: Options,
) -> GraphComparison {
    let none: [Document; 0] = [];
    let old_docs = before.unwrap_or(&none);
    let new_docs = after.unwrap_or(&none);
    let old = Machine::new(old_docs, lookup).draw(layer);
    let new = Machine::new(new_docs, lookup).draw(layer);
    let mut hidden = 0;
    let mut nodes = Vec::new();
    let mut changes = Vec::new();
    let old_nodes: HashMap<&str, &(GraphNode, Vec<Field>)> = old.nodes.iter().map(|entry| (entry.0.id.as_str(), entry)).collect();
    for (node, node_fields) in &new.nodes {
        let previous = old_nodes.get(node.id.as_str());
        let mut shown = node.clone();
        let (rows, noise) = changed_rows(previous.map(|entry| entry.1.as_slice()), Some(node_fields), options);
        hidden += noise;
        let renamed = previous.is_some_and(|entry| entry.0.title != node.title);
        shown.state = match previous {
            None => State::Added,
            Some(_) if renamed || rows.iter().any(|row| row.state != State::Same) => State::Changed,
            Some(_) => State::Same,
        };
        if shown.state != State::Same && !matches!(node.kind, NodeKind::Entry | NodeKind::AnyState | NodeKind::Exit) {
            changes.push(GraphChange {
                state: shown.state,
                kind: "State",
                name: node.title.clone(),
                detail: motion_of(node_fields),
                target: ChangeTarget::Node(nodes.len()),
                rows,
            });
        }
        nodes.push(shown);
    }
    for (node, node_fields) in &old.nodes {
        if !new.nodes.iter().any(|(new_node, _)| new_node.id == node.id) {
            let mut shown = node.clone();
            shown.state = State::Removed;
            let (rows, _) = changed_rows(Some(node_fields), None, options);
            changes.push(GraphChange {
                state: State::Removed,
                kind: "State",
                name: node.title.clone(),
                detail: motion_of(node_fields),
                target: ChangeTarget::Node(nodes.len()),
                rows,
            });
            nodes.push(shown);
        }
    }
    let title_of = |id: &str| {
        nodes
            .iter()
            .find(|node| node.id == id)
            .map_or_else(|| id.to_owned(), |node| node.title.clone())
    };
    let mut edges = Vec::new();
    let old_edges: HashMap<&str, &(GraphEdge, Vec<Field>)> = old.edges.iter().map(|entry| (entry.0.id.as_str(), entry)).collect();
    for (edge, edge_fields) in &new.edges {
        let previous = old_edges.get(edge.id.as_str());
        let mut shown = edge.clone();
        let (rows, noise) = changed_rows(previous.map(|entry| entry.1.as_slice()), Some(edge_fields), options);
        hidden += noise;
        let moved = previous.is_some_and(|entry| entry.0.from != edge.from || entry.0.to != edge.to);
        shown.state = match previous {
            None => State::Added,
            Some(_) if moved || rows.iter().any(|row| row.state != State::Same) => State::Changed,
            Some(_) => State::Same,
        };
        if shown.state != State::Same {
            changes.push(GraphChange {
                state: shown.state,
                kind: "Transition",
                name: format!("{} → {}", title_of(&edge.from), title_of(&edge.to)),
                detail: transition_detail(&rows, shown.state),
                target: ChangeTarget::Edge(edges.len()),
                rows,
            });
        }
        edges.push(shown);
    }
    for (edge, edge_fields) in &old.edges {
        if !new.edges.iter().any(|(new_edge, _)| new_edge.id == edge.id) {
            let mut shown = edge.clone();
            shown.state = State::Removed;
            let (rows, _) = changed_rows(Some(edge_fields), None, options);
            changes.push(GraphChange {
                state: State::Removed,
                kind: "Transition",
                name: format!("{} → {}", title_of(&edge.from), title_of(&edge.to)),
                detail: String::new(),
                target: ChangeTarget::Edge(edges.len()),
                rows,
            });
            edges.push(shown);
        }
    }
    let old_parameters = parameters(old_docs);
    let new_parameters = parameters(new_docs);
    let mut shown_parameters = Vec::new();
    for (name, kind, _, default) in &new_parameters {
        let state = match old_parameters.iter().find(|(old_name, ..)| old_name == name) {
            None => State::Added,
            Some((_, old_kind, _, old_default)) if old_kind == kind && old_default == default => State::Same,
            Some(_) => State::Changed,
        };
        if state != State::Same {
            changes.push(GraphChange {
                state,
                kind: "Parameter",
                name: name.clone(),
                detail: kind.clone(),
                target: ChangeTarget::Parameter,
                rows: Vec::new(),
            });
        }
        shown_parameters.push((name.clone(), kind.clone(), state));
    }
    for (name, kind, ..) in &old_parameters {
        if !new_parameters.iter().any(|(new_name, ..)| new_name == name) {
            changes.push(GraphChange {
                state: State::Removed,
                kind: "Parameter",
                name: name.clone(),
                detail: kind.clone(),
                target: ChangeTarget::Parameter,
                rows: Vec::new(),
            });
            shown_parameters.push((name.clone(), kind.clone(), State::Removed));
        }
    }
    let mut all_layers = layers(new_docs);
    for name in layers(old_docs) {
        if !all_layers.contains(&name) {
            all_layers.push(name);
        }
    }
    GraphComparison {
        nodes,
        edges,
        parameters: shown_parameters,
        parameters_title: "Parameters",
        changes,
        layers: all_layers,
        has_ports: false,
        hidden,
    }
}

fn motion_of(node_fields: &[Field]) -> String {
    node_fields
        .iter()
        .find(|field| field.path == "m_Motion")
        .map(|field| match &field.value {
            Value::Reference(reference) if reference.missing => reference.name.clone(),
            Value::Reference(reference) => format!("{}.anim", reference.name),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn transition_detail(rows: &[super::compare::Row], state: State) -> String {
    if state == State::Changed {
        let changed = rows.iter().filter(|row| row.state != State::Same).count();
        return format!("{changed} {}", if changed == 1 { "setting" } else { "settings" });
    }
    rows.iter()
        .find(|row| row.label == "Exit Time")
        .and_then(|row| row.after.as_ref())
        .map(|value| format!("Exit time {}", value.text()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{compare_controllers, layers};
    use crate::unity::{
        compare::{Options, State},
        graph::NodeKind,
        value::Lookup,
        yaml::documents,
    };

    fn controller(with_squish: bool, exit_time: &str) -> String {
        let squish_child = if with_squish {
            "  - serializedVersion: 1
    m_State: {fileID: 1102003}
    m_Position: {x: 300, y: 200, z: 0}
"
        } else {
            ""
        };
        let squish = if with_squish {
            "--- !u!1102 &1102003
AnimatorState:
  m_Name: Squish
  m_Speed: 1
  m_Transitions: []
  m_Motion: {fileID: 0}
"
        } else {
            ""
        };
        format!(
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!91 &9100000
AnimatorController:
  m_Name: Slime
  m_AnimatorParameters:
  - m_Name: Jump
    m_Type: 9
    m_DefaultFloat: 0
    m_DefaultInt: 0
    m_DefaultBool: 0
  m_AnimatorLayers:
  - serializedVersion: 5
    m_Name: Base Layer
    m_StateMachine: {{fileID: 1107000}}
--- !u!1107 &1107000
AnimatorStateMachine:
  m_Name: Base Layer
  m_ChildStates:
  - serializedVersion: 1
    m_State: {{fileID: 1102001}}
    m_Position: {{x: 200, y: 0, z: 0}}
  - serializedVersion: 1
    m_State: {{fileID: 1102002}}
    m_Position: {{x: 400, y: 0, z: 0}}
{squish_child}  m_AnyStateTransitions: []
  m_EntryTransitions: []
  m_AnyStatePosition: {{x: 50, y: 20, z: 0}}
  m_EntryPosition: {{x: 50, y: 120, z: 0}}
  m_ExitPosition: {{x: 800, y: 120, z: 0}}
  m_DefaultState: {{fileID: 1102001}}
--- !u!1102 &1102001
AnimatorState:
  m_Name: Idle
  m_Speed: 1
  m_Transitions:
  - {{fileID: 1101001}}
  m_Motion: {{fileID: 0}}
--- !u!1102 &1102002
AnimatorState:
  m_Name: Hop
  m_Speed: 1
  m_Transitions: []
  m_Motion: {{fileID: 0}}
--- !u!1101 &1101001
AnimatorStateTransition:
  m_Name:
  m_Conditions:
  - m_ConditionMode: 1
    m_ConditionEvent: Jump
    m_EventTreshold: 0
  m_DstState: {{fileID: 1102002}}
  m_HasExitTime: 1
  m_ExitTime: {exit_time}
{squish}"
        )
    }

    #[test]
    fn states_and_transitions_that_changed_are_listed() {
        let before = documents(&controller(false, "0.75"));
        let after = documents(&controller(true, "0.5"));
        let comparison = compare_controllers(Some(&before), Some(&after), &Lookup::default(), 0, Options::default());
        let changes: Vec<(&str, &str, State)> = comparison.changes.iter().map(|change| (change.kind, change.name.as_str(), change.state)).collect();
        assert!(changes.contains(&("State", "Squish", State::Added)));
        assert!(changes.contains(&("Transition", "Idle → Hop", State::Changed)));
        assert_eq!(layers(&after), vec!["Base Layer".to_owned()]);
        let idle = comparison.nodes.iter().find(|node| node.title == "Idle").map(|node| node.kind);
        assert_eq!(idle, Some(NodeKind::DefaultState));
        assert!(comparison.edges.iter().any(|edge| edge.from == "entry" && edge.id == "default"));
    }

    #[test]
    fn a_trigger_condition_reads_as_the_trigger_name() {
        let after = documents(&controller(false, "0.75"));
        let comparison = compare_controllers(None, Some(&after), &Lookup::default(), 0, Options::default());
        let transition = comparison.changes.iter().find(|change| change.kind == "Transition");
        let conditions = transition.and_then(|change| change.rows.iter().find(|row| row.label == "Conditions"));
        assert_eq!(conditions.and_then(|row| row.after.as_ref()).map(|value| value.text()), Some("Jump".to_owned()));
    }
}
