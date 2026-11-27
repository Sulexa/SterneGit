use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sternegit_reader_protocol::speaks_same_major;

pub(crate) const MANIFEST_FILE: &str = "reader.json";

const LONGEST_ID: usize = 40;
const SNIFFED_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ReaderId(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Manifest {
    #[serde(deserialize_with = "reader_id")]
    pub(crate) id: ReaderId,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) protocol: String,
    pub(crate) module: String,
    pub(crate) kinds: Vec<FileKind>,
    #[serde(default)]
    pub(crate) side_files: Vec<SideFile>,
    #[serde(default)]
    pub(crate) options: Vec<ReaderOption>,
    pub(crate) grant: Grant,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct FileKind {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) extensions: Vec<String>,
    #[serde(default)]
    pub(crate) sniff: Vec<String>,
    #[serde(default)]
    pub(crate) views: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct SideFile {
    pub(crate) pattern: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct ReaderOption {
    pub(crate) id: String,
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) detail: String,
    #[serde(default)]
    pub(crate) default: bool,
    #[serde(default)]
    pub(crate) hides: bool,
    #[serde(default)]
    pub(crate) rereads: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Reach {
    #[default]
    File,
    Repository,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Grant {
    #[serde(default)]
    pub(crate) read: Reach,
    #[serde(default)]
    pub(crate) jobs: BTreeSet<String>,
}

impl Grant {
    pub(crate) fn beyond(&self, granted: &Grant) -> Option<Grant> {
        let jobs: BTreeSet<String> = self.jobs.difference(&granted.jobs).cloned().collect();
        let wider = self.read > granted.read;
        (wider || !jobs.is_empty()).then(|| Grant {
            read: if wider { self.read } else { granted.read },
            jobs,
        })
    }

    pub(crate) fn from_json(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    pub(crate) fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

fn reader_id<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<ReaderId, D::Error> {
    let id = String::deserialize(deserializer)?;
    let fits = !id.is_empty()
        && id.len() <= LONGEST_ID
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_');
    if fits {
        Ok(ReaderId(id))
    } else {
        Err(serde::de::Error::custom(format!(
            "the reader's id {id:?} must be up to {LONGEST_ID} lowercase letters, digits, - or _"
        )))
    }
}

impl Manifest {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Manifest =
            serde_json::from_slice(bytes).map_err(|error| format!("The reader's manifest can't be read: {error}"))?;
        if !speaks_same_major(&manifest.protocol) {
            return Err(format!(
                "The reader speaks protocol {}, and this SterneGit speaks {}",
                manifest.protocol,
                sternegit_reader_protocol::PROTOCOL
            ));
        }
        if manifest.kinds.is_empty() {
            return Err("The reader names no kind of file".to_owned());
        }
        Ok(manifest)
    }

    pub(crate) fn kind_for(&self, path: &str, start: Option<&[u8]>) -> Option<&FileKind> {
        let lower = path.to_ascii_lowercase();
        self.kinds.iter().find(|kind| {
            kind.extensions
                .iter()
                .any(|extension| lower.ends_with(&extension.to_ascii_lowercase()))
                && start.is_none_or(|start| kind.sniffs(start))
        })
    }
}

impl FileKind {
    pub(crate) fn sniffs(&self, start: &[u8]) -> bool {
        let start = String::from_utf8_lossy(&start[..start.len().min(SNIFFED_BYTES)]);
        self.sniff.iter().all(|needle| start.contains(needle.as_str()))
    }
}

#[cfg(test)]
pub(crate) mod samples {
    pub(crate) const UNITY: &str = r#"{
        "id": "unity",
        "name": "Unity",
        "version": "1.0.0",
        "protocol": "1.0",
        "module": "unity.wasm",
        "kinds": [
            { "id": "prefab", "label": "Prefab", "extensions": [".prefab"], "sniff": ["%YAML", "!u!"], "views": ["inspector", "text"] }
        ],
        "side_files": [{ "pattern": "{path}.meta" }],
        "options": [{ "id": "noise", "label": "Hide noise", "default": true, "hides": true }],
        "grant": { "read": "repository", "jobs": ["compare", "side_files"] }
    }"#;
}

#[cfg(test)]
mod tests {
    use super::{Grant, Manifest, Reach, samples::UNITY};

    #[test]
    fn a_manifest_names_its_kinds_options_and_grant() {
        let manifest = Manifest::parse(UNITY.as_bytes()).unwrap();
        assert_eq!(manifest.id.0, "unity");
        assert_eq!(manifest.kinds[0].views, ["inspector", "text"]);
        assert!(manifest.options[0].hides && !manifest.options[0].rereads);
        assert_eq!(manifest.grant.read, Reach::Repository);
    }

    #[test]
    fn a_kind_is_chosen_by_extension_then_by_its_first_bytes() {
        let manifest = Manifest::parse(UNITY.as_bytes()).unwrap();
        assert!(manifest.kind_for("Assets/Hero.PREFAB", None).is_some());
        assert!(manifest.kind_for("Assets/Hero.prefab", Some(b"%YAML 1.1\n--- !u!1 &1")).is_some());
        assert!(manifest.kind_for("Assets/Hero.prefab", Some(b"{\"binary\": true}")).is_none());
        assert!(manifest.kind_for("Assets/Hero.png", None).is_none());
    }

    #[test]
    fn a_bad_id_or_a_newer_protocol_is_refused() {
        let shouting = UNITY.replace("\"id\": \"unity\"", "\"id\": \"Unity!\"");
        assert!(Manifest::parse(shouting.as_bytes()).unwrap_err().contains("lowercase"));
        let newer = UNITY.replace("\"protocol\": \"1.0\"", "\"protocol\": \"2.0\"");
        assert!(Manifest::parse(newer.as_bytes()).unwrap_err().contains("protocol 2.0"));
    }

    #[test]
    fn asking_for_the_repository_or_a_new_job_is_beyond_a_file_grant() {
        let granted = Grant {
            read: Reach::File,
            jobs: ["compare".to_owned()].into(),
        };
        let asked = Grant {
            read: Reach::Repository,
            jobs: ["compare".to_owned(), "fixes".to_owned()].into(),
        };
        assert_eq!(
            asked.beyond(&granted),
            Some(Grant {
                read: Reach::Repository,
                jobs: ["fixes".to_owned()].into()
            })
        );
        assert_eq!(granted.beyond(&asked), None);
    }
}
