use std::collections::HashSet;

use sternegit_reader_sdk::{Field, Icon, Model, Object, Part, Reference, Role, Value};

use crate::{
    scene::{self, Item, Key},
    value::{self, Glyph},
};

pub(crate) const NOISE: &str = "noise";

const NOISE_FIELDS: [&str; 6] = [
    "serializedVersion",
    "m_RootOrder",
    "m_LocalEulerAnglesHint",
    "m_ConstrainProportionsScale",
    "m_EditorClassIdentifier",
    "m_ModificationsOrder",
];

pub(crate) fn model(unity: &scene::Model) -> Model {
    let mut objects = Vec::with_capacity(unity.items.len());
    let mut placed = HashSet::new();
    let mut stack: Vec<Key> = unity.roots.iter().rev().copied().collect();
    while let Some(key) = stack.pop() {
        if !placed.insert(key) {
            continue;
        }
        if let Some(item) = unity.item(key) {
            objects.push(object(item));
        }
        if let Some(children) = unity.children.get(&key) {
            stack.extend(children.iter().rev().copied());
        }
    }
    objects.extend(unity.items.iter().filter(|item| !placed.contains(&item.key)).map(object));
    Model {
        title: None,
        objects,
        scopes: Vec::new(),
    }
}

fn object(item: &Item) -> Object {
    Object {
        id: item.key.to_string(),
        parent: item.parent.map(|parent| parent.to_string()),
        name: item.name.clone(),
        role: match &item.role {
            scene::Role::GameObject { root_of } => Role::Node {
                root_of: root_of.clone(),
            },
            scene::Role::Instance { source } => Role::Instance { source: source.clone() },
            scene::Role::Asset { type_name, detail } => Role::Asset {
                type_name: type_name.clone(),
                detail: detail.clone(),
            },
        },
        parts: item.parts.iter().map(part).collect(),
        tags: Vec::new(),
    }
}

fn part(part: &scene::Part) -> Part {
    Part {
        id: part.key.clone(),
        title: part.title.clone(),
        script: part.script.clone(),
        overrides: part.overrides,
        fields: tree(&part.fields),
    }
}

fn tree(flat: &[value::Field]) -> Vec<Field> {
    let mut roots: Vec<Field> = Vec::new();
    let mut open: Vec<(usize, String)> = Vec::new();
    for flat_field in flat {
        while open.last().is_some_and(|(depth, _)| *depth >= flat_field.depth) {
            open.pop();
        }
        let parent_path = open.last().map_or("", |(_, path)| path.as_str());
        let key = flat_field
            .path
            .strip_prefix(parent_path)
            .filter(|_| !parent_path.is_empty())
            .map_or(flat_field.path.as_str(), |rest| rest.strip_prefix('.').unwrap_or(rest));
        let field = Field {
            key: key.to_owned(),
            label: flat_field.label.clone(),
            value: value(&flat_field.value),
            tags: if is_noise_path(&flat_field.path) { vec![NOISE.to_owned()] } else { Vec::new() },
            children: Vec::new(),
        };
        let mut siblings = &mut roots;
        for _ in 0..open.len() {
            siblings = &mut siblings.last_mut().expect("an open field is in the tree").children;
        }
        siblings.push(field);
        open.push((flat_field.depth, flat_field.path.clone()));
    }
    roots
}

fn is_noise_path(path: &str) -> bool {
    let last = path.rsplit(['.', ':']).next().unwrap_or(path);
    NOISE_FIELDS.contains(&last) || NOISE_FIELDS.iter().any(|noise| path.starts_with(noise))
}

fn value(value: &value::Value) -> Value {
    match value {
        value::Value::Text(text) if text.is_empty() => Value::Empty,
        value::Value::Text(text) => Value::Text(text.clone()),
        value::Value::Long { start, more, digest } => Value::Long {
            start: start.clone(),
            more: *more,
            digest: digest.value(),
        },
        value::Value::Data { bytes, digest } => Value::Data {
            bytes: *bytes,
            digest: digest.value(),
        },
        value::Value::Number(text) => Value::Number(text.clone()),
        value::Value::Flag(on) => Value::Flag(*on),
        value::Value::Vector(parts) => {
            Value::Vector(parts.iter().map(|(axis, text)| ((*axis).to_owned(), text.clone())).collect())
        }
        value::Value::Color { rgba, hdr } => Value::Color { rgba: *rgba, hdr: *hdr },
        value::Value::Reference(reference) => Value::Reference(Reference {
            name: reference.name.clone(),
            kind: reference.kind.clone(),
            icon: icon(reference.glyph),
            target: reference.guid.clone(),
            missing: reference.missing,
        }),
        value::Value::Nothing(what) => Value::Nothing(what.clone()),
        value::Value::Count(count) => Value::Count(*count),
    }
}

fn icon(glyph: Glyph) -> Icon {
    match glyph {
        Glyph::Object => Icon::Object,
        Glyph::Asset => Icon::Asset,
        Glyph::Script => Icon::Script,
        Glyph::Image => Icon::Image,
        Glyph::Sound => Icon::Sound,
        Glyph::Material => Icon::Material,
        Glyph::Prefab => Icon::Prefab,
        Glyph::Clip => Icon::Clip,
    }
}

#[cfg(test)]
mod tests {
    use sternegit_reader_sdk::Field;

    use super::tree;
    use crate::value::{self, Value};

    fn flat(path: &str, depth: usize) -> value::Field {
        value::Field {
            path: path.into(),
            label: path.into(),
            depth,
            value: Value::Text(String::new()),
        }
    }

    fn paths(fields: &[Field], parent: &str, depth: usize, out: &mut Vec<(String, usize)>) {
        for field in fields {
            let path = field.path_under(parent);
            out.push((path.clone(), depth));
            paths(&field.children, &path, depth + 1, out);
        }
    }

    #[test]
    fn the_tree_gives_back_every_path_at_its_depth() {
        let flat_fields = [
            flat("m_Enabled", 0),
            flat("EmissionModule", 0),
            flat("EmissionModule.m_Bursts", 1),
            flat("EmissionModule.m_Bursts[0]", 2),
            flat("EmissionModule.m_Bursts[0].time", 3),
            flat("EmissionModule.m_Bursts[0].countCurve", 3),
            flat("EmissionModule.m_Bursts[0].countCurve.scalar", 4),
            flat("EmissionModule.rateOverTime", 1),
            flat("m_Speed", 0),
        ];
        let mut found = Vec::new();
        paths(&tree(&flat_fields), "", 0, &mut found);
        let expected: Vec<(String, usize)> =
            flat_fields.iter().map(|field| (field.path.clone(), field.depth)).collect();
        assert_eq!(found, expected);
    }
}
