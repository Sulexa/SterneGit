use std::{io, marker::PhantomData, path::PathBuf};

use serde_json::{Map, Value};

use super::{Hashes, Kind};
use crate::settings::{Folder, JsonFile};

const VERSION: u64 = 1;
const KNOWN_FIELDS: [&str; 6] = ["id", "version", "files", "grant", "updated", "replaces_bundled"];

pub(crate) struct Approval<K: Kind> {
    pub(crate) id: K::Id,
    pub(crate) version: String,
    pub(crate) files: Hashes,
    pub(crate) grant: K::Grant,
    pub(crate) updated: bool,
    pub(crate) replaces_bundled: bool,
    others: Map<String, Value>,
}

impl<K: Kind> Clone for Approval<K> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            version: self.version.clone(),
            files: self.files.clone(),
            grant: self.grant.clone(),
            updated: self.updated,
            replaces_bundled: self.replaces_bundled,
            others: self.others.clone(),
        }
    }
}

impl<K: Kind> PartialEq for Approval<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.version == other.version
            && self.files == other.files
            && self.grant == other.grant
            && self.updated == other.updated
            && self.replaces_bundled == other.replaces_bundled
            && self.others == other.others
    }
}

impl<K: Kind> std::fmt::Debug for Approval<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Approval")
            .field("id", &self.id)
            .field("version", &self.version)
            .field("grant", &self.grant)
            .field("updated", &self.updated)
            .finish_non_exhaustive()
    }
}

impl<K: Kind> Approval<K> {
    pub(crate) fn new(id: K::Id, version: String, files: Hashes, grant: K::Grant) -> Self {
        Self {
            id,
            version,
            files,
            grant,
            updated: false,
            replaces_bundled: false,
            others: Map::new(),
        }
    }

    fn from_json(value: &Value) -> Option<Self> {
        let fields = value.as_object()?;
        let text = |name: &str| fields.get(name).and_then(Value::as_str).map(str::to_owned);
        let flag = |name: &str| fields.get(name).and_then(Value::as_bool).unwrap_or(false);
        Some(Self {
            id: K::id_named(text("id")?),
            version: text("version")?,
            files: fields
                .get("files")?
                .as_object()?
                .iter()
                .map(|(file, hash)| Some((file.clone(), hash.as_str()?.to_owned())))
                .collect::<Option<_>>()?,
            grant: K::grant_from_json(fields.get("grant")?),
            updated: flag("updated"),
            replaces_bundled: flag("replaces_bundled"),
            others: fields
                .iter()
                .filter(|(key, _)| !KNOWN_FIELDS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        })
    }

    fn to_json(&self) -> Value {
        let mut fields = Map::new();
        fields.insert("id".into(), K::name_of(&self.id).into());
        fields.insert("version".into(), self.version.as_str().into());
        fields.insert(
            "files".into(),
            Value::Object(
                self.files
                    .iter()
                    .map(|(file, hash)| (file.clone(), hash.as_str().into()))
                    .collect(),
            ),
        );
        fields.insert("grant".into(), K::grant_to_json(&self.grant));
        fields.insert("updated".into(), self.updated.into());
        fields.insert("replaces_bundled".into(), self.replaces_bundled.into());
        fields.extend(self.others.clone());
        Value::Object(fields)
    }
}

pub(crate) struct Approvals<K: Kind> {
    file: Option<PathBuf>,
    entries: Vec<Approval<K>>,
    unreadable: Vec<Value>,
    kind: PhantomData<K>,
}

impl<K: Kind> Default for Approvals<K> {
    fn default() -> Self {
        Self {
            file: None,
            entries: Vec::new(),
            unreadable: Vec::new(),
            kind: PhantomData,
        }
    }
}

impl<K: Kind> Approvals<K> {
    pub(crate) fn load() -> Self {
        Self::at(Folder::Config.path().map(|folder| folder.join(K::APPROVALS_FILE)))
    }

    pub(crate) fn at(file: Option<PathBuf>) -> Self {
        let fields = file.clone().map(|path| JsonFile::at(path).read()).unwrap_or_default();
        let listed = fields
            .get(K::APPROVALS_LIST)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut entries = Vec::new();
        let mut unreadable = Vec::new();
        for value in listed {
            match Approval::from_json(&value) {
                Some(approval) => entries.push(approval),
                None => unreadable.push(value),
            }
        }
        Self {
            file,
            entries,
            unreadable,
            kind: PhantomData,
        }
    }

    pub(crate) fn get(&self, id: &K::Id) -> Option<&Approval<K>> {
        self.entries.iter().find(|approval| &approval.id == id)
    }

    pub(crate) fn record(&mut self, approval: Approval<K>) -> io::Result<()> {
        match self.entries.iter_mut().find(|known| known.id == approval.id) {
            Some(known) => *known = approval,
            None => self.entries.push(approval),
        }
        self.save()
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.file else {
            return Ok(());
        };
        let listed: Vec<Value> = self
            .entries
            .iter()
            .map(Approval::to_json)
            .chain(self.unreadable.iter().cloned())
            .collect();
        JsonFile::at(path.clone()).update(|fields| {
            fields.insert("version".into(), VERSION.into());
            fields.insert(K::APPROVALS_LIST.into(), listed.into());
        })
    }
}
