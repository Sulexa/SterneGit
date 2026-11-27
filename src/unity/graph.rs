use std::collections::{HashMap, HashSet};

use super::{
    compare::{Options, Row, State, changed_rows},
    names::nicify,
    value::{Field, Value},
};

const PORT_ROW: f64 = 20.;
const UNIT_HEADER: f64 = 26.;

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

fn port_label(key: &str) -> String {
    match key {
        "enter" | "exit" | "trigger" | "%enter" => String::new(),
        "ifTrue" => "True".to_owned(),
        "ifFalse" => "False".to_owned(),
        "a" | "b" | "%a" | "%b" => key.trim_start_matches('%').to_uppercase(),
        _ => nicify(key.trim_start_matches('%')),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ScriptSide {
    pub(crate) units: Vec<(String, GraphNode, Vec<Field>)>,
    pub(crate) edges: Vec<GraphEdge>,
    pub(crate) variables: Vec<Declared>,
}

pub(crate) fn compare_script_graphs(old: &ScriptSide, new: &ScriptSide, ports: bool, options: Options) -> GraphComparison {
    let old_nodes: HashMap<&str, (&GraphNode, &Vec<Field>)> =
        old.units.iter().map(|(key, node, fields)| (key.as_str(), (node, fields))).collect();
    let new_nodes: HashMap<&str, &GraphNode> = new.units.iter().map(|(key, node, _)| (key.as_str(), node)).collect();
    let mut nodes = Vec::new();
    let mut changes = Vec::new();
    let mut hidden = 0;
    for (key, node, after_fields) in &new.units {
        let previous_entry = old_nodes.get(key.as_str()).copied();
        let previous = previous_entry.map(|(previous, _)| previous);
        let mut shown = node.clone();
        let before_fields = previous_entry.map(|(_, fields)| fields.as_slice());
        let (rows, noise) = changed_rows(before_fields, Some(after_fields), options);
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
    for (key, node, fields) in &old.units {
        if !new_nodes.contains_key(key.as_str()) {
            let mut shown = node.clone();
            shown.state = State::Removed;
            let (rows, _) = changed_rows(Some(fields), None, options);
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
    GraphComparison {
        nodes,
        edges,
        parameters,
        parameters_title: "Graph variables",
        changes: variable_changes,
        layers: Vec::new(),
        has_ports: ports,
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
    use super::{GraphComparison, NodeKind, PortType, ScriptSide, Shown, compare_script_graphs};
    use crate::unity::{
        compare::{Options, State},
        ingest::{script_side, tests::read_text},
        value::Value,
    };
    use serde_json::json;

    fn side(graph: Option<&serde_json::Value>) -> (ScriptSide, bool) {
        let Some(graph) = graph else {
            return (ScriptSide::default(), true);
        };
        let asset = format!(
            "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!114 &11400000\nMonoBehaviour:\n  m_Name: Flow\n  _data:\n    _json: '{}'\n",
            graph.to_string().replace('\'', "''")
        );
        let read = read_text("Flow.asset", &asset).graph.expect("the asset reads as a graph");
        (script_side(Some(&read)), read.ports)
    }

    fn compare_graphs(before: Option<&serde_json::Value>, after: Option<&serde_json::Value>, options: Options) -> GraphComparison {
        let (old, _) = side(before);
        let (new, ports) = side(after.or(before));
        let new = if after.is_some() { new } else { ScriptSide::default() };
        compare_script_graphs(&old, &new, ports, options)
    }

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
        let comparison = compare_graphs(Some(&before), Some(&after), Options::default());
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
        let comparison = compare_graphs(Some(&graph(0.8, false, false)), Some(&graph(0.8, true, false)), Options::default());
        let vector = comparison.changes.iter().find(|change| change.name == "Create Vector 2").map(|change| &change.rows);
        let y = vector.and_then(|rows| rows.iter().find(|row| row.label == "Y"));
        assert_eq!(y.and_then(|row| row.before.as_ref()).map(|value| value.text()), Some("6".to_owned()));
        assert_eq!(y.and_then(|row| row.after.as_ref()).map(|value| value.text()), Some("← Multiply (Product)".to_owned()));
    }

    #[test]
    fn the_version_before_reads_old_values_and_hides_what_was_added() {
        let comparison = compare_graphs(Some(&graph(0.8, false, true)), Some(&graph(0.6, true, false)), Options::default());
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
        let comparison = compare_graphs(Some(&before), Some(&after), Options::default());
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
        let comparison = compare_graphs(Some(&before), Some(&after), Options::default());
        let change = comparison.changes.iter().find(|change| change.name == "OffersPrice");
        assert_eq!(change.map(|change| (change.state, change.detail.as_str())), Some((State::Changed, "Value")));
        let rows: Vec<(usize, &str, Option<String>, Option<String>)> = change
            .map(|change| change.rows.iter().map(|row| (row.depth, row.label.as_str(), row.before.as_ref().map(Value::text), row.after.as_ref().map(Value::text))).collect())
            .unwrap_or_default();
        assert_eq!(rows, vec![(0, "Value", Some(String::new()), Some(String::new())), (1, "Value", Some("-20".to_owned()), Some("-15".to_owned()))]);
        let added = compare_graphs(None, Some(&after), Options::default());
        let fields: Vec<&str> = added.changes.iter().find(|change| change.name == "OffersPrice").map(|change| change.rows.iter().map(|row| row.label.as_str()).collect()).unwrap_or_default();
        assert_eq!(fields, vec!["Type", "Value", "Stat Type", "Modifier Type", "Value"]);
        let offers = comparison.parameters.iter().find(|parameter| parameter.name == "OffersPrice");
        assert_eq!(offers.map(|offers| (offers.state, offers.value.clone())), Some((State::Changed, None)));
    }

    #[test]
    fn a_picked_graph_variable_points_at_the_nodes_that_use_it() {
        let int = json!({"$content": 3, "$type": "System.Int32"});
        let both = with_variables(graph(0.8, false, false), &[("Speed", int, "System.Int32")]);
        let comparison = compare_graphs(Some(&both), Some(&both), Options::default());
        let speed = comparison.parameters.iter().position(|parameter| parameter.name == "Speed").unwrap_or(usize::MAX);
        let users: Vec<&str> = comparison.users_of(speed).map(|ix| comparison.nodes[ix].id.as_str()).collect();
        assert_eq!(users, vec!["u-get-speed"]);
    }

    #[test]
    fn events_have_a_flow_output_and_values_get_their_type() {
        let comparison = compare_graphs(None, Some(&graph(0.8, true, false)), Options::default());
        let update = comparison.nodes.iter().find(|node| node.title == "On Update");
        assert_eq!(update.map(|node| node.kind), Some(NodeKind::Event));
        let cooldown = comparison.nodes.iter().find(|node| node.title == "Cooldown");
        let duration = cooldown.and_then(|node| node.inputs.iter().find(|port| port.key == "duration"));
        assert_eq!(duration.map(|port| port.kind), Some(PortType::Float));
        assert_eq!(duration.and_then(|port| port.literal.clone()), Some("0.8".to_owned()));
    }
}
