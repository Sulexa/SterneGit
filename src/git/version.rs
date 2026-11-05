use std::ffi::{OsStr, OsString};

use anyhow::{Context as _, Result};
use gix::ObjectId;

use super::{
    FileVersion, Repo,
    command::{Exit, READ_ONLY, run},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Revision {
    WorkTree,
    Index,
    Head,
    Commit(ObjectId),
}

impl Revision {
    fn spec(self, path: &OsStr) -> Option<OsString> {
        let mut spec = match self {
            Revision::WorkTree => return None,
            Revision::Index => OsString::from(":"),
            Revision::Head => OsString::from("HEAD:"),
            Revision::Commit(id) => OsString::from(format!("{id}:")),
        };
        spec.push(path);
        Some(spec)
    }
}

impl Repo {
    pub(crate) fn file_bytes(&self, revision: Revision, path: &OsStr) -> Result<Vec<u8>> {
        match revision.spec(path) {
            None => {
                let on_disk = self.location().join(path);
                std::fs::read(&on_disk)
                    .with_context(|| format!("Couldn't read {}", on_disk.display()))
            }
            Some(spec) => run(
                self.location(),
                &[OsStr::new(READ_ONLY), OsStr::new("cat-file"), OsStr::new("blob"), &spec],
                Exit::Success,
            ),
        }
    }

    pub(crate) fn file_version(&self, revision: Revision, path: &OsStr) -> Result<FileVersion> {
        self.resolve_lfs(self.file_bytes(revision, path)?)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use anyhow::Result;

    use super::Revision;
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn each_revision_reads_its_own_copy_of_a_file() -> Result<()> {
        let scratch = ScratchRepo::new("versions")?;
        scratch.write("a.txt", "committed\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", "staged\n")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.write("a.txt", "on disk\n")?;
        let (repo, history) = scratch.open()?;
        let path = OsStr::new("a.txt");
        let first = history.commits[0].id;
        assert_eq!(repo.file_bytes(Revision::Head, path)?, b"committed\n");
        assert_eq!(repo.file_bytes(Revision::Commit(first), path)?, b"committed\n");
        assert_eq!(repo.file_bytes(Revision::Index, path)?, b"staged\n");
        assert_eq!(repo.file_bytes(Revision::WorkTree, path)?, b"on disk\n");
        assert!(repo.file_bytes(Revision::Head, OsStr::new("missing.txt")).is_err());
        Ok(())
    }
}
