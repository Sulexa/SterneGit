use std::collections::HashMap;

use sternegit_reader_sdk::{Field, Icon, Reference, Value, digest};

use crate::text::{split_top, unquoted};

const LONGEST_VALUE_SHOWN: usize = 300;
const AXES: [&str; 4] = ["x", "y", "z", "w"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Resource {
    pub(crate) kind: String,
    pub(crate) path: String,
}

pub(crate) struct Resources {
    pub(crate) external: HashMap<String, Resource>,
    pub(crate) internal: HashMap<String, String>,
}

pub(crate) fn label(key: &str) -> String {
    key.split('/')
        .map(|part| {
            part.split('_')
                .filter(|word| !word.is_empty())
                .map(|word| {
                    let mut chars = word.chars();
                    chars.next().map_or_else(String::new, |first| first.to_uppercase().collect::<String>() + chars.as_str())
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" › ")
}

pub(crate) fn file_name(path: &str) -> &str {
    path.trim_start_matches("res://").rsplit('/').next().unwrap_or(path)
}

fn call<'a>(raw: &'a str, names: &[&str]) -> Option<(&'a str, &'a str)> {
    let open = raw.find('(')?;
    let name = &raw[..open];
    let inner = raw[open + 1..].strip_suffix(')')?;
    names.iter().any(|known| *known == name).then_some((name, inner))
}

fn icon_for(kind: &str) -> Icon {
    match kind {
        "Script" | "GDScript" | "CSharpScript" => Icon::Script,
        "Texture2D" | "CompressedTexture2D" | "AtlasTexture" | "Image" => Icon::Image,
        "AudioStream" | "AudioStreamWAV" | "AudioStreamOggVorbis" | "AudioStreamMP3" => Icon::Sound,
        "Material" | "ShaderMaterial" | "StandardMaterial3D" | "CanvasItemMaterial" => Icon::Material,
        "PackedScene" => Icon::Prefab,
        "Animation" | "AnimationLibrary" => Icon::Clip,
        _ => Icon::Asset,
    }
}

fn shown_text(text: String) -> Value {
    if text.chars().count() <= LONGEST_VALUE_SHOWN {
        return Value::Text(text);
    }
    let start: String = text.chars().take(LONGEST_VALUE_SHOWN).collect();
    Value::Long {
        more: text.chars().count() - LONGEST_VALUE_SHOWN,
        digest: digest(text.as_bytes()),
        start,
    }
}

fn field(key: String, label: String, raw: &str, resources: &Resources) -> Field {
    let (value, children) = value(raw, resources);
    Field {
        key,
        label,
        value,
        tags: Vec::new(),
        children,
    }
}

pub(crate) fn property(key: &str, raw: &str, resources: &Resources) -> Field {
    field(key.to_owned(), label(key), raw, resources)
}

fn elements(items: Vec<String>, resources: &Resources) -> Vec<Field> {
    items
        .into_iter()
        .enumerate()
        .map(|(ix, item)| field(format!("[{ix}]"), format!("Element {ix}"), &item, resources))
        .collect()
}

pub(crate) fn value(raw: &str, resources: &Resources) -> (Value, Vec<Field>) {
    let raw = raw.trim();
    let leaf = |value: Value| (value, Vec::new());
    match raw {
        "true" => return leaf(Value::Flag(true)),
        "false" => return leaf(Value::Flag(false)),
        "null" => return leaf(Value::Nothing("Object".to_owned())),
        _ => {}
    }
    if raw.starts_with('"') {
        return leaf(shown_text(unquoted(raw).replace("\\\"", "\"").replace("\\n", "\n")));
    }
    if let Some(rest) = raw.strip_prefix('&').or_else(|| raw.strip_prefix('^')) {
        return leaf(Value::Text(unquoted(rest).to_owned()));
    }
    if raw.parse::<f64>().is_ok() {
        return leaf(Value::Number(raw.to_owned()));
    }
    if let Some((_, inner)) = call(raw, &["Vector2", "Vector2i", "Vector3", "Vector3i", "Vector4", "Vector4i", "Quaternion"]) {
        let parts = split_top(inner, ',');
        return leaf(Value::Vector(AXES.iter().zip(parts).map(|(axis, part)| ((*axis).to_owned(), part)).collect()));
    }
    if let Some((_, inner)) = call(raw, &["Color"]) {
        let channel = |part: &String| part.parse::<f32>().unwrap_or(0.);
        let parts: Vec<f32> = split_top(inner, ',').iter().map(channel).collect();
        let at = |ix: usize, default: f32| parts.get(ix).copied().unwrap_or(default);
        return leaf(Value::Color {
            rgba: [at(0, 0.), at(1, 0.), at(2, 0.), at(3, 1.)],
            hdr: None,
        });
    }
    if let Some((_, inner)) = call(raw, &["ExtResource"]) {
        let id = unquoted(inner.trim());
        return leaf(Value::Reference(match resources.external.get(id) {
            Some(resource) => Reference {
                name: file_name(&resource.path).to_owned(),
                kind: resource.kind.clone(),
                icon: icon_for(&resource.kind),
                target: Some(resource.path.clone()),
                missing: false,
            },
            None => Reference {
                name: id.to_owned(),
                kind: "Resource".to_owned(),
                icon: Icon::Asset,
                target: None,
                missing: true,
            },
        }));
    }
    if let Some((_, inner)) = call(raw, &["SubResource"]) {
        let id = unquoted(inner.trim());
        let kind = resources.internal.get(id).cloned().unwrap_or_else(|| "Sub-resource".to_owned());
        return leaf(Value::Reference(Reference {
            name: kind.clone(),
            kind: "Sub-resource".to_owned(),
            icon: icon_for(&kind),
            target: Some(format!("sub:{id}")),
            missing: false,
        }));
    }
    if let Some(inner) = raw.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        let items = split_top(inner, ',');
        return (Value::Count(items.len()), elements(items, resources));
    }
    if raw.starts_with("Packed") && raw.ends_with(')') {
        if let Some(open) = raw.find('(') {
            let items = split_top(&raw[open + 1..raw.len() - 1], ',');
            return leaf(if items.len() > 64 {
                Value::Data {
                    bytes: items.len(),
                    digest: digest(raw.as_bytes()),
                }
            } else {
                Value::Count(items.len())
            });
        }
    }
    if let Some(inner) = raw.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
        let children = split_top(inner, ',')
            .into_iter()
            .filter_map(|pair| {
                let (key, raw) = pair.split_once(':')?;
                let key = unquoted(key.trim()).to_owned();
                Some(field(key.clone(), label(&key), raw, resources))
            })
            .collect();
        return (Value::Empty, children);
    }
    leaf(shown_text(raw.to_owned()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sternegit_reader_sdk::Value;

    use super::{Resource, Resources, label, value};

    fn resources() -> Resources {
        Resources {
            external: HashMap::from([(
                "1_abc".to_owned(),
                Resource {
                    kind: "Texture2D".to_owned(),
                    path: "res://art/slime.png".to_owned(),
                },
            )]),
            internal: HashMap::from([("Shape_x".to_owned(), "RectangleShape2D".to_owned())]),
        }
    }

    #[test]
    fn property_names_read_as_words() {
        assert_eq!(label("max_health"), "Max Health");
        assert_eq!(label("theme_override_colors/font_color"), "Theme Override Colors › Font Color");
    }

    #[test]
    fn godot_values_become_vectors_colors_references_and_lists() {
        let resources = resources();
        assert_eq!(value("Vector2(0, -8)", &resources).0, Value::Vector(vec![("x".into(), "0".into()), ("y".into(), "-8".into())]));
        assert!(matches!(value("Color(1, 0.5, 0, 1)", &resources).0, Value::Color { rgba: [1., 0.5, 0., 1.], .. }));
        let Value::Reference(texture) = value("ExtResource(\"1_abc\")", &resources).0 else { panic!() };
        assert_eq!((texture.name.as_str(), texture.target.as_deref()), ("slime.png", Some("res://art/slime.png")));
        let Value::Reference(shape) = value("SubResource(\"Shape_x\")", &resources).0 else { panic!() };
        assert_eq!(shape.name, "RectangleShape2D");
        let (count, children) = value("[1, \"two\", Vector2(3, 4)]", &resources);
        assert_eq!((count, children.len()), (Value::Count(3), 3));
        assert_eq!(value("true", &resources).0, Value::Flag(true));
        assert_eq!(value("\"Hi\"", &resources).0, Value::Text("Hi".into()));
        assert_eq!(value("300.0", &resources).0, Value::Number("300.0".into()));
    }
}
