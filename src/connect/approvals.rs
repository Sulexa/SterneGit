use std::{collections::BTreeMap, io, path::PathBuf};

use serde_json::{Map, Value};

use super::grant::Grant;
use crate::{
    net::services::ConnectorId,
    settings::{Folder, JsonFile},
};

const FILE_NAME: &str = "connectors.json";
const VERSION: u64 = 1;
const CONNECTORS: &str = "connectors";
const KNOWN_FIELDS: [&str; 6] = ["id", "version", "files", "grant", "updated", "replaces_bundled"];

pub(crate) type Hashes = BTreeMap<String, String>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Approval {
    pub(crate) id: ConnectorId,
    pub(crate) version: String,
    pub(crate) files: Hashes,
    pub(crate) grant: Grant,
    pub(crate) updated: bool,
    pub(crate) replaces_bundled: bool,
    others: Map<String, Value>,
}

impl Approval {
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn new(id: ConnectorId, version: String, files: Hashes, grant: Grant) -> Self {
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
            id: ConnectorId(text("id")?),
            version: text("version")?,
            files: fields
                .get("files")?
                .as_object()?
                .iter()
                .map(|(file, hash)| Some((file.clone(), hash.as_str()?.to_owned())))
                .collect::<Option<_>>()?,
            grant: Grant::from_json(fields.get("grant")?),
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
        fields.insert("id".into(), self.id.0.as_str().into());
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
        fields.insert("grant".into(), self.grant.to_json());
        fields.insert("updated".into(), self.updated.into());
        fields.insert("replaces_bundled".into(), self.replaces_bundled.into());
        fields.extend(self.others.clone());
        Value::Object(fields)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Approvals {
    file: Option<PathBuf>,
    entries: Vec<Approval>,
    unreadable: Vec<Value>,
}

impl Approvals {
    pub(crate) fn load() -> Self {
        Self::at(Folder::Config.path().map(|folder| folder.join(FILE_NAME)))
    }

    pub(crate) fn at(file: Option<PathBuf>) -> Self {
        let fields = file
            .clone()
            .map(|path| JsonFile::at(path).read())
            .unwrap_or_default();
        let listed = fields
            .get(CONNECTORS)
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
        }
    }

    pub(crate) fn get(&self, id: &ConnectorId) -> Option<&Approval> {
        self.entries.iter().find(|approval| &approval.id == id)
    }

    pub(crate) fn record(&mut self, approval: Approval) -> io::Result<()> {
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
            fields.insert(CONNECTORS.into(), listed.into());
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{Approval, Approvals, Hashes};
    use crate::{connect::grant::Grant, net::services::ConnectorId};

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("fastgit-approvals-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    fn approval(id: &str) -> Approval {
        let files = Hashes::from([("connector.json".into(), "ab".repeat(32))]);
        let mut grant = Grant::default();
        grant.features.insert("pulls".into());
        Approval::new(ConnectorId(id.into()), "1.2.0".into(), files, grant)
    }

    #[test]
    fn a_recorded_approval_reads_back() {
        let folder = scratch("round-trip");
        let file = Some(folder.join("connectors.json"));
        let mut approvals = Approvals::at(file.clone());
        let mut gitea = approval("gitea");
        gitea.updated = true;
        approvals.record(gitea.clone()).unwrap();
        approvals.record(approval("gitlab")).unwrap();
        let read = Approvals::at(file);
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read.get(&ConnectorId("gitea".into())), Some(&gitea));
        assert_eq!(read.get(&ConnectorId("gitlab".into())).map(|found| found.updated), Some(false));
        assert_eq!(read.get(&ConnectorId("github".into())), None);
    }

    #[test]
    fn recording_again_replaces_and_keeps_what_fastgit_does_not_know() {
        let folder = scratch("unknown");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("connectors.json");
        let written = json!({
            "version": 1,
            "from_a_newer_fastgit": true,
            "connectors": [
                { "id": "gitea", "version": "1.0.0", "files": {}, "grant": {}, "pinned_by": "me" },
                { "id": "broken" }
            ]
        });
        std::fs::write(&path, written.to_string()).unwrap();
        let mut approvals = Approvals::at(Some(path.clone()));
        let mut newer = approvals.get(&ConnectorId("gitea".into())).unwrap().clone();
        newer.version = "1.1.0".into();
        approvals.record(newer).unwrap();
        let reread: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(reread["from_a_newer_fastgit"], true);
        assert_eq!(reread["connectors"][0]["version"], "1.1.0");
        assert_eq!(reread["connectors"][0]["pinned_by"], "me");
        assert_eq!(reread["connectors"][1], json!({ "id": "broken" }));
    }
}
