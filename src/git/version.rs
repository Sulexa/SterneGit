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
    Conflict(Version),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Version {
    Current,
    Incoming,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Conflicted {
    Text(String),
    Versions { current: bool, incoming: bool },
}

impl Revision {
    fn spec(self, path: &OsStr) -> Option<OsString> {
        let mut spec = match self {
            Revision::WorkTree => return None,
            Revision::Index => OsString::from(":"),
            Revision::Head => OsString::from("HEAD:"),
            Revision::Commit(id) => OsString::from(format!("{id}:")),
            Revision::Conflict(Version::Current) => OsString::from(":2:"),
            Revision::Conflict(Version::Incoming) => OsString::from(":3:"),
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

    pub(crate) fn read_conflicted(&self, path: &OsStr) -> Result<Conflicted> {
        let has = |version| self.file_bytes(Revision::Conflict(version), path).is_ok();
        let (current, incoming) = (has(Version::Current), has(Version::Incoming));
        let text = (current && incoming)
            .then(|| self.file_bytes(Revision::WorkTree, path).ok())
            .flatten()
            .filter(|bytes| !bytes.contains(&0))
            .and_then(|bytes| String::from_utf8(bytes).ok());
        Ok(match text {
            Some(text) => Conflicted::Text(text),
            None => Conflicted::Versions { current, incoming },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use anyhow::Result;

    use super::{Conflicted, Revision, Version};
    use crate::git::scratch::ScratchRepo;

    fn both_change(scratch: &ScratchRepo, name: &str, ours: &str, theirs: &str) -> Result<()> {
        scratch.write(name, "base\n")?;
        scratch.git(&["add", name])?;
        scratch.commit("base")?;
        scratch.git(&["switch", "--quiet", "--create", "theirs"])?;
        scratch.write(name, theirs)?;
        scratch.git(&["commit", "--quiet", "-am", "theirs"])?;
        scratch.git(&["switch", "--quiet", "-"])?;
        scratch.write(name, ours)?;
        scratch.git(&["commit", "--quiet", "-am", "ours"])?;
        let _ = scratch.git(&["merge", "--quiet", "theirs"]);
        Ok(())
    }

    #[test]
    fn a_conflicted_text_file_is_read_with_its_markers() -> Result<()> {
        let scratch = ScratchRepo::new("conflicted-text")?;
        both_change(&scratch, "a.txt", "ours\n", "theirs\n")?;
        let (repo, _) = scratch.open()?;
        let path = OsStr::new("a.txt");
        let Conflicted::Text(text) = repo.read_conflicted(path)? else {
            panic!("a text conflict");
        };
        assert!(text.starts_with("<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> theirs"));
        assert_eq!(repo.file_bytes(Revision::Conflict(Version::Current), path)?, b"ours\n");
        assert_eq!(repo.file_bytes(Revision::Conflict(Version::Incoming), path)?, b"theirs\n");
        Ok(())
    }

    #[test]
    fn a_conflict_without_text_offers_the_versions_it_has() -> Result<()> {
        let scratch = ScratchRepo::new("conflicted-deleted")?;
        scratch.write("gone.txt", "base\n")?;
        scratch.git(&["add", "gone.txt"])?;
        scratch.commit("base")?;
        scratch.git(&["switch", "--quiet", "--create", "theirs"])?;
        scratch.git(&["rm", "--quiet", "gone.txt"])?;
        scratch.commit("delete")?;
        scratch.git(&["switch", "--quiet", "-"])?;
        scratch.write("gone.txt", "changed\n")?;
        scratch.git(&["commit", "--quiet", "-am", "change"])?;
        let _ = scratch.git(&["merge", "--quiet", "theirs"]);
        let (repo, _) = scratch.open()?;
        assert_eq!(
            repo.read_conflicted(OsStr::new("gone.txt"))?,
            Conflicted::Versions {
                current: true,
                incoming: false
            }
        );
        Ok(())
    }

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
