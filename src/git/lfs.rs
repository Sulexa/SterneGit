use std::path::PathBuf;

use anyhow::{Result, bail};

use super::Repo;

const POINTER_FIRST_LINE: &str = "version https://git-lfs.github.com/spec/v1";
const LARGEST_POINTER: usize = 1024;
const OBJECT_ID_PREFIX: &str = "oid sha256:";
const SIZE_PREFIX: &str = "size ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LfsPointer {
    object_id: String,
    pub(crate) size: u64,
}

impl LfsPointer {
    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > LARGEST_POINTER {
            return None;
        }
        let text = std::str::from_utf8(bytes).ok()?;
        let mut lines = text.lines();
        if lines.next()? != POINTER_FIRST_LINE {
            return None;
        }
        let mut object_id = None;
        let mut size = None;
        for line in lines {
            if let Some(id) = line.strip_prefix(OBJECT_ID_PREFIX) {
                object_id = Some(id.trim().to_owned());
            } else if let Some(bytes) = line.strip_prefix(SIZE_PREFIX) {
                size = bytes.trim().parse().ok();
            }
        }
        let object_id = object_id.filter(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))?;
        Some(Self {
            object_id,
            size: size?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileVersion {
    pub(crate) bytes: Vec<u8>,
    pub(crate) stored_with_lfs: bool,
}

impl Repo {
    pub(crate) fn resolve_lfs(&self, bytes: Vec<u8>) -> Result<FileVersion> {
        let Some(pointer) = LfsPointer::parse(&bytes) else {
            return Ok(FileVersion {
                bytes,
                stored_with_lfs: false,
            });
        };
        let object = self.lfs_object_path(&pointer);
        match std::fs::read(&object) {
            Ok(bytes) => Ok(FileVersion {
                bytes,
                stored_with_lfs: true,
            }),
            Err(_) => bail!(
                "Git LFS hasn't downloaded this version yet ({} bytes). Run git lfs pull to get it.",
                pointer.size
            ),
        }
    }

    pub(crate) fn lfs_object(&self, bytes: &[u8]) -> Option<PathBuf> {
        LfsPointer::parse(bytes)
            .map(|pointer| self.lfs_object_path(&pointer))
            .filter(|path| path.is_file())
    }

    fn lfs_object_path(&self, pointer: &LfsPointer) -> PathBuf {
        let id = &pointer.object_id;
        self.local()
            .common_dir()
            .join("lfs")
            .join("objects")
            .join(&id[..2])
            .join(&id[2..4])
            .join(id)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::LfsPointer;
    use crate::git::scratch::ScratchRepo;

    const ID: &str = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";

    fn pointer(size: u64) -> String {
        format!("version https://git-lfs.github.com/spec/v1\noid sha256:{ID}\nsize {size}\n")
    }

    #[test]
    fn a_pointer_names_its_object_and_size() {
        let parsed = LfsPointer::parse(pointer(12345).as_bytes()).unwrap();
        assert_eq!(parsed.size, 12345);
        assert!(LfsPointer::parse(b"version 1\noid sha256:abc\nsize 3\n").is_none());
        assert!(LfsPointer::parse(b"\x89PNG\r\n").is_none());
    }

    #[test]
    fn a_downloaded_object_replaces_its_pointer() -> Result<()> {
        let scratch = ScratchRepo::new("lfs")?;
        scratch.write("a.txt", "a\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        let missing = repo.resolve_lfs(pointer(5).into_bytes());
        assert!(missing.is_err());
        let objects = scratch.dir.join(".git/lfs/objects").join(&ID[..2]).join(&ID[2..4]);
        std::fs::create_dir_all(&objects)?;
        std::fs::write(objects.join(ID), b"movie")?;
        let found = repo.resolve_lfs(pointer(5).into_bytes())?;
        assert_eq!(found.bytes, b"movie");
        assert!(found.stored_with_lfs);
        let plain = repo.resolve_lfs(b"plain".to_vec())?;
        assert!(!plain.stored_with_lfs);
        Ok(())
    }
}
