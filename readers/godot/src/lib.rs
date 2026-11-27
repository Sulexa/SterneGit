mod text;
mod values;

use std::collections::HashMap;

use sternegit_reader_sdk::{Failure, Model, Note, Object, Part, ReadInput, Reader, Role, Scope, export, protocol, read_text};

use crate::{
    text::{Section, is_godot_text, sections, unquoted},
    values::{Resource, Resources, file_name, property},
};

const ROOT: &str = ".";
const SIGNALS: &str = "signals";
const EXT_RESOURCE: &str = "ExtResource(\"";

pub struct Godot;

fn attribute(section: &Section, key: &str) -> Option<String> {
    section.attribute(key).map(|value| unquoted(value).to_owned())
}

fn resources(found: &[Section]) -> Resources {
    let mut external = HashMap::new();
    let mut internal = HashMap::new();
    for section in found {
        match section.tag.as_str() {
            "ext_resource" => {
                if let (Some(id), Some(path)) = (attribute(section, "id"), attribute(section, "path")) {
                    let kind = attribute(section, "type").unwrap_or_else(|| "Resource".to_owned());
                    external.insert(id, Resource { kind, path });
                }
            }
            "sub_resource" => {
                if let (Some(id), Some(kind)) = (attribute(section, "id"), attribute(section, "type")) {
                    internal.insert(id, kind);
                }
            }
            _ => {}
        }
    }
    Resources { external, internal }
}

fn node_path(section: &Section) -> (String, Option<String>) {
    let name = attribute(section, "name").unwrap_or_default();
    match attribute(section, "parent") {
        None => (ROOT.to_owned(), None),
        Some(parent) if parent == ROOT => (name, Some(ROOT.to_owned())),
        Some(parent) => (format!("{parent}/{name}"), Some(parent)),
    }
}

fn fields_of(section: &Section, resources: &Resources) -> Vec<protocol::Field> {
    section.properties.iter().map(|(key, raw)| property(key, raw, resources)).collect()
}

fn script_of(section: &Section, resources: &Resources) -> Option<String> {
    let (_, raw) = section.properties.iter().find(|(key, _)| key == "script")?;
    let id = raw.trim().strip_prefix(EXT_RESOURCE)?.split('"').next()?;
    let path = &resources.external.get(id)?.path;
    Some(file_name(path).rsplit_once('.').map_or(file_name(path), |(stem, _)| stem).to_owned())
}

fn objects(found: &[Section], path: &str, resources: &Resources) -> Vec<Object> {
    let mut objects = Vec::new();
    let mut connections = Vec::new();
    for section in found {
        match section.tag.as_str() {
            "node" => {
                let (id, parent) = node_path(section);
                let name = attribute(section, "name").unwrap_or_else(|| id.clone());
                let instance = section
                    .attribute("instance")
                    .and_then(|raw| raw.strip_prefix(EXT_RESOURCE))
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|id| resources.external.get(id))
                    .map(|resource| resource.path.clone());
                let title = attribute(section, "type").unwrap_or_else(|| "Instance".to_owned());
                objects.push(Object {
                    id,
                    parent,
                    name,
                    role: match instance {
                        Some(source) => Role::Instance {
                            source: Some(file_name(&source).to_owned()),
                        },
                        None => Role::Node { root_of: None },
                    },
                    parts: vec![Part {
                        id: "node".to_owned(),
                        title,
                        script: script_of(section, resources),
                        overrides: false,
                        fields: fields_of(section, resources),
                    }],
                    tags: Vec::new(),
                });
            }
            "sub_resource" => {
                let id = attribute(section, "id").unwrap_or_default();
                let kind = attribute(section, "type").unwrap_or_else(|| "Resource".to_owned());
                objects.push(Object {
                    id: format!("sub:{id}"),
                    parent: None,
                    name: id,
                    role: Role::Asset {
                        type_name: kind.clone(),
                        detail: Some("Sub-resource".to_owned()),
                    },
                    parts: vec![Part {
                        id: "resource".to_owned(),
                        title: kind,
                        script: script_of(section, resources),
                        overrides: false,
                        fields: fields_of(section, resources),
                    }],
                    tags: Vec::new(),
                });
            }
            "resource" => {
                let kind = attribute(&found[0], "script_class")
                    .or_else(|| attribute(&found[0], "type"))
                    .unwrap_or_else(|| "Resource".to_owned());
                objects.push(Object {
                    id: "resource".to_owned(),
                    parent: None,
                    name: file_name(path).to_owned(),
                    role: Role::Asset {
                        type_name: kind.clone(),
                        detail: None,
                    },
                    parts: vec![Part {
                        id: "resource".to_owned(),
                        title: kind,
                        script: script_of(section, resources),
                        overrides: false,
                        fields: fields_of(section, resources),
                    }],
                    tags: Vec::new(),
                });
            }
            "connection" => {
                let get = |key: &str| attribute(section, key).unwrap_or_default();
                let (from, signal, to, method) = (get("from"), get("signal"), get("to"), get("method"));
                connections.push(protocol::Field {
                    key: format!("{from}:{signal}:{to}"),
                    label: format!("{from} · {signal} → {to}"),
                    value: protocol::Value::Text(method),
                    tags: Vec::new(),
                    children: Vec::new(),
                });
            }
            _ => {}
        }
    }
    if !connections.is_empty() {
        objects.push(Object {
            id: SIGNALS.to_owned(),
            parent: None,
            name: "Signals".to_owned(),
            role: Role::Asset {
                type_name: "Connections".to_owned(),
                detail: None,
            },
            parts: vec![Part {
                id: SIGNALS.to_owned(),
                title: "Connections".to_owned(),
                script: None,
                overrides: false,
                fields: connections,
            }],
            tags: Vec::new(),
        });
    }
    objects
}

fn scopes(found: &[Section]) -> Vec<Scope> {
    found
        .iter()
        .filter_map(|section| {
            let label = match section.tag.as_str() {
                "node" => node_path(section).0,
                "sub_resource" => format!("{} {}", attribute(section, "type")?, attribute(section, "id")?),
                "ext_resource" => format!("Uses {}", file_name(&attribute(section, "path")?)),
                "resource" => "Resource".to_owned(),
                "connection" => "Signals".to_owned(),
                _ => return None,
            };
            Some(Scope {
                lines: (section.first_line, section.last_line),
                label,
                object: None,
            })
        })
        .collect()
}

fn notes(text: &str, resources: &Resources) -> Vec<Note> {
    text.lines()
        .enumerate()
        .filter_map(|(ix, line)| {
            let at = line.find(EXT_RESOURCE)?;
            let id = line[at + EXT_RESOURCE.len()..].split('"').next()?;
            let resource = resources.external.get(id)?;
            Some(Note {
                line: ix as u32 + 1,
                text: file_name(&resource.path).to_owned(),
            })
        })
        .collect()
}

impl Reader for Godot {
    fn read(&self, input: ReadInput) -> Result<Model, Failure> {
        let text = read_text(&input.path).ok_or_else(|| Failure::Missing { path: input.path.clone() })?;
        if !is_godot_text(&text) {
            return Err(Failure::NotThisFormat {
                message: format!("{} isn't a Godot text scene or resource", input.path),
            });
        }
        let found = sections(&text);
        let resources = resources(&found);
        Ok(Model {
            objects: objects(&found, &input.path, &resources),
            scopes: scopes(&found),
            notes: notes(&text, &resources),
            ..Model::default()
        })
    }
}

export!(Godot);

#[cfg(test)]
mod tests {
    use std::fs;

    use sternegit_reader_sdk::{Folder, ReadInput, Reader, Role, Side, Value, with_files};

    use super::Godot;

    const SCENE: &str = "[gd_scene load_steps=4 format=3 uid=\"uid://b1\"]

[ext_resource type=\"Script\" path=\"res://player.gd\" id=\"1_s\"]
[ext_resource type=\"Texture2D\" path=\"res://art/slime.png\" id=\"2_t\"]
[ext_resource type=\"PackedScene\" path=\"res://enemy.tscn\" id=\"3_e\"]

[sub_resource type=\"RectangleShape2D\" id=\"Shape_1\"]
size = Vector2(32, 32)

[node name=\"Player\" type=\"CharacterBody2D\"]
script = ExtResource(\"1_s\")
speed = 300.0

[node name=\"Sprite\" type=\"Sprite2D\" parent=\".\"]
texture = ExtResource(\"2_t\")
position = Vector2(0, -8)

[node name=\"Shape\" type=\"CollisionShape2D\" parent=\"Sprite\"]
shape = SubResource(\"Shape_1\")

[node name=\"Enemy\" parent=\".\" instance=ExtResource(\"3_e\")]

[connection signal=\"body_entered\" from=\"Sprite\" to=\".\" method=\"_on_body_entered\"]
";

    fn read(path: &str, text: &str) -> sternegit_reader_sdk::Model {
        let root = std::env::temp_dir().join(format!("sternegit-godot-{}", path.replace('.', "-")));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(path), text).unwrap();
        let input = ReadInput {
            kind: "scene".into(),
            path: path.into(),
            side: Side::After,
            options: Default::default(),
        };
        with_files(Folder::new(&root), || Godot.read(input)).unwrap()
    }

    #[test]
    fn a_scene_reads_as_its_node_tree_with_properties_and_signals() {
        let model = read("player.tscn", SCENE);
        let nodes: Vec<(&str, Option<&str>)> = model.objects.iter().map(|object| (object.id.as_str(), object.parent.as_deref())).collect();
        assert_eq!(
            nodes,
            [("sub:Shape_1", None), (".", None), ("Sprite", Some(".")), ("Sprite/Shape", Some("Sprite")), ("Enemy", Some(".")), ("signals", None)]
        );
        let player = &model.objects[1];
        assert_eq!((player.name.as_str(), player.parts[0].title.as_str(), player.parts[0].script.as_deref()), ("Player", "CharacterBody2D", Some("player")));
        assert_eq!(player.parts[0].fields[1].label, "Speed");
        assert!(matches!(&model.objects[4].role, Role::Instance { source: Some(source) } if source == "enemy.tscn"));
        let Value::Reference(texture) = &model.objects[2].parts[0].fields[0].value else { panic!() };
        assert_eq!(texture.name, "slime.png");
        assert_eq!(model.objects[5].parts[0].fields[0].label, "Sprite · body_entered → .");
        assert!(model.scopes.iter().any(|scope| scope.label == "Sprite/Shape"));
        assert!(model.notes.iter().any(|note| note.text == "slime.png"));
    }

    #[test]
    fn a_resource_reads_as_one_asset_named_after_its_file() {
        let model = read(
            "stats.tres",
            "[gd_resource type=\"Resource\" script_class=\"Stats\" load_steps=2 format=3]\n\n[ext_resource type=\"Script\" path=\"res://stats.gd\" id=\"1\"]\n\n[resource]\nscript = ExtResource(\"1\")\nhealth = 100\nnames = [\"a\", \"b\"]\n",
        );
        let resource = &model.objects[0];
        assert_eq!(resource.name, "stats.tres");
        assert!(matches!(&resource.role, Role::Asset { type_name, .. } if type_name == "Stats"));
        assert_eq!(resource.parts[0].fields[2].children.len(), 2);
    }

    #[test]
    fn a_file_that_is_not_godot_text_is_refused() {
        let root = std::env::temp_dir().join("sternegit-godot-binary");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("level.tscn"), "RSRC binary").unwrap();
        let input = ReadInput {
            kind: "scene".into(),
            path: "level.tscn".into(),
            side: Side::After,
            options: Default::default(),
        };
        assert!(with_files(Folder::new(&root), || Godot.read(input)).is_err());
    }
}
