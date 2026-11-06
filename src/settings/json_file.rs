use std::{io, path::PathBuf};

use serde_json::{Map, Value};

use super::Folder;

const UNFINISHED: &str = "unfinished";
const BROKEN: &str = "broken";

pub(crate) type Fields = Map<String, Value>;

pub(crate) struct JsonFile {
    path: PathBuf,
}

impl JsonFile {
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn in_folder(folder: Folder, name: &str) -> Option<Self> {
        Some(Self::at(folder.path()?.join(name)))
    }

    #[cfg(test)]
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub(crate) fn read(&self) -> Fields {
        let Ok(bytes) = std::fs::read(&self.path) else {
            return Fields::new();
        };
        match serde_json::from_slice(&bytes) {
            Ok(Value::Object(fields)) => fields,
            _ => {
                self.set_aside();
                Fields::new()
            }
        }
    }

    pub(crate) fn write(&self, fields: &Fields) -> io::Result<()> {
        if let Some(folder) = self.path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let text = serde_json::to_vec_pretty(fields).map_err(io::Error::other)?;
        let unfinished = self.path.with_extension(UNFINISHED);
        std::fs::write(&unfinished, text)?;
        std::fs::rename(&unfinished, &self.path)
    }

    pub(crate) fn update(&self, change: impl FnOnce(&mut Fields)) -> io::Result<()> {
        let mut fields = self.read();
        change(&mut fields);
        self.write(&fields)
    }

    fn set_aside(&self) {
        let _ = std::fs::rename(&self.path, self.path.with_extension(BROKEN));
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::{Fields, JsonFile};

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-json-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    fn fields(value: Value) -> Fields {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn written_fields_read_back_in_order() {
        let folder = scratch("round-trip");
        let file = JsonFile::at(folder.join("accounts.json"));
        let written = fields(json!({ "version": 1, "accounts": [{ "id": "a" }], "zeta": true }));
        file.write(&written).unwrap();
        let read = file.read();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read, written);
        assert_eq!(read.keys().collect::<Vec<_>>(), ["version", "accounts", "zeta"]);
    }

    #[test]
    fn a_missing_file_reads_as_nothing() {
        let file = JsonFile::at(scratch("missing").join("accounts.json"));
        assert_eq!(file.read(), Fields::new());
    }

    #[test]
    fn a_broken_file_is_set_aside_and_reads_as_nothing() {
        let folder = scratch("broken");
        std::fs::create_dir_all(&folder).unwrap();
        let file = JsonFile::at(folder.join("accounts.json"));
        std::fs::write(file.path(), "{ not json").unwrap();
        let read = file.read();
        let set_aside = std::fs::read_to_string(folder.join("accounts.broken")).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read, Fields::new());
        assert_eq!(set_aside, "{ not json");
    }

    #[test]
    fn a_file_holding_something_other_than_an_object_counts_as_broken() {
        let folder = scratch("not-object");
        std::fs::create_dir_all(&folder).unwrap();
        let file = JsonFile::at(folder.join("accounts.json"));
        std::fs::write(file.path(), "[1, 2]").unwrap();
        let read = file.read();
        let kept_aside = folder.join("accounts.broken").exists();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read, Fields::new());
        assert!(kept_aside);
    }

    #[test]
    fn an_update_keeps_the_fields_it_does_not_touch() {
        let folder = scratch("update");
        let file = JsonFile::at(folder.join("accounts.json"));
        file.write(&fields(json!({ "from_a_newer_sternegit": [1, 2], "version": 1 })))
            .unwrap();
        file.update(|fields| {
            fields.insert("version".into(), json!(2));
        })
        .unwrap();
        let read = file.read();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read, fields(json!({ "from_a_newer_sternegit": [1, 2], "version": 2 })));
    }
}
