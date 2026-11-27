use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash as _, Hasher as _},
};

use sternegit_reader_protocol as protocol;

use super::{
    scene::{Item, Key, Model, Part, Role},
    value::{Digest, Field, Glyph, Reference, Value},
};

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

fn flatten(tree: &[protocol::Field], parent: &str, depth: usize, out: &mut Vec<Field>) {
    for field in tree {
        let path = field.path_under(parent);
        out.push(Field {
            path: path.clone(),
            label: field.label.clone(),
            depth,
            value: value(&field.value),
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

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        ffi::OsString,
        path::Path,
        sync::{Arc, OnceLock},
    };

    use crate::{
        git::{Revision, scratch::ScratchRepo},
        read::Reader,
        unity::{ByReader, Source, read},
    };

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
    fn the_unity_reader_gives_the_same_objects_as_reading_in_sternegit() -> anyhow::Result<()> {
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
            let native = read(&repo, path, None, source(), None)?;
            let through_reader = read(&repo, path, None, source(), Some(&by))?;
            let model = |files: &crate::unity::UnityFiles| {
                let after = files.after.as_ref().unwrap();
                (after.model.items.clone(), after.model.roots.clone(), after.model.children.clone())
            };
            assert_eq!(model(&through_reader), model(&native), "{path}");
        }
        Ok(())
    }
}
