use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Model {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<Object>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<Scope>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub name: String,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Role {
    Node {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        root_of: Option<String>,
    },
    Instance {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    Asset {
        type_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub overrides: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    #[serde(rename = "k")]
    pub key: String,
    #[serde(rename = "l")]
    pub label: String,
    #[serde(rename = "v")]
    pub value: Value,
    #[serde(rename = "t", default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(rename = "c", default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Field>,
}

impl Field {
    pub fn path_under(&self, parent: &str) -> String {
        if parent.is_empty() || self.key.starts_with(['[', ':']) {
            format!("{parent}{}", self.key)
        } else {
            format!("{parent}.{}", self.key)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    #[serde(rename = "t")]
    Text(String),
    #[serde(rename = "long")]
    Long { start: String, more: usize, digest: u64 },
    #[serde(rename = "data")]
    Data { bytes: usize, digest: u64 },
    #[serde(rename = "n")]
    Number(String),
    #[serde(rename = "f")]
    Flag(bool),
    #[serde(rename = "vec")]
    Vector(Vec<(String, String)>),
    #[serde(rename = "rgba")]
    Color { rgba: [f32; 4], hdr: Option<f32> },
    #[serde(rename = "ref")]
    Reference(Reference),
    #[serde(rename = "none")]
    Nothing(String),
    #[serde(rename = "count")]
    Count(usize),
    #[serde(rename = "")]
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub name: String,
    pub kind: String,
    pub icon: Icon,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub missing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Icon {
    Object,
    Asset,
    Script,
    Image,
    Sound,
    Material,
    Prefab,
    Clip,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub lines: (u32, u32),
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::{Field, Model, Object, Part, Role, Value};

    fn field(key: &str, value: Value, children: Vec<Field>) -> Field {
        Field {
            key: key.into(),
            label: key.into(),
            value,
            tags: Vec::new(),
            children,
        }
    }

    #[test]
    fn a_model_survives_json_and_leaves_out_what_is_empty() {
        let model = Model {
            title: None,
            objects: vec![Object {
                id: "1".into(),
                parent: None,
                name: "Player".into(),
                role: Role::Node { root_of: None },
                parts: vec![Part {
                    id: "4".into(),
                    title: "Transform".into(),
                    script: None,
                    overrides: false,
                    fields: vec![field("m_LocalPosition", Value::Vector(vec![("x".into(), "1".into())]), Vec::new())],
                }],
                tags: vec!["noise".into()],
            }],
            scopes: Vec::new(),
        };
        let json = serde_json::to_string(&model).unwrap();
        assert!(!json.contains("scopes") && !json.contains("overrides"), "{json}");
        assert_eq!(serde_json::from_str::<Model>(&json).unwrap(), model);
    }

    #[test]
    fn a_number_field_takes_few_bytes() {
        let json = serde_json::to_string(&field("time", Value::Number("0".into()), Vec::new())).unwrap();
        assert_eq!(json, r#"{"k":"time","l":"time","v":{"n":"0"}}"#);
        let empty = serde_json::to_string(&field("m_Curve", Value::Empty, Vec::new())).unwrap();
        assert_eq!(empty, r#"{"k":"m_Curve","l":"m_Curve","v":""}"#);
    }

    #[test]
    fn paths_join_keys_with_dots_except_before_indexes() {
        let curve = field("m_Curve", Value::Empty, Vec::new());
        let first = field("[0]", Value::Empty, Vec::new());
        assert_eq!(curve.path_under(""), "m_Curve");
        assert_eq!(first.path_under("Module.m_Curve"), "Module.m_Curve[0]");
        assert_eq!(curve.path_under("Module"), "Module.m_Curve");
    }
}
