use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use sha2::{Digest as _, Sha256};

const SOURCE_DIST: &str = "dist";

pub(crate) type Hashes = BTreeMap<String, String>;

pub(crate) struct Pin {
    pub(crate) id: &'static str,
    sums: &'static str,
}

impl Pin {
    pub(crate) const fn new(id: &'static str, sums: &'static str) -> Self {
        Self { id, sums }
    }

    pub(crate) fn files(&self) -> Hashes {
        read_sums(self.sums)
    }

    pub(crate) fn source_folder(&self, plugins: &Path) -> PathBuf {
        plugins.join(self.id).join(SOURCE_DIST)
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
    use super::{Hashes, read_sums, sha256};

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
        let hash = sha256(b"abc");
        assert_eq!(
            read_sums(&format!("{}  connector.json\n", hash.to_uppercase())),
            Hashes::from([("connector.json".into(), hash)])
        );
    }
}
