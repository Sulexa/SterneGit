use std::collections::HashMap;

use super::{
    names::{component_title, file_stem, nicify},
    value::Field,
    yaml::{Document, Node},
};

pub(crate) type Key = i64;

const GAME_OBJECT: u32 = 1;
const PREFAB_INSTANCE: u32 = 1001;

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

pub(crate) fn object_names(documents: &[Document], lookup_assets: &HashMap<String, String>) -> HashMap<i64, (String, String)> {
    let game_objects: HashMap<i64, String> = documents
        .iter()
        .filter(|document| document.class_id == GAME_OBJECT && !document.stripped)
        .map(|document| (document.file_id, name_of(&document.body)))
        .collect();
    let instances: HashMap<i64, String> = documents
        .iter()
        .filter(|document| document.class_id == PREFAB_INSTANCE)
        .map(|document| (document.file_id, instance_name(&document.body, lookup_assets)))
        .collect();
    let mut names = HashMap::new();
    for document in documents {
        let owner = owner_id(&document.body);
        let name = if document.class_id == GAME_OBJECT {
            game_objects.get(&document.file_id).cloned()
        } else if let Some(instance) = instance_of(&document.body) {
            instances.get(&instance).cloned()
        } else if document.class_id == PREFAB_INSTANCE {
            instances.get(&document.file_id).cloned()
        } else {
            owner.and_then(|owner| game_objects.get(&owner).cloned())
        }
        .or_else(|| non_empty(name_of(&document.body)));
        let kind = if document.class_id == GAME_OBJECT {
            "GameObject".to_owned()
        } else if document.class_id == PREFAB_INSTANCE {
            "Prefab instance".to_owned()
        } else if document.type_name == "MonoBehaviour" {
            script_guid(&document.body)
                .and_then(|guid| lookup_assets.get(guid))
                .map_or_else(|| "Script".to_owned(), |path| nicify(file_stem(path)))
        } else {
            component_title(&document.type_name)
        };
        if let Some(name) = name {
            names.insert(document.file_id, (name, kind));
        }
    }
    names
}

pub(crate) fn script_guid(body: &Node) -> Option<&str> {
    body.at(&["m_Script", "guid"]).and_then(Node::text)
}

fn non_empty(text: String) -> Option<String> {
    (!text.is_empty()).then_some(text)
}

fn name_of(body: &Node) -> String {
    body.get("m_Name").and_then(Node::text).unwrap_or_default().to_owned()
}

fn file_id(node: Option<&Node>) -> Option<i64> {
    node?.get("fileID")?.integer().filter(|&id| id != 0)
}

fn owner_id(body: &Node) -> Option<i64> {
    file_id(body.get("m_GameObject"))
}

fn instance_of(body: &Node) -> Option<i64> {
    file_id(body.get("m_PrefabInstance")).or_else(|| file_id(body.get("m_PrefabInternal")))
}

fn modification(body: &Node) -> Option<&Node> {
    body.get("m_Modification")
}

fn source_guid(body: &Node) -> Option<&str> {
    body.get("m_SourcePrefab")
        .or_else(|| body.get("m_ParentPrefab"))
        .and_then(|source| source.get("guid"))
        .and_then(Node::text)
}

fn modifications(body: &Node) -> &[Node] {
    modification(body).and_then(|found| found.get("m_Modifications")).map_or(&[], Node::items)
}

fn instance_name(body: &Node, assets: &HashMap<String, String>) -> String {
    modifications(body)
        .iter()
        .find(|change| change.get("propertyPath").and_then(Node::text) == Some("m_Name"))
        .and_then(|change| change.get("value").and_then(Node::text))
        .map(ToOwned::to_owned)
        .or_else(|| source_guid(body).and_then(|guid| assets.get(guid)).map(|path| file_stem(path).to_owned()))
        .unwrap_or_else(|| "Prefab instance".to_owned())
}
