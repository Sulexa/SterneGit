use std::{
    ffi::OsStr,
    hash::{DefaultHasher, Hash as _, Hasher as _},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use gix::ObjectId;
use gpui_kit::SharedString;

use super::{
    Repo, WorkingChanges, absolute, decode_each_commit_once, folder_name,
    command::{Exit, run},
    history::short_id,
    text::decode,
};

const FOLDER_OF_WORKTREES: &str = "-worktrees";
const ROW_ID_MARK: [u8; 4] = *b"wip:";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Worktree {
    pub(crate) location: Arc<Path>,
    pub(crate) name: SharedString,
    pub(crate) branch: Option<SharedString>,
    pub(crate) head: Option<ObjectId>,
    pub(crate) is_main: bool,
    pub(crate) is_here: bool,
    pub(crate) lock: Option<SharedString>,
    pub(crate) is_missing: bool,
}

impl Worktree {
    pub(crate) fn checked_out(&self) -> SharedString {
        match (&self.branch, self.head) {
            (Some(branch), _) => branch.clone(),
            (None, Some(head)) => format!("detached at {}", short_id(head)).into(),
            (None, None) => "no commit yet".into(),
        }
    }

    pub(crate) fn is_locked(&self) -> bool {
        self.lock.is_some()
    }

    pub(crate) fn can_be_removed(&self) -> bool {
        !self.is_main && !self.is_here && !self.is_locked()
    }

    pub(crate) fn can_be_locked(&self) -> bool {
        !self.is_main && !self.is_missing
    }

    pub(crate) fn changes_row(&self) -> ObjectId {
        changes_row_of(&self.location)
    }

    fn shows_changes(&self) -> bool {
        !self.is_here && !self.is_missing && self.head.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorktreeStart {
    Branch(String),
    Tracking { branch: String, upstream: String },
    NewBranch { name: String, from: String },
}

#[derive(Clone)]
pub(crate) struct WorktreeChanges {
    pub(crate) location: Arc<Path>,
    pub(crate) changes: Arc<WorkingChanges>,
}

pub(crate) fn changes_row_of(location: &Path) -> ObjectId {
    let half = |seed: u8| {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        location.hash(&mut hasher);
        hasher.finish().to_be_bytes()
    };
    let mut bytes = [0u8; 20];
    bytes[..4].copy_from_slice(&ROW_ID_MARK);
    bytes[4..12].copy_from_slice(&half(1));
    bytes[12..].copy_from_slice(&half(2));
    ObjectId::from_bytes_or_panic(&bytes)
}

pub(crate) fn read_worktrees(repo: &gix::Repository, here: &Path) -> Vec<Worktree> {
    let here = same_folder_key(here);
    let mut found = Vec::new();
    if let Ok(main) = repo.main_repo()
        && let Some(workdir) = main.workdir()
    {
        found.push(worktree_of(&main, absolute(workdir), true, None, false));
    }
    for proxy in repo.worktrees().unwrap_or_default() {
        let lock = proxy
            .is_locked()
            .then(|| decode(&proxy.lock_reason().unwrap_or_default()).into());
        let is_missing = proxy.is_prunable();
        let Ok(base) = proxy.base() else { continue };
        let Ok(opened) = proxy.into_repo_with_possibly_inaccessible_worktree() else {
            continue;
        };
        found.push(worktree_of(&opened, absolute(&base), false, lock, is_missing));
    }
    for worktree in &mut found {
        worktree.is_here = same_folder_key(&worktree.location) == here;
    }
    found
}

fn worktree_of(
    repo: &gix::Repository,
    location: PathBuf,
    is_main: bool,
    lock: Option<SharedString>,
    is_missing: bool,
) -> Worktree {
    Worktree {
        name: folder_name(&location),
        location: location.into(),
        branch: repo
            .head_name()
            .ok()
            .flatten()
            .map(|name| decode(name.shorten()).into()),
        head: repo.head_id().ok().map(|id| id.detach()),
        is_main,
        is_here: false,
        lock,
        is_missing,
    }
}

fn same_folder_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

pub(crate) fn changes_elsewhere(worktrees: &[Worktree]) -> Vec<WorktreeChanges> {
    let read = |worktree: &Worktree| {
        let changes = Repo::open_worktree(&worktree.location).ok()?.status().ok()?;
        Some(WorktreeChanges {
            location: worktree.location.clone(),
            changes: Arc::new(changes),
        })
    };
    std::thread::scope(|scope| {
        let readers: Vec<_> = worktrees
            .iter()
            .filter(|worktree| worktree.shows_changes())
            .map(|worktree| scope.spawn(move || read(worktree)))
            .collect();
        readers
            .into_iter()
            .filter_map(|reader| reader.join().ok().flatten())
            .collect()
    })
}

pub(crate) fn is_linked_worktree(folder: &Path) -> bool {
    const POINTER: &str = "gitdir:";
    let Ok(pointer) = std::fs::read_to_string(folder.join(".git")) else {
        return false;
    };
    let Some(git_dir) = pointer.trim().strip_prefix(POINTER) else {
        return false;
    };
    Path::new(git_dir.trim())
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|holder| holder == "worktrees")
}

pub(crate) fn folder_of_worktrees(main: &Path) -> PathBuf {
    let repository = main
        .file_name()
        .map_or_else(|| "repository".into(), |name| name.to_string_lossy().into_owned());
    let parent = main.parent().unwrap_or(main);
    parent.join(format!("{repository}{FOLDER_OF_WORKTREES}"))
}

pub(crate) fn suggested_folder(main: &Path, branch: &str) -> PathBuf {
    let folder = folder_of_worktrees(main);
    let name = branch
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("worktree");
    std::iter::once(name.to_owned())
        .chain((2..).map(|n| format!("{name}-{n}")))
        .map(|candidate| folder.join(candidate))
        .find(|candidate| folder_is_free(candidate))
        .expect("an endless list of names always holds a free one")
}

pub(crate) fn folder_is_free(path: &Path) -> bool {
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_none(),
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

impl Repo {
    pub(crate) fn open_worktree(location: &Path) -> Result<Self> {
        let mut repo = gix::open(location)?;
        decode_each_commit_once(&mut repo);
        Ok(Self::new(repo))
    }

    pub(crate) fn add_worktree(&self, folder: &Path, start: &WorktreeStart) -> Result<()> {
        let folder = folder.as_os_str();
        let mut args = vec![OsStr::new("worktree"), OsStr::new("add"), OsStr::new("--quiet")];
        match start {
            WorktreeStart::Branch(branch) => args.extend([folder, OsStr::new(branch)]),
            WorktreeStart::Tracking { branch, upstream } => args.extend([
                OsStr::new("--track"),
                OsStr::new("-b"),
                OsStr::new(branch),
                folder,
                OsStr::new(upstream),
            ]),
            WorktreeStart::NewBranch { name, from } => {
                args.extend([OsStr::new("-b"), OsStr::new(name), folder, OsStr::new(from)])
            }
        }
        run(&self.location, &args, Exit::Success)?;
        Ok(())
    }

    pub(crate) fn remove_worktree(&self, location: &Path) -> Result<()> {
        let args = [OsStr::new("worktree"), OsStr::new("remove"), OsStr::new("--force"), location.as_os_str()];
        run(&self.location, &args, Exit::Success)?;
        Ok(())
    }

    pub(crate) fn lock_worktree(&self, location: &Path, reason: &str) -> Result<()> {
        let mut args = vec![OsStr::new("worktree"), OsStr::new("lock")];
        let reason = reason.trim();
        if !reason.is_empty() {
            args.extend([OsStr::new("--reason"), OsStr::new(reason)]);
        }
        args.push(location.as_os_str());
        run(&self.location, &args, Exit::Success)?;
        Ok(())
    }

    pub(crate) fn unlock_worktree(&self, location: &Path) -> Result<()> {
        let args = [OsStr::new("worktree"), OsStr::new("unlock"), location.as_os_str()];
        run(&self.location, &args, Exit::Success)?;
        Ok(())
    }

    pub(crate) fn forget_missing_worktrees(&self) -> Result<()> {
        run(&self.location, &["worktree", "prune"], Exit::Success)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;

    use super::{
        WorktreeStart, changes_elsewhere, changes_row_of, folder_is_free, is_linked_worktree, read_worktrees,
        suggested_folder,
    };
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn a_new_worktree_folder_sits_next_to_the_repository_named_after_the_branch() {
        let main = Path::new("/projects/sternegit");
        assert_eq!(
            suggested_folder(main, "feature/diff-view"),
            Path::new("/projects/sternegit-worktrees/diff-view")
        );
    }

    #[test]
    fn a_taken_folder_name_gets_a_number() -> Result<()> {
        let scratch = ScratchRepo::new("worktree-folder-taken")?;
        let taken = scratch.dir.join("project-worktrees").join("tabs");
        std::fs::create_dir_all(&taken)?;
        std::fs::write(taken.join("file.txt"), "kept")?;
        let main = scratch.dir.join("project");
        assert_eq!(suggested_folder(&main, "topic/tabs"), scratch.dir.join("project-worktrees").join("tabs-2"));
        assert!(!folder_is_free(&taken));
        assert!(folder_is_free(&scratch.dir.join("nowhere")));
        Ok(())
    }

    #[test]
    fn a_linked_worktree_is_told_apart_from_a_submodule_by_its_git_file() -> Result<()> {
        let scratch = ScratchRepo::new("worktree-pointer")?;
        let linked = scratch.dir.join("linked");
        let submodule = scratch.dir.join("submodule");
        std::fs::create_dir_all(&linked)?;
        std::fs::create_dir_all(&submodule)?;
        std::fs::write(linked.join(".git"), "gitdir: /repo/.git/worktrees/linked\n")?;
        std::fs::write(submodule.join(".git"), "gitdir: ../.git/modules/submodule\n")?;
        assert!(is_linked_worktree(&linked));
        assert!(!is_linked_worktree(&submodule));
        assert!(!is_linked_worktree(&scratch.dir));
        Ok(())
    }

    #[test]
    fn each_worktree_has_its_own_changes_row() {
        let one = changes_row_of(Path::new("/a"));
        assert_ne!(one, changes_row_of(Path::new("/b")));
        assert_eq!(one, changes_row_of(Path::new("/a")));
    }

    #[test]
    fn worktrees_list_the_main_one_first_with_their_branch_lock_and_changes() -> Result<()> {
        let scratch = ScratchRepo::new("worktree-list")?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        let linked = scratch.dir.with_file_name(format!(
            "{}-linked",
            scratch.dir.file_name().unwrap().to_string_lossy()
        ));
        let _ = std::fs::remove_dir_all(&linked);
        repo.add_worktree(
            &linked,
            &WorktreeStart::NewBranch {
                name: "topic".into(),
                from: "main".into(),
            },
        )?;
        repo.lock_worktree(&linked, "on a stick")?;
        std::fs::write(linked.join("new.txt"), "new")?;
        let local = gix::open(&scratch.dir)?;
        let worktrees = read_worktrees(&local, &scratch.dir);
        let names: Vec<(bool, bool, Option<&str>)> = worktrees
            .iter()
            .map(|worktree| (worktree.is_main, worktree.is_here, worktree.branch.as_deref()))
            .collect();
        assert_eq!(names, [(true, true, Some("main")), (false, false, Some("topic"))]);
        assert_eq!(worktrees[1].lock.as_deref(), Some("on a stick"));
        let elsewhere = changes_elsewhere(&worktrees);
        assert_eq!(elsewhere.len(), 1);
        assert_eq!(elsewhere[0].changes.unstaged.len(), 1);
        repo.unlock_worktree(&linked)?;
        repo.remove_worktree(&linked)?;
        assert!(!linked.exists());
        assert_eq!(read_worktrees(&local, &scratch.dir).len(), 1);
        Ok(())
    }

    #[test]
    fn a_deleted_worktree_folder_is_missing_until_it_is_forgotten() -> Result<()> {
        let scratch = ScratchRepo::new("worktree-missing")?;
        scratch.commit("first")?;
        let (repo, _) = scratch.open()?;
        let linked = scratch.dir.with_file_name(format!(
            "{}-gone",
            scratch.dir.file_name().unwrap().to_string_lossy()
        ));
        let _ = std::fs::remove_dir_all(&linked);
        repo.add_worktree(&linked, &WorktreeStart::NewBranch { name: "gone".into(), from: "main".into() })?;
        std::fs::remove_dir_all(&linked)?;
        let local = gix::open(&scratch.dir)?;
        assert!(read_worktrees(&local, &scratch.dir)[1].is_missing);
        repo.forget_missing_worktrees()?;
        assert_eq!(read_worktrees(&local, &scratch.dir).len(), 1);
        Ok(())
    }
}
