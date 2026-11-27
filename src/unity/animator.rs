use std::collections::HashMap;

use super::{
    compare::{Options, State, changed_rows},
    graph::{ChangeTarget, Declared, GraphChange, GraphComparison, GraphEdge, GraphNode, NodeKind, ParameterLook, compare_parameters},
    value::{Field, Value},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Drawn {
    pub(crate) nodes: Vec<(GraphNode, Vec<Field>)>,
    pub(crate) edges: Vec<(GraphEdge, Vec<Field>)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Machine {
    pub(crate) layers: Vec<(String, Drawn)>,
    pub(crate) parameters: Vec<Declared>,
}

pub(crate) fn compare_controllers(before: &Machine, after: &Machine, layer: usize, options: Options) -> GraphComparison {
    let empty = Drawn::default();
    let drawn = |machine: &Machine| machine.layers.get(layer).map_or(empty.clone(), |(_, drawn)| drawn.clone());
    let old = drawn(before);
    let new = drawn(after);
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
        shown.was_at = previous.map(|entry| entry.0.at).filter(|at| *at != node.at);
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
    let look = ParameterLook {
        value_label: "Default",
        value_shown: false,
    };
    let (shown_parameters, mut parameter_changes) = compare_parameters(&before.parameters, &after.parameters, "Parameter", look, options);
    changes.append(&mut parameter_changes);
    let mut all_layers: Vec<String> = after.layers.iter().map(|(name, _)| name.clone()).collect();
    for (name, _) in &before.layers {
        if !all_layers.contains(name) {
            all_layers.push(name.clone());
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
    use super::{Machine, compare_controllers};
    use crate::unity::{
        compare::{Options, State},
        graph::NodeKind,
        ingest::{machine, tests::read_text},
    };

    fn read(text: &str) -> Machine {
        machine(read_text("Slime.controller", text).graph.as_ref())
    }

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
        let before = read(&controller(false, "0.75"));
        let after = read(&controller(true, "0.5"));
        let comparison = compare_controllers(&before, &after, 0, Options::default());
        let changes: Vec<(&str, &str, State)> = comparison.changes.iter().map(|change| (change.kind, change.name.as_str(), change.state)).collect();
        assert!(changes.contains(&("State", "Squish", State::Added)));
        assert!(changes.contains(&("Transition", "Idle → Hop", State::Changed)));
        assert_eq!(after.layers.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(), ["Base Layer"]);
        let idle = comparison.nodes.iter().find(|node| node.title == "Idle").map(|node| node.kind);
        assert_eq!(idle, Some(NodeKind::DefaultState));
        assert!(comparison.edges.iter().any(|edge| edge.from == "entry" && edge.id == "default"));
    }

    #[test]
    fn a_trigger_condition_reads_as_the_trigger_name() {
        let after = read(&controller(false, "0.75"));
        let comparison = compare_controllers(&Machine::default(), &after, 0, Options::default());
        let transition = comparison.changes.iter().find(|change| change.kind == "Transition");
        let conditions = transition.and_then(|change| change.rows.iter().find(|row| row.label == "Conditions"));
        assert_eq!(conditions.and_then(|row| row.after.as_ref()).map(|value| value.text()), Some("Jump".to_owned()));
    }

    #[test]
    fn a_parameter_default_that_changed_shows_before_and_after() {
        let with_speed = |default: &str| {
            controller(false, "0.75").replace(
                "  m_AnimatorLayers:",
                &format!("  - m_Name: Speed\n    m_Type: 1\n    m_DefaultFloat: {default}\n    m_DefaultInt: 0\n    m_DefaultBool: 0\n  m_AnimatorLayers:"),
            )
        };
        let comparison = compare_controllers(&read(&with_speed("1")), &read(&with_speed("2.5")), 0, Options::default());
        let speed = comparison.changes.iter().find(|change| change.name == "Speed");
        assert_eq!(speed.map(|change| (change.kind, change.state, change.detail.as_str())), Some(("Parameter", State::Changed, "Default")));
        let default = speed.and_then(|change| change.rows.iter().find(|row| row.label == "Default"));
        assert_eq!(default.and_then(|row| row.before.as_ref()).map(|value| value.text()), Some("1".to_owned()));
        assert_eq!(default.and_then(|row| row.after.as_ref()).map(|value| value.text()), Some("2.5".to_owned()));
    }
}
