use std::sync::Arc;

use serde_json::Value;

use super::{
    Reader,
    manifest::{Grant, MANIFEST_FILE, Manifest, ReaderId},
};
use crate::plugin::{self, Kind, Pin, Stop};

pub(crate) const READERS_FOLDER: &str = "readers";

pub(crate) const PINS: [Pin; 1] = [Pin::new("unity", include_str!("../../readers/unity/dist/SHA256SUMS"))];

pub(crate) struct Readers;

pub(crate) type ReaderCatalog = plugin::Catalog<Readers>;
pub(crate) type ReaderApprovals = plugin::Approvals<Readers>;

impl Kind for Readers {
    type Id = ReaderId;
    type Manifest = Manifest;
    type Grant = Grant;
    type Started = Reader;
    type Host = ();

    const MANIFEST_FILE: &'static str = MANIFEST_FILE;
    const APPROVALS_FILE: &'static str = "readers.json";
    const APPROVALS_LIST: &'static str = "readers";
    const NOUN: &'static str = "reader";

    fn pins() -> &'static [Pin] {
        &PINS
    }

    fn id_named(name: String) -> ReaderId {
        ReaderId(name)
    }

    fn name_of(id: &ReaderId) -> &str {
        &id.0
    }

    fn parse(bytes: &[u8]) -> Result<Manifest, String> {
        Manifest::parse(bytes)
    }

    fn id_of(manifest: &Manifest) -> &ReaderId {
        &manifest.id
    }

    fn version_of(manifest: &Manifest) -> &str {
        &manifest.version
    }

    fn module_of(manifest: &Manifest) -> &str {
        &manifest.module
    }

    fn asked(manifest: &Manifest) -> Grant {
        manifest.grant.clone()
    }

    fn beyond(asked: &Grant, granted: &Grant) -> Option<Grant> {
        asked.beyond(granted)
    }

    fn grant_from_json(value: &Value) -> Grant {
        Grant::from_json(value)
    }

    fn grant_to_json(grant: &Grant) -> Value {
        grant.to_json()
    }

    fn start(_: &ReaderId, wasm: &[u8], _: &Arc<()>) -> Result<Reader, Stop> {
        Reader::compile(wasm)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, sync::Arc};

    use super::{PINS, ReaderApprovals, ReaderCatalog};
    use crate::plugin::{Folders, Hashes, Standing, sha256};

    fn sources() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("readers")
    }

    #[test]
    fn every_pin_matches_the_files_in_its_dist_folder() {
        for pin in &PINS {
            let dist = pin.source_folder(&sources());
            let on_disk: Hashes = fs::read_dir(&dist)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.file_name().is_some_and(|name| name != "SHA256SUMS"))
                .map(|path| {
                    let name = path.file_name().unwrap().to_string_lossy().into_owned();
                    (name, sha256(&fs::read(&path).unwrap()))
                })
                .collect();
            assert_eq!(
                on_disk,
                pin.files(),
                "{} differs from the SHA256SUMS SterneGit embeds; run scripts/build-connectors.ps1 and rebuild",
                dist.display()
            );
        }
    }

    #[test]
    fn every_reader_with_a_dist_folder_is_pinned() {
        for entry in fs::read_dir(sources()).unwrap() {
            let folder = entry.unwrap().path();
            if folder.join("dist").is_dir() {
                let id = folder.file_name().unwrap().to_string_lossy().into_owned();
                assert!(PINS.iter().any(|pin| pin.id == id), "{id} has a dist folder but no pin");
            }
        }
    }

    #[test]
    fn the_bundled_unity_reader_is_found_in_the_sources_and_starts() {
        let folders = Folders {
            source: Some(sources()),
            ..Folders::default()
        };
        let mut catalog = ReaderCatalog::discover(&folders, ReaderApprovals::default());
        catalog.start(&Arc::new(()));
        let unity = &catalog.plugins()[0];
        assert_eq!(unity.id.0, "unity");
        assert_eq!(unity.standing, Standing::Ready);
        assert!(unity.started.is_some());
    }
}
