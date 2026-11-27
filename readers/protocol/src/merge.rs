use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pick {
    Current,
    Incoming,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub object: String,
    pub field: String,
    pub pick: Pick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeInput {
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub base: Option<String>,
    pub current: String,
    pub incoming: String,
    #[serde(default)]
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    pub object: String,
    pub name: String,
    pub part: String,
    pub field: String,
    pub label: String,
    pub current: String,
    pub incoming: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Merged {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
    pub merged: usize,
}
