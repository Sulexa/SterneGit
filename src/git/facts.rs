use std::ffi::OsStr;

use anyhow::Result;
use gix::ObjectId;

use super::{
    Repo, Revision,
    command::{Exit, READ_ONLY, run, run_text},
    lfs::LfsPointer,
};

const SMALL_ENOUGH_TO_BE_A_POINTER: u64 = 1024;
const LAST_CHANGE_FORMAT: &str = "--format=%ad by %an";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileSize {
    pub(crate) bytes: u64,
    pub(crate) stored_with_lfs: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DifftoolTarget {
    Unstaged,
    Staged,
    Commit { parent: ObjectId, id: ObjectId },
}

impl Repo {
    pub(crate) fn file_size(&self, revision: Revision, path: &OsStr) -> Result<FileSize> {
        let bytes = match revision {
            Revision::WorkTree => std::fs::metadata(self.location().join(path))?.len(),
            _ => {
                let spec = revision_spec(revision, path);
                let size = run_text(
                    self.location(),
                    &[OsStr::new(READ_ONLY), OsStr::new("cat-file"), OsStr::new("-s"), &spec],
                    Exit::Success,
                )?;
                size.trim().parse()?
            }
        };
        if bytes > SMALL_ENOUGH_TO_BE_A_POINTER || revision == Revision::WorkTree {
            return Ok(FileSize {
                bytes,
                stored_with_lfs: false,
            });
        }
        let pointer = LfsPointer::parse(&self.file_bytes(revision, path)?);
        Ok(match pointer {
            Some(pointer) => FileSize {
                bytes: pointer.size,
                stored_with_lfs: true,
            },
            None => FileSize {
                bytes,
                stored_with_lfs: false,
            },
        })
    }

    pub(crate) fn last_change(&self, revision: Revision, path: &OsStr) -> Result<Option<String>> {
        let start = match revision {
            Revision::WorkTree => return Ok(None),
            Revision::Head | Revision::Index => "HEAD".to_owned(),
            Revision::Commit(id) => id.to_string(),
        };
        let text = run_text(
            self.location(),
            &[
                OsStr::new(READ_ONLY),
                OsStr::new("log"),
                OsStr::new("-1"),
                OsStr::new("--date=short"),
                OsStr::new(LAST_CHANGE_FORMAT),
                OsStr::new(&start),
                OsStr::new("--"),
                path,
            ],
            Exit::Success,
        )?;
        let line = text.trim();
        Ok((!line.is_empty()).then(|| line.to_owned()))
    }

    pub(crate) fn diff_tool(&self) -> Option<String> {
        let name = run_text(
            self.location(),
            &["config", "--get", "diff.tool"],
            Exit::Success,
        )
        .ok()?;
        let name = name.trim();
        (!name.is_empty()).then(|| name.to_owned())
    }

    pub(crate) fn run_difftool(&self, target: &DifftoolTarget, path: &OsStr) -> Result<()> {
        let mut args: Vec<std::ffi::OsString> = vec!["difftool".into(), "--no-prompt".into()];
        match target {
            DifftoolTarget::Unstaged => {}
            DifftoolTarget::Staged => args.push("--cached".into()),
            DifftoolTarget::Commit { parent, id } => {
                args.push(parent.to_string().into());
                args.push(id.to_string().into());
            }
        }
        args.push("--".into());
        args.push(path.to_owned());
        run(self.location(), &args, Exit::SuccessOrDifference)?;
        Ok(())
    }
}

fn revision_spec(revision: Revision, path: &OsStr) -> std::ffi::OsString {
    let mut spec = std::ffi::OsString::from(match revision {
        Revision::Index => ":".to_owned(),
        Revision::Head => "HEAD:".to_owned(),
        Revision::Commit(id) => format!("{id}:"),
        Revision::WorkTree => String::new(),
    });
    spec.push(path);
    spec
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use anyhow::Result;

    use crate::git::{Revision, scratch::ScratchRepo};

    #[test]
    fn each_version_knows_its_size_and_when_it_last_changed() -> Result<()> {
        let scratch = ScratchRepo::new("facts")?;
        scratch.write("model.bin", "0123456789")?;
        scratch.git(&["add", "model.bin"])?;
        scratch.commit("first")?;
        scratch.write("model.bin", "0123456789abcdef")?;
        let (repo, _) = scratch.open()?;
        let path = OsStr::new("model.bin");
        assert_eq!(repo.file_size(Revision::Head, path)?.bytes, 10);
        assert_eq!(repo.file_size(Revision::WorkTree, path)?.bytes, 16);
        let change = repo.last_change(Revision::Head, path)?.expect("model.bin was committed");
        assert!(change.ends_with(" by Test"), "{change}");
        assert_eq!(repo.last_change(Revision::WorkTree, path)?, None);
        Ok(())
    }
}
