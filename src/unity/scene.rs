use std::collections::HashMap;

use super::value::Field;

pub(crate) type Key = i64;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Role {
    GameObject { root_of: Option<String> },
    Instance { source: Option<String> },
    Asset { type_name: String, detail: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Part {
    pub(crate) key: String,
    pub(crate) title: String,
    pub(crate) script: Option<String>,
    pub(crate) overrides: bool,
    pub(crate) fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Item {
    pub(crate) key: Key,
    pub(crate) name: String,
    pub(crate) role: Role,
    pub(crate) parent: Option<Key>,
    pub(crate) parts: Vec<Part>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Model {
    pub(crate) items: Vec<Item>,
    pub(crate) roots: Vec<Key>,
    pub(crate) children: HashMap<Key, Vec<Key>>,
}

impl Model {
    pub(crate) fn item(&self, key: Key) -> Option<&Item> {
        self.items.iter().find(|item| item.key == key)
    }
}
