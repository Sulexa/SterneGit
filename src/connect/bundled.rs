use crate::plugin::Pin;

pub(crate) const PINS: [Pin; 2] = [
    Pin::new("github", include_str!("../../connectors/github/dist/SHA256SUMS")),
    Pin::new("azure-devops", include_str!("../../connectors/azure-devops/dist/SHA256SUMS")),
];

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::PINS;
    use crate::plugin::{Hashes, sha256};

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
                "{} differs from the SHA256SUMS SterneGit embeds; run scripts/build-connectors.ps1 and rebuild",
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
}
