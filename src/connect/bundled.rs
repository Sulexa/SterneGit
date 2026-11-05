use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

use sha2::{Digest as _, Sha256};

use super::approvals::Hashes;

const SOURCE_DIST: &str = "dist";

pub(crate) struct Pin {
    pub(crate) id: &'static str,
    sums: &'static str,
}

pub(crate) const PINS: [Pin; 1] = [Pin {
    id: "github",
    sums: include_str!("../../connectors/github/dist/SHA256SUMS"),
}];

impl Pin {
    pub(crate) fn files(&self) -> Hashes {
        read_sums(self.sums)
    }

    pub(crate) fn source_folder(&self, connectors: &Path) -> PathBuf {
        connectors.join(self.id).join(SOURCE_DIST)
    }
}

pub(crate) fn read_sums(text: &str) -> Hashes {
    text.lines()
        .filter_map(|line| {
            let (hash, file) = line.split_once("  ")?;
            Some((file.trim().to_owned(), hash.trim().to_ascii_lowercase()))
        })
        .collect()
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{PINS, read_sums, sha256};
    use crate::connect::approvals::Hashes;

    const SUMS_FILE: &str = "SHA256SUMS";

    fn connectors() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/connectors"))
    }

    #[test]
    fn every_pin_matches_the_files_in_its_dist_folder() {
        for pin in &PINS {
            let dist = pin.source_folder(connectors());
            let on_disk: Hashes = fs::read_dir(&dist)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.file_name().is_some_and(|name| name != SUMS_FILE))
                .map(|path| {
                    let name = path.file_name().unwrap().to_string_lossy().into_owned();
                    (name, sha256(&fs::read(&path).unwrap()))
                })
                .collect();
            assert_eq!(
                on_disk,
                pin.files(),
                "{} differs from the SHA256SUMS FastGit embeds; run scripts/build-connectors.ps1 and rebuild",
                dist.display()
            );
        }
    }

    #[test]
    fn every_connector_with_a_dist_folder_is_pinned() {
        for entry in fs::read_dir(connectors()).unwrap() {
            let folder = entry.unwrap().path();
            if folder.join("dist").is_dir() {
                let id = folder.file_name().unwrap().to_string_lossy().into_owned();
                assert!(PINS.iter().any(|pin| pin.id == id), "{id} has a dist folder but no pin");
            }
        }
    }

    #[test]
    fn sums_are_read_as_sha256sum_writes_them() {
        let text = "AB01  connector.json\nab02  github.wasm\n\nnot a line\n";
        assert_eq!(
            read_sums(text),
            Hashes::from([
                ("connector.json".into(), "ab01".into()),
                ("github.wasm".into(), "ab02".into()),
            ])
        );
    }

    #[test]
    fn hashes_are_lowercase_hex() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
