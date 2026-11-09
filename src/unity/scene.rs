use std::collections::{HashMap, HashSet};

use super::{
    names::{UnityKind, component_title, file_name, file_stem, nicify},
    value::{Field, Lookup, Target, Value, fields, rotation_value, single},
    yaml::{Document, Node},
};

pub(crate) type Key = i64;

const GAME_OBJECT: u32 = 1;
const TRANSFORM: u32 = 4;
const RECT_TRANSFORM: u32 = 224;
const PREFAB_INSTANCE: u32 = 1001;
const SCENE_ROOTS: &str = "SceneRoots";
const META_KEY: Key = 1;

const ALWAYS_SKIPPED: [&str; 12] = [
    "m_ObjectHideFlags",
    "m_CorrespondingSourceObject",
    "m_PrefabInstance",
    "m_PrefabAsset",
    "m_PrefabInternal",
    "m_PrefabParentObject",
    "m_GameObject",
    "m_Script",
    "m_EditorHideFlags",
    "m_Father",
    "m_Children",
    "m_Component",
];

const AXIS_SUFFIXES: [&str; 8] = ["x", "y", "z", "w", "r", "g", "b", "a"];

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

    pub(crate) fn from_documents(kind: UnityKind, documents: &[Document], path: &str, lookup: &Lookup) -> Self {
        if kind.has_hierarchy() || documents.iter().any(|document| document.class_id == GAME_OBJECT) {
            Hierarchy::new(documents, path, kind, lookup).model()
        } else {
            standalone(documents, path, lookup)
        }
    }

    pub(crate) fn from_meta(meta: &Node, asset: &str, lookup: &Lookup) -> Self {
        let mut items = Vec::new();
        let header: Vec<(String, Node)> = meta
            .entries()
            .iter()
            .filter(|(_, value)| matches!(value, Node::Scalar(_)))
            .cloned()
            .collect();
        let importer = meta.entries().iter().find(|(_, value)| matches!(value, Node::Map(_)));
        let mut parts = vec![Part {
            key: "asset".to_owned(),
            title: "Asset".to_owned(),
            script: None,
            overrides: false,
            fields: fields(&Node::Map(header), &[], lookup)
                .into_iter()
                .map(|mut field| {
                    if field.path == "guid" {
                        field.label = "GUID".to_owned();
                    }
                    field
                })
                .collect(),
        }];
        if let Some((_, body)) = importer {
            parts.push(Part {
                key: "importer".to_owned(),
                title: "Settings".to_owned(),
                script: None,
                overrides: false,
                fields: fields(body, &[], lookup),
            });
        }
        let type_name = importer.map_or_else(|| "Import settings".to_owned(), |(name, _)| nicify(name));
        items.push(Item {
            key: META_KEY,
            name: file_name(asset).to_owned(),
            role: Role::Asset { type_name, detail: None },
            parent: None,
            parts,
        });
        Model {
            items,
            roots: Vec::new(),
            children: HashMap::new(),
        }
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

pub(crate) fn guids_in(documents: &[Document]) -> Vec<String> {
    let mut found = HashSet::new();
    for document in documents {
        collect_guids(&document.body, &mut found);
    }
    let mut guids: Vec<String> = found.into_iter().collect();
    guids.sort();
    guids
}

pub(crate) fn guids_in_node(node: &Node, found: &mut HashSet<String>) {
    collect_guids(node, found);
}

fn collect_guids(node: &Node, found: &mut HashSet<String>) {
    match node {
        Node::Map(entries) => {
            for (key, value) in entries {
                match (key.as_str(), value) {
                    ("guid", Node::Scalar(guid)) if guid.len() == 32 => {
                        found.insert(guid.clone());
                    }
                    _ => collect_guids(value, found),
                }
            }
        }
        Node::List(items) => items.iter().for_each(|item| collect_guids(item, found)),
        Node::Scalar(_) => {}
    }
}

pub(crate) fn source_prefabs(documents: &[Document]) -> Vec<String> {
    documents
        .iter()
        .filter(|document| document.class_id == PREFAB_INSTANCE)
        .filter_map(|document| source_guid(&document.body))
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn targets_of(guid: &str, documents: &[Document]) -> HashMap<(String, i64), Target> {
    let names = object_names(documents, &HashMap::new());
    documents
        .iter()
        .map(|document| {
            let object = names.get(&document.file_id).map(|(name, _)| name.clone()).unwrap_or_default();
            let target = Target {
                type_name: document.type_name.clone(),
                object,
                script: None,
            };
            ((guid.to_owned(), document.file_id), target)
        })
        .collect()
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

fn component_ids(body: &Node) -> Vec<i64> {
    body.get("m_Component")
        .map(Node::items)
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            entry
                .get("component")
                .or_else(|| entry.entries().first().map(|(_, value)| value))
                .and_then(|reference| file_id(Some(reference)))
        })
        .collect()
}

struct Hierarchy<'a> {
    documents: HashMap<i64, &'a Document>,
    order: Vec<i64>,
    path: &'a str,
    kind: UnityKind,
    lookup: &'a Lookup,
}

impl<'a> Hierarchy<'a> {
    fn new(documents: &'a [Document], path: &'a str, kind: UnityKind, lookup: &'a Lookup) -> Self {
        Self {
            documents: documents.iter().map(|document| (document.file_id, document)).collect(),
            order: documents.iter().map(|document| document.file_id).collect(),
            path,
            kind,
            lookup,
        }
    }

    fn document(&self, id: i64) -> Option<&'a Document> {
        self.documents.get(&id).copied()
    }

    fn owner_of_transform(&self, transform: i64) -> Option<Key> {
        let document = self.document(transform)?;
        if document.stripped {
            return instance_of(&document.body);
        }
        owner_id(&document.body)
    }

    fn transform_of(&self, game_object: i64) -> Option<&'a Document> {
        let document = self.document(game_object)?;
        component_ids(&document.body)
            .into_iter()
            .filter_map(|id| self.document(id))
            .find(|component| matches!(component.class_id, TRANSFORM | RECT_TRANSFORM))
    }

    fn parent_of(&self, key: Key) -> Option<Key> {
        let document = self.document(key)?;
        let father = if document.class_id == PREFAB_INSTANCE {
            file_id(modification(&document.body).and_then(|found| found.get("m_TransformParent")))
        } else {
            self.transform_of(key).and_then(|transform| file_id(transform.body.get("m_Father")))
        };
        father.and_then(|father| self.owner_of_transform(father))
    }

    fn child_keys(&self, key: Key) -> Vec<Key> {
        self.transform_of(key)
            .and_then(|transform| transform.body.get("m_Children"))
            .map(Node::items)
            .unwrap_or_default()
            .iter()
            .filter_map(|child| file_id(Some(child)))
            .filter_map(|child| self.owner_of_transform(child))
            .collect()
    }

    fn keys(&self) -> Vec<Key> {
        self.order
            .iter()
            .copied()
            .filter(|id| {
                self.document(*id).is_some_and(|document| {
                    !document.stripped && matches!(document.class_id, GAME_OBJECT | PREFAB_INSTANCE)
                })
            })
            .collect()
    }

    fn root_order(&self, key: Key) -> Option<i64> {
        let document = self.document(key)?;
        if document.class_id == PREFAB_INSTANCE {
            modifications(&document.body)
                .iter()
                .find(|change| change.get("propertyPath").and_then(Node::text) == Some("m_RootOrder"))
                .and_then(|change| change.get("value").and_then(Node::number))
                .map(|order| order as i64)
        } else {
            self.transform_of(key)
                .and_then(|transform| transform.body.get("m_RootOrder"))
                .and_then(Node::number)
                .map(|order| order as i64)
        }
    }

    fn roots(&self, keys: &[Key], parents: &HashMap<Key, Key>) -> Vec<Key> {
        let listed: Option<Vec<Key>> = self
            .order
            .iter()
            .filter_map(|id| self.document(*id))
            .find(|document| document.type_name == SCENE_ROOTS)
            .map(|document| {
                document
                    .body
                    .get("m_Roots")
                    .map(Node::items)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|root| file_id(Some(root)))
                    .filter_map(|root| self.owner_of_transform(root).or(Some(root)))
                    .collect()
            });
        let mut roots: Vec<Key> = keys.iter().copied().filter(|key| !parents.contains_key(key)).collect();
        match listed {
            Some(listed) => {
                let position = |key: &Key| listed.iter().position(|root| root == key).unwrap_or(usize::MAX);
                roots.sort_by_key(position);
            }
            None => roots.sort_by_key(|key| self.root_order(*key).unwrap_or(i64::MAX)),
        }
        roots
    }

    fn model(&self) -> Model {
        let keys = self.keys();
        let known: HashSet<Key> = keys.iter().copied().collect();
        let parents: HashMap<Key, Key> = keys
            .iter()
            .filter_map(|key| Some((*key, self.parent_of(*key).filter(|parent| known.contains(parent))?)))
            .collect();
        let mut children: HashMap<Key, Vec<Key>> = HashMap::new();
        for key in &keys {
            let listed: Vec<Key> = self.child_keys(*key).into_iter().filter(|child| known.contains(child)).collect();
            let mut ordered = listed.clone();
            for (child, parent) in &parents {
                if parent == key && !ordered.contains(child) {
                    ordered.push(*child);
                }
            }
            let extra_start = listed.len();
            ordered[extra_start..].sort_by_key(|child| self.order.iter().position(|id| id == child));
            if !ordered.is_empty() {
                children.insert(*key, ordered);
            }
        }
        let roots = self.roots(&keys, &parents);
        let names: HashMap<Key, String> = keys.iter().map(|key| (*key, self.name(*key))).collect();
        let mut items = Vec::new();
        let mut stack: Vec<Key> = roots.iter().rev().copied().collect();
        let mut visited = HashSet::new();
        while let Some(key) = stack.pop() {
            if !visited.insert(key) {
                continue;
            }
            let parent = parents.get(&key).copied();
            items.push(self.item(key, parent, &names));
            if let Some(kids) = children.get(&key) {
                stack.extend(kids.iter().rev().copied());
            }
        }
        for key in &keys {
            if !visited.contains(key) {
                items.push(self.item(*key, parents.get(key).copied(), &names));
            }
        }
        Model { items, roots, children }
    }

    fn name(&self, key: Key) -> String {
        match self.document(key) {
            Some(document) if document.class_id == PREFAB_INSTANCE => instance_name(&document.body, &self.lookup.assets),
            Some(document) => name_of(&document.body),
            None => String::new(),
        }
    }

    fn item(&self, key: Key, parent: Option<Key>, names: &HashMap<Key, String>) -> Item {
        let Some(document) = self.document(key) else {
            return Item {
                key,
                name: String::new(),
                role: Role::GameObject { root_of: None },
                parent,
                parts: Vec::new(),
            };
        };
        let parent_name = parent.and_then(|parent| names.get(&parent)).cloned();
        if document.class_id == PREFAB_INSTANCE {
            let source = source_guid(&document.body).map(ToOwned::to_owned);
            return Item {
                key,
                name: names.get(&key).cloned().unwrap_or_default(),
                role: Role::Instance { source: source.clone() },
                parent,
                parts: self.instance_parts(&document.body, source.as_deref(), parent_name),
            };
        }
        let root_of = (parent.is_none() && self.kind == UnityKind::Prefab).then(|| file_name(self.path).to_owned());
        let mut parts = vec![self.game_object_part(&document.body, parent_name)];
        for id in component_ids(&document.body) {
            if let Some(component) = self.document(id) {
                parts.push(component_part(component, self.lookup));
            }
        }
        Item {
            key,
            name: name_of(&document.body),
            role: Role::GameObject { root_of },
            parent,
            parts,
        }
    }

    fn game_object_part(&self, body: &Node, parent: Option<String>) -> Part {
        let mut found = fields(body, &ALWAYS_SKIPPED, self.lookup);
        found.push(Field {
            path: "parent".to_owned(),
            label: "Parent".to_owned(),
            depth: 0,
            value: Value::Text(parent.unwrap_or_else(|| "None".to_owned())),
        });
        Part {
            key: "game-object".to_owned(),
            title: "GameObject".to_owned(),
            script: None,
            overrides: false,
            fields: found,
        }
    }

    fn instance_parts(&self, body: &Node, source: Option<&str>, parent: Option<String>) -> Vec<Part> {
        let mut parts: Vec<Part> = vec![Part {
            key: "game-object".to_owned(),
            title: "Prefab instance".to_owned(),
            script: None,
            overrides: false,
            fields: vec![
                Field {
                    path: "source".to_owned(),
                    label: "Prefab".to_owned(),
                    depth: 0,
                    value: body
                        .get("m_SourcePrefab")
                        .or_else(|| body.get("m_ParentPrefab"))
                        .and_then(|reference| self.lookup.reference(reference))
                        .unwrap_or_else(|| Value::Text(String::new())),
                },
                Field {
                    path: "parent".to_owned(),
                    label: "Parent".to_owned(),
                    depth: 0,
                    value: Value::Text(parent.unwrap_or_else(|| "None".to_owned())),
                },
            ],
        }];
        let root_object = source.and_then(|guid| {
            self.lookup
                .targets
                .iter()
                .find(|((owner, _), target)| owner == guid && target.type_name == "GameObject")
                .map(|(_, target)| target.object.clone())
        });
        for change in modifications(body) {
            let Some(target) = file_id(change.get("target")) else {
                continue;
            };
            let Some(path) = change.get("propertyPath").and_then(Node::text) else {
                continue;
            };
            let value = match file_id(change.get("objectReference")) {
                Some(_) => change.get("objectReference").and_then(|reference| self.lookup.reference(reference)),
                None => None,
            }
            .unwrap_or_else(|| {
                let last = path.rsplit('.').next().unwrap_or(path);
                single(last, change.get("value").unwrap_or(&Node::Scalar(String::new())), self.lookup)
            });
            let key = format!("override:{target}");
            let at = match parts.iter().position(|part| part.key == key) {
                Some(at) => at,
                None => {
                    let found = source.and_then(|guid| self.lookup.targets.get(&(guid.to_owned(), target)));
                    parts.push(Part {
                        key: key.clone(),
                        title: override_title(found, root_object.as_deref(), self.lookup, target),
                        script: found.and_then(|target| target.script.clone()),
                        overrides: true,
                        fields: Vec::new(),
                    });
                    parts.len() - 1
                }
            };
            add_override(&mut parts[at].fields, path, value);
        }
        for part in &mut parts {
            finish_rotation(&mut part.fields);
        }
        parts
    }
}

fn override_title(found: Option<&Target>, root: Option<&str>, _: &Lookup, target: i64) -> String {
    match found {
        Some(target) => {
            let title = target.script.clone().map_or_else(|| component_title(&target.type_name), |script| nicify(&script));
            match root {
                Some(root) if root != target.object && !target.object.is_empty() => format!("{} › {title}", target.object),
                _ => title,
            }
        }
        None => format!("Override of object {target}"),
    }
}

fn add_override(fields: &mut Vec<Field>, path: &str, value: Value) {
    let (base, axis) = match path.rsplit_once('.') {
        Some((base, axis)) if AXIS_SUFFIXES.contains(&axis) && !base.ends_with("Array") => (base, Some(axis)),
        _ => (path, None),
    };
    let label = override_label(base);
    match (axis, value) {
        (Some(axis), Value::Number(number) | Value::Text(number)) => {
            let axis: &'static str = AXIS_SUFFIXES.iter().find(|known| **known == axis).copied().unwrap_or("x");
            match fields.iter_mut().find(|field| field.path == base) {
                Some(Field {
                    value: Value::Vector(axes),
                    ..
                }) => {
                    axes.push((axis, number));
                    axes.sort_by_key(|(name, _)| AXIS_SUFFIXES.iter().position(|known| known == name));
                }
                _ => fields.push(Field {
                    path: base.to_owned(),
                    label,
                    depth: 0,
                    value: Value::Vector(vec![(axis, number)]),
                }),
            }
        }
        (_, value) => fields.push(Field {
            path: path.to_owned(),
            label: override_label(path),
            depth: 0,
            value,
        }),
    }
}

fn finish_rotation(fields: &mut [Field]) {
    for field in fields.iter_mut() {
        if field.path != "m_LocalRotation" {
            continue;
        }
        if let Value::Vector(axes) = &field.value {
            let part = |name: &str| axes.iter().find(|(axis, _)| *axis == name).and_then(|(_, value)| value.parse::<f64>().ok());
            if let (Some(x), Some(y), Some(z), Some(w)) = (part("x"), part("y"), part("z"), part("w")) {
                let node = Node::Map(
                    [("x", x), ("y", y), ("z", z), ("w", w)]
                        .iter()
                        .map(|(name, value)| ((*name).to_owned(), Node::Scalar(value.to_string())))
                        .collect(),
                );
                if let Some(rotation) = rotation_value(&node) {
                    field.value = rotation;
                    field.label = "Rotation".to_owned();
                }
            }
        }
    }
}

fn override_label(path: &str) -> String {
    match path {
        "m_LocalPosition" => return "Position".to_owned(),
        "m_LocalRotation" => return "Rotation".to_owned(),
        "m_LocalScale" => return "Scale".to_owned(),
        "m_LocalEulerAnglesHint" => return "Rotation hint".to_owned(),
        _ => {}
    }
    let mut out: Vec<String> = Vec::new();
    let mut parts = path.split('.').peekable();
    while let Some(part) = parts.next() {
        if part == "Array" {
            match parts.next() {
                Some("size") => out.push("Size".to_owned()),
                Some(data) => {
                    let index = data.trim_start_matches("data[").trim_end_matches(']');
                    out.push(format!("Element {index}"));
                }
                None => {}
            }
        } else {
            out.push(nicify(part));
        }
    }
    out.join(" › ")
}

pub(crate) fn component_part(component: &Document, lookup: &Lookup) -> Part {
    let script = (component.type_name == "MonoBehaviour")
        .then(|| script_guid(&component.body).map(|guid| lookup.script_name(guid).unwrap_or_else(|| "Missing script".to_owned())))
        .flatten();
    let title = script.as_deref().map_or_else(|| component_title(&component.type_name), nicify);
    let mut skipped = ALWAYS_SKIPPED.to_vec();
    if script.is_some() {
        skipped.push("m_Name");
    }
    let mut found = fields(&component.body, &skipped, lookup);
    if matches!(component.class_id, TRANSFORM | RECT_TRANSFORM) {
        for field in &mut found {
            match field.path.as_str() {
                "m_LocalPosition" => field.label = "Position".to_owned(),
                "m_LocalScale" => field.label = "Scale".to_owned(),
                "m_LocalRotation" => {
                    if let Some(rotation) = component.body.get("m_LocalRotation").and_then(rotation_value) {
                        field.value = rotation;
                    }
                    field.label = "Rotation".to_owned();
                }
                _ => {}
            }
        }
        found.retain(|field| !field.path.starts_with("m_LocalRotation."));
    }
    Part {
        key: component.file_id.to_string(),
        title,
        script,
        overrides: false,
        fields: found,
    }
}

fn standalone(documents: &[Document], path: &str, lookup: &Lookup) -> Model {
    let items = documents
        .iter()
        .filter(|document| !document.stripped)
        .map(|document| {
            let script = (document.type_name == "MonoBehaviour")
                .then(|| script_guid(&document.body).and_then(|guid| lookup.script_name(guid)))
                .flatten();
            let type_name = script.as_deref().map_or_else(|| component_title(&document.type_name), nicify);
            let name = non_empty(name_of(&document.body)).unwrap_or_else(|| file_stem(path).to_owned());
            let (parts, detail) = if document.type_name == "Material" {
                material_parts(&document.body, lookup)
            } else {
                (vec![component_part(document, lookup)], None)
            };
            Item {
                key: document.file_id,
                name,
                role: Role::Asset { type_name, detail },
                parent: None,
                parts,
            }
        })
        .collect();
    Model {
        items,
        roots: Vec::new(),
        children: HashMap::new(),
    }
}

fn material_parts(body: &Node, lookup: &Lookup) -> (Vec<Part>, Option<String>) {
    let shader = body.get("m_Shader").map(|reference| shader_name(reference, lookup));
    let saved = body.get("m_SavedProperties");
    let group = |key: &str, title: &str, source: &str| {
        let list = saved.and_then(|saved| saved.get(source)).cloned().unwrap_or(Node::List(Vec::new()));
        Part {
            key: key.to_owned(),
            title: title.to_owned(),
            script: None,
            overrides: false,
            fields: fields(&Node::Map(vec![(source.to_owned(), list)]), &[], lookup)
                .into_iter()
                .skip(1)
                .map(|mut field| {
                    field.depth = field.depth.saturating_sub(1);
                    field
                })
                .collect(),
        }
    };
    let keywords = ["m_ValidKeywords", "m_InvalidKeywords", "m_ShaderKeywords"]
        .iter()
        .filter_map(|key| Some((key, body.get(key)?)))
        .flat_map(|(_, node)| match node {
            Node::Scalar(text) => text.split_whitespace().map(ToOwned::to_owned).collect::<Vec<_>>(),
            Node::List(items) => items.iter().filter_map(|item| item.text().map(ToOwned::to_owned)).collect(),
            Node::Map(_) => Vec::new(),
        })
        .collect::<std::collections::BTreeSet<String>>();
    let keyword_fields = keywords
        .into_iter()
        .map(|keyword| Field {
            path: format!("keyword:{keyword}"),
            label: keyword,
            depth: 0,
            value: Value::Flag(true),
        })
        .collect();
    let settings_skipped = [
        "m_SavedProperties",
        "m_ValidKeywords",
        "m_InvalidKeywords",
        "m_ShaderKeywords",
        "m_Name",
        "m_ObjectHideFlags",
        "m_CorrespondingSourceObject",
        "m_PrefabInstance",
        "m_PrefabAsset",
        "m_BuildTextureStacks",
    ];
    let parts = vec![
        group("textures", "Textures", "m_TexEnvs"),
        group("floats", "Numbers", "m_Floats"),
        group("ints", "Integers", "m_Ints"),
        group("colors", "Colors", "m_Colors"),
        Part {
            key: "keywords".to_owned(),
            title: "Keywords".to_owned(),
            script: None,
            overrides: false,
            fields: keyword_fields,
        },
        Part {
            key: "settings".to_owned(),
            title: "Settings".to_owned(),
            script: None,
            overrides: false,
            fields: fields(body, &settings_skipped, lookup),
        },
    ];
    (parts, shader)
}

fn shader_name(reference: &Node, lookup: &Lookup) -> String {
    let file_id = reference.get("fileID").and_then(Node::integer).unwrap_or(0);
    match (file_id, reference.get("guid").and_then(Node::text)) {
        (46, _) => "Standard".to_owned(),
        (45, _) => "Standard (Specular setup)".to_owned(),
        (10753, _) => "Sprites/Default".to_owned(),
        (10770, _) => "UI/Default".to_owned(),
        (_, Some(guid)) => lookup.asset_path(guid).map_or_else(|| "Shader".to_owned(), |path| file_stem(path).to_owned()),
        _ => "Shader".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Model, Role, object_names};
    use crate::unity::{
        names::UnityKind,
        value::{Lookup, Value},
        yaml::{documents, plain},
    };

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
  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}
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
  crown: {fileID: 410}
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

    fn lookup() -> Lookup {
        let docs = documents(PREFAB);
        let mut lookup = Lookup::default();
        lookup.assets.insert("4f1d0c7a2b9e4e3f8a6b5c4d3e2f1a09".to_owned(), "Assets/Scripts/SlimeController.cs".to_owned());
        lookup.objects = object_names(&docs, &lookup.assets);
        lookup
    }

    #[test]
    fn file_ids_beyond_float_precision_still_find_their_objects() {
        let text = PREFAB.replace("410", "1633666572594267926").replace("400", "4336690671088430637").replace("110", "488688405476195148");
        let docs = documents(&text);
        let mut lookup = Lookup::default();
        lookup.objects = object_names(&docs, &lookup.assets);
        let model = Model::from_documents(UnityKind::Prefab, &docs, "Assets/Slime.prefab", &lookup);
        assert_eq!(model.children.get(&100), Some(&vec![488688405476195148]));
        let crown = model.items.iter().find(|item| item.name == "Crown").map(|item| item.parts.len());
        assert_eq!(crown, Some(2));
    }

    #[test]
    fn a_prefab_reads_as_its_hierarchy_with_components_in_order() {
        let lookup = lookup();
        let model = Model::from_documents(UnityKind::Prefab, &documents(PREFAB), "Assets/Slime.prefab", &lookup);
        let names: Vec<&str> = model.items.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(names, vec!["Slime", "Crown"]);
        assert_eq!(model.roots, vec![100]);
        assert_eq!(model.children.get(&100), Some(&vec![110]));
        let slime = &model.items[0];
        assert_eq!(slime.role, Role::GameObject { root_of: Some("Slime.prefab".to_owned()) });
        let titles: Vec<&str> = slime.parts.iter().map(|part| part.title.as_str()).collect();
        assert_eq!(titles, vec!["GameObject", "Transform", "Slime Controller"]);
    }

    #[test]
    fn a_script_field_that_points_inside_the_prefab_names_the_object() {
        let lookup = lookup();
        let model = Model::from_documents(UnityKind::Prefab, &documents(PREFAB), "Assets/Slime.prefab", &lookup);
        let script = &model.items[0].parts[2];
        let crown = script.fields.iter().find(|field| field.label == "Crown").map(|field| field.value.text());
        assert_eq!(crown.as_deref(), Some("Crown (Transform)"));
        assert_eq!(script.script.as_deref(), Some("SlimeController"));
    }

    #[test]
    fn a_meta_file_reads_as_its_guid_and_its_importer_settings() {
        let meta = plain("fileFormatVersion: 2\nguid: 9a3c51e07d2b4f6c8e1a0b2c3d4e5f60\nAudioImporter:\n  loadType: 0\n  preloadAudioData: 1\n");
        let model = Model::from_meta(&meta, "Assets/Audio/slime-hit.wav", &Lookup::default());
        let item = &model.items[0];
        assert_eq!(item.name, "slime-hit.wav");
        assert_eq!(item.role, Role::Asset { type_name: "Audio Importer".to_owned(), detail: None });
        assert_eq!(item.parts[0].fields[1].label, "GUID");
        assert_eq!(item.parts[1].fields[1].value, Value::Flag(true));
    }
}
