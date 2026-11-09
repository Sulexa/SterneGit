use std::collections::{HashMap, HashSet};

use super::{
    scene::{Item, Key, Model, Part, Role},
    value::{Field, Sameness, Value},
};

const NOISE_FIELDS: [&str; 6] = [
    "serializedVersion",
    "m_RootOrder",
    "m_LocalEulerAnglesHint",
    "m_ConstrainProportionsScale",
    "m_EditorClassIdentifier",
    "m_ModificationsOrder",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Options {
    pub(crate) unchanged: bool,
    pub(crate) hide_noise: bool,
    pub(crate) file_ids: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            unchanged: false,
            hide_noise: true,
            file_ids: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Added,
    Changed,
    Removed,
    Same,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub(crate) label: String,
    pub(crate) depth: usize,
    pub(crate) before: Option<Value>,
    pub(crate) after: Option<Value>,
    pub(crate) state: State,
}

impl Row {
    fn is_container(&self) -> bool {
        let container = |value: &Option<Value>| match value {
            Some(Value::Text(text)) => text.is_empty(),
            Some(Value::Count(_)) | None => true,
            _ => false,
        };
        container(&self.before) && container(&self.after)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartChange {
    pub(crate) title: String,
    pub(crate) script: Option<String>,
    pub(crate) overrides: bool,
    pub(crate) state: State,
    pub(crate) rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ObjectChange {
    pub(crate) key: Key,
    pub(crate) name: String,
    pub(crate) parent: Option<String>,
    pub(crate) what: Option<String>,
    pub(crate) prefab: bool,
    pub(crate) state: State,
    pub(crate) parts: Vec<PartChange>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TreeRow {
    pub(crate) key: Key,
    pub(crate) name: String,
    pub(crate) depth: usize,
    pub(crate) state: State,
    pub(crate) prefab: bool,
    pub(crate) has_children: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Anchor {
    pub(crate) object: usize,
    pub(crate) part: Option<usize>,
    pub(crate) row: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Comparison {
    pub(crate) tree: Vec<TreeRow>,
    pub(crate) objects: Vec<ObjectChange>,
    pub(crate) hidden: usize,
    pub(crate) anchors: Vec<Anchor>,
}

impl Comparison {
    pub(crate) fn changed_objects(&self) -> usize {
        self.objects.len()
    }

    pub(crate) fn state_of(&self, key: Key) -> State {
        self.objects
            .iter()
            .find(|object| object.key == key)
            .map_or(State::Same, |object| object.state)
    }
}

pub(crate) fn compare(before: Option<&Model>, after: Option<&Model>, options: Options) -> Comparison {
    let empty = Model::default();
    let before = before.unwrap_or(&empty);
    let after = after.unwrap_or(&empty);
    let mut hidden = 0;
    let order = merged_order(before, after);
    let mut objects = Vec::new();
    for key in &order {
        let old = before.item(*key);
        let new = after.item(*key);
        if let Some(object) = compare_item(old, new, options, &mut hidden) {
            objects.push(object);
        }
    }
    let states: HashMap<Key, State> = objects.iter().map(|object| (object.key, object.state)).collect();
    let tree = if before.roots.is_empty() && after.roots.is_empty() {
        Vec::new()
    } else {
        merged_tree(before, after, &states)
    };
    let anchors = anchors(&objects);
    Comparison {
        tree,
        objects,
        hidden,
        anchors,
    }
}

fn anchors(objects: &[ObjectChange]) -> Vec<Anchor> {
    let mut anchors = Vec::new();
    for (object_ix, object) in objects.iter().enumerate() {
        if object.state != State::Changed {
            anchors.push(Anchor {
                object: object_ix,
                part: None,
                row: None,
            });
            continue;
        }
        for (part_ix, part) in object.parts.iter().enumerate() {
            if matches!(part.state, State::Added | State::Removed) {
                anchors.push(Anchor {
                    object: object_ix,
                    part: Some(part_ix),
                    row: None,
                });
                continue;
            }
            for (row_ix, row) in part.rows.iter().enumerate() {
                if row.state != State::Same && !row.is_container() {
                    anchors.push(Anchor {
                        object: object_ix,
                        part: Some(part_ix),
                        row: Some(row_ix),
                    });
                }
            }
        }
    }
    anchors
}

fn merged_order(before: &Model, after: &Model) -> Vec<Key> {
    let after_keys: Vec<Key> = after.items.iter().map(|item| item.key).collect();
    let before_keys: Vec<Key> = before.items.iter().map(|item| item.key).collect();
    merge_keys(&before_keys, &after_keys)
}

fn merge_keys<T: Copy + Eq + std::hash::Hash>(before: &[T], after: &[T]) -> Vec<T> {
    let in_after: HashSet<T> = after.iter().copied().collect();
    let position: HashMap<T, usize> = after.iter().enumerate().map(|(ix, key)| (*key, ix)).collect();
    let mut out = Vec::with_capacity(before.len().max(after.len()));
    let mut next = 0;
    for key in before {
        if in_after.contains(key) {
            let at = position[key];
            while next <= at {
                out.push(after[next]);
                next += 1;
            }
        } else {
            out.push(*key);
        }
    }
    out.extend(after[next.min(after.len())..].iter().copied());
    let mut seen = HashSet::new();
    out.retain(|key| seen.insert(*key));
    out
}

fn what(item: &Item, options: Options) -> Option<String> {
    let base = match &item.role {
        Role::GameObject { root_of } => root_of.as_ref().map(|prefab| format!("Root of {prefab}")),
        Role::Instance { .. } => item
            .parts
            .first()
            .and_then(|part| part.fields.first())
            .map(|field| match &field.value {
                Value::Reference(reference) => format!("Instance of {}.prefab", reference.name),
                _ => "Prefab instance".to_owned(),
            }),
        Role::Asset { type_name, detail } => Some(match detail {
            Some(detail) => format!("{type_name} · {detail}"),
            None => type_name.clone(),
        }),
    };
    if options.file_ids {
        Some(match base {
            Some(base) => format!("{base} · fileID {}", item.key),
            None => format!("fileID {}", item.key),
        })
    } else {
        base
    }
}

fn compare_item(before: Option<&Item>, after: Option<&Item>, options: Options, hidden: &mut usize) -> Option<ObjectChange> {
    let shown = after.or(before)?;
    let state = match (before, after) {
        (None, Some(_)) => State::Added,
        (Some(_), None) => State::Removed,
        _ => State::Same,
    };
    let empty: Vec<Part> = Vec::new();
    let old_parts = before.map_or(&empty, |item| &item.parts);
    let new_parts = after.map_or(&empty, |item| &item.parts);
    let order = merge_part_keys(old_parts, new_parts);
    let mut parts = Vec::new();
    for key in order {
        let old = old_parts.iter().find(|part| part.key == key);
        let new = new_parts.iter().find(|part| part.key == key);
        if let Some(part) = compare_part(old, new, options, state, hidden) {
            parts.push(part);
        }
    }
    let changed = state != State::Same || parts.iter().any(|part| part.state != State::Same);
    if !changed {
        return None;
    }
    let mut parts = parts;
    if state == State::Same {
        parts.retain(|part| part.state != State::Same || options.unchanged);
    }
    let prefab = matches!(shown.role, Role::Instance { .. });
    let parent = shown.parts.first().and_then(|part| part.fields.iter().find(|field| field.path == "parent")).and_then(|field| match &field.value {
        Value::Text(name) if name != "None" && !name.is_empty() => Some(name.clone()),
        _ => None,
    });
    Some(ObjectChange {
        key: shown.key,
        name: shown.name.clone(),
        parent,
        what: what(shown, options),
        prefab,
        state: if state == State::Same { State::Changed } else { state },
        parts,
    })
}

fn merge_part_keys(before: &[Part], after: &[Part]) -> Vec<String> {
    let old: Vec<&str> = before.iter().map(|part| part.key.as_str()).collect();
    let new: Vec<&str> = after.iter().map(|part| part.key.as_str()).collect();
    merge_keys(&old, &new).into_iter().map(ToOwned::to_owned).collect()
}

fn compare_part(
    before: Option<&Part>,
    after: Option<&Part>,
    options: Options,
    object: State,
    hidden: &mut usize,
) -> Option<PartChange> {
    let shown = after.or(before)?;
    let empty: Vec<Field> = Vec::new();
    let old_fields = before.map_or(&empty, |part| &part.fields);
    let new_fields = after.map_or(&empty, |part| &part.fields);
    let state = match (before, after) {
        (None, Some(_)) => State::Added,
        (Some(_), None) => State::Removed,
        _ => State::Same,
    };
    let mut uncounted = 0;
    let counted = if before.is_some() && after.is_some() { hidden } else { &mut uncounted };
    let mut rows = rows(old_fields, new_fields, options, counted);
    let changed = rows.iter().any(|row| row.state != State::Same && !row.is_container())
        || rows.iter().any(|row| row.state != State::Same && row.depth == 0);
    let state = match state {
        State::Same if changed => State::Changed,
        state => state,
    };
    if object == State::Same && state == State::Same && !options.unchanged {
        return Some(PartChange {
            title: shown.title.clone(),
            script: shown.script.clone(),
            overrides: shown.overrides,
            state,
            rows: Vec::new(),
        });
    }
    if !options.unchanged && state != State::Added && state != State::Removed {
        rows = only_changes(rows);
    }
    let title = if options.file_ids && shown.key.chars().all(|c| c.is_ascii_digit() || c == '-') {
        format!("{} · fileID {}", shown.title, shown.key)
    } else {
        shown.title.clone()
    };
    Some(PartChange {
        title,
        script: shown.script.clone(),
        overrides: shown.overrides,
        state,
        rows,
    })
}

pub(crate) fn changed_rows(before: Option<&[Field]>, after: Option<&[Field]>, options: Options) -> (Vec<Row>, usize) {
    let mut hidden = 0;
    let rows = rows(before.unwrap_or_default(), after.unwrap_or_default(), options, &mut hidden);
    let both = before.is_some() && after.is_some();
    (if options.unchanged || !both { rows } else { only_changes(rows) }, hidden)
}

fn is_noise_path(path: &str) -> bool {
    let last = path.rsplit(['.', ':']).next().unwrap_or(path);
    NOISE_FIELDS.contains(&last) || NOISE_FIELDS.iter().any(|noise| path.starts_with(noise))
}

fn rows(before: &[Field], after: &[Field], options: Options, hidden: &mut usize) -> Vec<Row> {
    let old: HashMap<&str, &Field> = before.iter().map(|field| (field.path.as_str(), field)).collect();
    let new: HashMap<&str, &Field> = after.iter().map(|field| (field.path.as_str(), field)).collect();
    let order = merge_keys(
        &before.iter().map(|field| field.path.as_str()).collect::<Vec<_>>(),
        &after.iter().map(|field| field.path.as_str()).collect::<Vec<_>>(),
    );
    let mut out = Vec::with_capacity(order.len());
    for path in order {
        let old = old.get(path).copied();
        let new = new.get(path).copied();
        let Some(shown) = new.or(old) else {
            continue;
        };
        let state = match (old, new) {
            (None, Some(_)) => State::Added,
            (Some(_), None) => State::Removed,
            (Some(old), Some(new)) => match old.value.same_as(&new.value) {
                Sameness::Equal => State::Same,
                Sameness::Rounding if options.hide_noise => {
                    *hidden += 1;
                    State::Same
                }
                _ if options.hide_noise && is_noise_path(path) => {
                    *hidden += 1;
                    State::Same
                }
                _ => State::Changed,
            },
            (None, None) => continue,
        };
        if options.hide_noise && is_noise_path(path) && matches!(state, State::Added | State::Removed) && old.is_some() != new.is_some() {
            *hidden += 1;
            continue;
        }
        out.push(Row {
            label: shown.label.clone(),
            depth: shown.depth,
            before: old.map(|field| field.value.clone()),
            after: new.map(|field| field.value.clone()),
            state,
        });
    }
    out
}

fn only_changes(rows: Vec<Row>) -> Vec<Row> {
    let mut keep: Vec<bool> = rows
        .iter()
        .map(|row| row.state != State::Same && (!row.is_container() || row.state != State::Changed || row.before != row.after))
        .collect();
    for ix in 0..rows.len() {
        if !keep[ix] {
            continue;
        }
        let mut depth = rows[ix].depth;
        let mut at = ix;
        while depth > 0 && at > 0 {
            at -= 1;
            if rows[at].depth < depth {
                keep[at] = true;
                depth = rows[at].depth;
            }
        }
    }
    rows.into_iter().zip(keep).filter_map(|(row, keep)| keep.then_some(row)).collect()
}

fn merged_tree(before: &Model, after: &Model, states: &HashMap<Key, State>) -> Vec<TreeRow> {
    let mut out = Vec::new();
    let roots = merge_keys(&before.roots, &after.roots);
    let mut visited = HashSet::new();
    let mut stack: Vec<(Key, usize)> = roots.iter().rev().map(|key| (*key, 0)).collect();
    while let Some((key, depth)) = stack.pop() {
        if !visited.insert(key) {
            continue;
        }
        let empty = Vec::new();
        let kids = merge_keys(
            before.children.get(&key).unwrap_or(&empty),
            after.children.get(&key).unwrap_or(&empty),
        );
        let Some(item) = after.item(key).or_else(|| before.item(key)) else {
            continue;
        };
        out.push(TreeRow {
            key,
            name: item.name.clone(),
            depth,
            state: states.get(&key).copied().unwrap_or(State::Same),
            prefab: matches!(item.role, Role::Instance { .. }) || matches!(&item.role, Role::GameObject { root_of: Some(_) }),
            has_children: !kids.is_empty(),
        });
        stack.extend(kids.iter().rev().map(|kid| (*kid, depth + 1)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Options, State, compare, merge_keys};
    use crate::unity::{
        names::UnityKind,
        scene::{Model, object_names},
        value::Lookup,
        yaml::documents,
    };

    fn model(text: &str) -> Model {
        let docs = documents(text);
        let mut lookup = Lookup::default();
        lookup.objects = object_names(&docs, &lookup.assets);
        Model::from_documents(UnityKind::Prefab, &docs, "Assets/Slime.prefab", &lookup)
    }

    fn prefab(mass: &str, shadow_scale: &str, with_crown: bool, with_particles: bool) -> String {
        let mut text = format!(
            "%YAML 1.1
%TAG !u! tag:unity3d.com,2011:
--- !u!1 &100
GameObject:
  m_Component:
  - component: {{fileID: 400}}
  - component: {{fileID: 500}}
  m_Name: Slime
--- !u!4 &400
Transform:
  m_GameObject: {{fileID: 100}}
  m_LocalPosition: {{x: 0, y: 0, z: 0}}
  m_Children:
  - {{fileID: 420}}
{}{}  m_Father: {{fileID: 0}}
--- !u!50 &500
Rigidbody2D:
  m_GameObject: {{fileID: 100}}
  m_Mass: {mass}
  m_GravityScale: 3
--- !u!1 &120
GameObject:
  m_Component:
  - component: {{fileID: 420}}
  m_Name: Shadow
--- !u!4 &420
Transform:
  m_GameObject: {{fileID: 120}}
  m_LocalScale: {{x: 1, y: {shadow_scale}, z: 1}}
  m_Father: {{fileID: 400}}
",
            if with_crown { "  - {fileID: 410}\n" } else { "" },
            if with_particles { "  - {fileID: 430}\n" } else { "" },
        );
        if with_crown {
            text.push_str(
                "--- !u!1 &110
GameObject:
  m_Component:
  - component: {fileID: 410}
  m_Name: Crown
--- !u!4 &410
Transform:
  m_GameObject: {fileID: 110}
  m_LocalPosition: {x: 0, y: 0.62, z: 0}
  m_Father: {fileID: 400}
",
            );
        }
        if with_particles {
            text.push_str(
                "--- !u!1 &130
GameObject:
  m_Component:
  - component: {fileID: 430}
  m_Name: SquishParticles
--- !u!4 &430
Transform:
  m_GameObject: {fileID: 130}
  m_Father: {fileID: 400}
",
            );
        }
        text
    }

    #[test]
    fn only_what_changed_is_listed_with_each_object_in_hierarchy_order() {
        let before = model(&prefab("1", "0.3", false, true));
        let after = model(&prefab("1.4", "0.25", true, false));
        let comparison = compare(Some(&before), Some(&after), Options::default());
        let objects: Vec<(&str, State)> = comparison.objects.iter().map(|object| (object.name.as_str(), object.state)).collect();
        assert_eq!(
            objects,
            vec![
                ("Slime", State::Changed),
                ("Shadow", State::Changed),
                ("SquishParticles", State::Removed),
                ("Crown", State::Added),
            ]
        );
        let slime = &comparison.objects[0];
        assert_eq!(slime.parts.len(), 1);
        assert_eq!(slime.parts[0].title, "Rigidbody 2D");
        assert_eq!(slime.parts[0].rows.len(), 1);
        assert_eq!(slime.parts[0].rows[0].label, "Mass");
    }

    #[test]
    fn the_tree_merges_both_versions_and_marks_each_object() {
        let before = model(&prefab("1", "0.3", false, true));
        let after = model(&prefab("1", "0.3", true, false));
        let comparison = compare(Some(&before), Some(&after), Options::default());
        let tree: Vec<(&str, usize, State)> = comparison.tree.iter().map(|row| (row.name.as_str(), row.depth, row.state)).collect();
        assert_eq!(
            tree,
            vec![
                ("Slime", 0, State::Same),
                ("Shadow", 1, State::Same),
                ("SquishParticles", 1, State::Removed),
                ("Crown", 1, State::Added),
            ]
        );
    }

    #[test]
    fn a_float_that_only_rounds_differently_is_hidden_and_counted() {
        let before = model(&prefab("1", "0.3", false, false));
        let after = model(&prefab("0.99999994", "0.3", false, false));
        let hiding = compare(Some(&before), Some(&after), Options::default());
        assert!(hiding.objects.is_empty());
        assert_eq!(hiding.hidden, 1);
        let showing = compare(Some(&before), Some(&after), Options { hide_noise: false, ..Options::default() });
        assert_eq!(showing.objects.len(), 1);
    }

    #[test]
    fn showing_unchanged_properties_lists_the_whole_component() {
        let before = model(&prefab("1", "0.3", false, false));
        let after = model(&prefab("2", "0.3", false, false));
        let all = compare(Some(&before), Some(&after), Options { unchanged: true, ..Options::default() });
        let body = all.objects[0].parts.iter().find(|part| part.title == "Rigidbody 2D").map(|part| part.rows.len());
        assert_eq!(body, Some(2));
    }

    #[test]
    fn keys_keep_the_new_order_and_put_removed_ones_where_they_were() {
        assert_eq!(merge_keys(&[1, 2, 3, 4], &[1, 3, 5, 4]), vec![1, 2, 3, 5, 4]);
        assert_eq!(merge_keys::<i32>(&[], &[7, 8]), vec![7, 8]);
        assert_eq!(merge_keys(&[7, 8], &[]), vec![7, 8]);
    }

    #[test]
    fn each_change_has_an_anchor_to_step_to() {
        let before = model(&prefab("1", "0.3", false, true));
        let after = model(&prefab("1.4", "0.25", true, false));
        let comparison = compare(Some(&before), Some(&after), Options::default());
        assert_eq!(comparison.anchors.len(), 4);
    }
}
