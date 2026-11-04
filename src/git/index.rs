use anyhow::Result;

use super::{
    FileChange, Repo,
    command::{Exit, run, run_with_input},
};

const NAMES_FROM_STDIN: [&str; 2] = ["--pathspec-from-file=-", "--pathspec-file-nul"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    Everything,
    Files(Vec<FileChange>),
}

impl Scope {
    fn files(&self) -> Option<&[FileChange]> {
        match self {
            Scope::Everything => None,
            Scope::Files(files) => Some(files),
        }
    }
}

impl Repo {
    pub(crate) fn stage(&self, scope: &Scope) -> Result<()> {
        let names = scope
            .files()
            .map(|files| nul_separated(files.iter().map(FileChange::name_as_git_wrote_it)));
        self.update_index(&["add", "--all"], names, None)
    }

    pub(crate) fn unstage(&self, scope: &Scope) -> Result<()> {
        let names = scope
            .files()
            .map(|files| nul_separated(files.iter().flat_map(FileChange::names_as_git_wrote_them)));
        if self.local().head_id().is_ok() {
            self.update_index(&["reset", "--quiet"], names, None)
        } else {
            self.update_index(&["rm", "--cached", "-r", "--quiet"], names, Some("."))
        }
    }

    fn update_index(
        &self,
        command: &[&str],
        names: Option<Vec<u8>>,
        everything: Option<&str>,
    ) -> Result<()> {
        let mut args = vec!["--literal-pathspecs"];
        args.extend(command);
        match names {
            Some(names) => {
                args.extend(NAMES_FROM_STDIN);
                run_with_input(&self.location, &args, &names).map(drop)
            }
            None => {
                args.push("--");
                args.extend(everything);
                run(&self.location, &args, Exit::Success).map(drop)
            }
        }
    }
}

fn nul_separated<'a>(names: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut joined = Vec::new();
    for name in names {
        joined.extend_from_slice(name);
        joined.push(0);
    }
    joined
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::Scope;
    use crate::git::{ChangeKind, FilePatch, Side, scratch::ScratchRepo};

    fn added_and_removed(patch: &FilePatch) -> (usize, usize) {
        let FilePatch::Diff(text) = patch else {
            panic!("expected a diff");
        };
        let lines = text
            .lines()
            .filter(|line| !line.starts_with("+++") && !line.starts_with("---"));
        lines.fold((0, 0), |(added, removed), line| {
            match line.as_bytes().first() {
                Some(b'+') => (added + 1, removed),
                Some(b'-') => (added, removed + 1),
                _ => (added, removed),
            }
        })
    }

    fn round_trip(scratch: &ScratchRepo, shown: &str) -> Result<()> {
        let (repo, _) = scratch.open()?;
        let untracked = repo.status()?.unstaged[0].clone();
        assert_eq!(&*untracked.path, shown);
        assert_eq!(
            added_and_removed(&repo.file_patch(&untracked, Side::Unstaged)?),
            (1, 0)
        );
        repo.stage(&Scope::Files(vec![untracked]))?;
        let staged = repo.status()?.staged[0].clone();
        assert_eq!((&*staged.path, staged.kind), (shown, ChangeKind::Added));
        assert_eq!(
            added_and_removed(&repo.file_patch(&staged, Side::Staged)?),
            (1, 0)
        );
        repo.unstage(&Scope::Files(vec![staged]))?;
        assert!(repo.status()?.staged.is_empty());
        Ok(())
    }

    #[test]
    fn an_accented_file_name_stages_unstages_and_compares() -> Result<()> {
        let scratch = ScratchRepo::new("accented-name")?;
        scratch.write("café crème.txt", "un\n")?;
        round_trip(&scratch, "café crème.txt")
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_named_in_latin_1_stages_unstages_and_compares() -> Result<()> {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _};
        let scratch = ScratchRepo::new("latin-1-name")?;
        std::fs::write(scratch.dir.join(OsStr::from_bytes(b"caf\xE9.txt")), "un\n")?;
        round_trip(&scratch, "café.txt")
    }

    #[test]
    fn a_deleted_file_named_in_latin_1_stages_and_unstages() -> Result<()> {
        let scratch = ScratchRepo::new("latin-1-deleted")?;
        scratch.commit_file_named(b"caf\xE9.txt", "un\n")?;
        let (repo, _) = scratch.open()?;
        let deleted = repo.status()?.unstaged[0].clone();
        assert_eq!(
            (&*deleted.path, deleted.kind),
            ("café.txt", ChangeKind::Deleted)
        );
        repo.stage(&Scope::Files(vec![deleted]))?;
        let staged = repo.status()?.staged[0].clone();
        assert_eq!(
            (&*staged.path, staged.kind),
            ("café.txt", ChangeKind::Deleted)
        );
        repo.unstage(&Scope::Files(vec![staged]))?;
        let unstaged_again = repo.status()?;
        assert!(unstaged_again.staged.is_empty());
        assert_eq!(unstaged_again.unstaged[0].kind, ChangeKind::Deleted);
        Ok(())
    }

    #[test]
    fn files_stage_unstage_and_diff() -> Result<()> {
        let scratch = ScratchRepo::new("stage")?;
        scratch.write("a.txt", "one\n")?;
        let (repo, _) = scratch.open()?;
        let untracked = repo.status()?.unstaged[0].clone();
        assert_eq!(untracked.kind, ChangeKind::Untracked);
        assert_eq!(
            added_and_removed(&repo.file_patch(&untracked, Side::Unstaged)?),
            (1, 0)
        );
        repo.stage(&Scope::Files(vec![untracked.clone()]))?;
        assert_eq!(repo.status()?.staged[0].kind, ChangeKind::Added);
        repo.unstage(&Scope::Everything)?;
        assert!(repo.status()?.staged.is_empty(), "before the first commit");

        repo.stage(&Scope::Everything)?;
        scratch.commit("first")?;
        scratch.write("a.txt", "one\ntwo\n")?;
        let before = repo.status()?;
        let modified = before.unstaged[0].clone();
        assert_eq!(modified.kind, ChangeKind::Modified);
        repo.stage(&Scope::Files(vec![modified.clone()]))?;
        let staged = repo.status()?;
        assert_ne!(staged.digest, before.digest);
        assert_eq!(
            added_and_removed(&repo.file_patch(&staged.staged[0], Side::Staged)?),
            (1, 0)
        );
        repo.unstage(&Scope::Files(vec![modified]))?;
        assert!(repo.status()?.staged.is_empty(), "after the first commit");
        Ok(())
    }
}
