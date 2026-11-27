use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    sync::{LazyLock, Mutex, OnceLock},
};

use sternegit_reader_protocol::{Found, Search};

use crate::git::{Repo, Revision, matches_in_commit, matches_in_working_tree, paths_changed_since};

pub(crate) trait Tree: Send + Sync {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn search(&self, search: &Search) -> Vec<Found>;
}

const MOST_PATHS_NAMED: usize = 200;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Asked {
    root: PathBuf,
    commit: Option<String>,
    regex: String,
    glob: String,
}

static SEARCHED: LazyLock<Mutex<HashMap<Asked, Vec<Found>>>> = LazyLock::new(Mutex::default);

pub(crate) fn forget_searches_in(root: &Path) {
    if let Ok(mut searched) = SEARCHED.lock() {
        searched.retain(|asked, _| asked.root != root);
    }
}

fn remembered(asked: Asked, find: impl FnOnce() -> Vec<Found>) -> Vec<Found> {
    if let Some(found) = SEARCHED.lock().ok().and_then(|searched| searched.get(&asked).cloned()) {
        return found;
    }
    let found = find();
    if let Ok(mut searched) = SEARCHED.lock() {
        searched.insert(asked, found.clone());
    }
    found
}

fn stays_inside(path: &str) -> bool {
    Path::new(path).components().all(|part| matches!(part, Component::Normal(_)))
}

fn found(matches: anyhow::Result<Vec<(String, String)>>) -> Vec<Found> {
    matches
        .unwrap_or_default()
        .into_iter()
        .map(|(path, text)| Found { path, text })
        .collect()
}

pub(crate) struct WorkingTree {
    root: PathBuf,
}

impl WorkingTree {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn asked(&self, commit: Option<String>, search: &Search) -> Asked {
        Asked {
            root: self.root.clone(),
            commit,
            regex: search.regex.clone(),
            glob: search.glob.clone(),
        }
    }
}

impl Tree for WorkingTree {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        stays_inside(path).then(|| fs::read(self.root.join(path)).ok()).flatten()
    }

    fn search(&self, search: &Search) -> Vec<Found> {
        remembered(self.asked(None, search), || {
            found(matches_in_working_tree(&self.root, &search.regex, &search.glob))
        })
    }
}

pub(crate) struct RevisionTree {
    repo: Repo,
    revision: Revision,
    file: String,
    working: WorkingTree,
    tree: OnceLock<Option<gix::ObjectId>>,
}

impl RevisionTree {
    pub(crate) fn new(repo: Repo, revision: Revision, file: impl Into<String>) -> Self {
        let working = WorkingTree::new(repo.location());
        Self {
            repo,
            revision,
            file: file.into(),
            working,
            tree: OnceLock::new(),
        }
    }

    fn commit(&self) -> Option<String> {
        match self.revision {
            Revision::Head => Some("HEAD".to_owned()),
            Revision::Commit(id) => Some(id.to_string()),
            _ => None,
        }
    }

    fn tree(&self) -> Option<gix::ObjectId> {
        *self.tree.get_or_init(|| self.repo.tree_of(self.revision))
    }

    fn search_commit(&self, commit: &str, search: &Search) -> Vec<Found> {
        let location = self.repo.location();
        let whole_commit = || found(matches_in_commit(location, commit, &search.regex, &[search.glob.as_str()]));
        let Ok(changed) = paths_changed_since(location, commit, &search.glob) else {
            return whole_commit();
        };
        if changed.len() > MOST_PATHS_NAMED {
            return whole_commit();
        }
        let changed: HashSet<&str> = changed.iter().map(String::as_str).collect();
        let mut answers: Vec<Found> = self
            .working
            .search(search)
            .into_iter()
            .filter(|answer| !changed.contains(answer.path.as_str()))
            .collect();
        if !changed.is_empty() {
            let named: Vec<&str> = changed.iter().copied().collect();
            answers.extend(found(matches_in_commit(location, commit, &search.regex, &named)));
        }
        answers
    }
}

impl Tree for RevisionTree {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        if !stays_inside(path) {
            return None;
        }
        if path == self.file {
            return self.repo.file_version(self.revision, OsStr::new(path)).ok().map(|version| version.bytes);
        }
        match self.tree() {
            Some(tree) => self.repo.blob_in_tree(tree, path).or_else(|| self.working.read(path)),
            None => self.working.read(path),
        }
    }

    fn search(&self, search: &Search) -> Vec<Found> {
        let Some(commit) = self.commit() else {
            return self.working.search(search);
        };
        remembered(self.working.asked(Some(commit.clone()), search), || self.search_commit(&commit, search))
    }
}

#[cfg(test)]
mod tests {
    use sternegit_reader_protocol::Search;

    use super::{RevisionTree, Tree, WorkingTree, forget_searches_in};
    use crate::git::{Revision, scratch::ScratchRepo};

    #[test]
    fn a_path_leaving_the_tree_reads_nothing() {
        let folder = std::env::temp_dir().join("sternegit-tree-inside");
        std::fs::create_dir_all(folder.join("Assets")).unwrap();
        std::fs::write(folder.join("Assets").join("A.prefab"), "a").unwrap();
        let tree = WorkingTree::new(&folder);
        assert_eq!(tree.read("Assets/A.prefab").as_deref(), Some(&b"a"[..]));
        assert_eq!(tree.read("../outside"), None);
        assert_eq!(tree.read("C:/Windows/win.ini"), None);
    }

    #[test]
    fn a_commit_tree_reads_its_own_files_then_the_working_tree() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("revision-tree")?;
        scratch.write("Hero.prefab.meta", "guid: 0123456789abcdef0123456789abcdef\n")?;
        scratch.git(&["add", "-A"])?;
        scratch.commit("first")?;
        scratch.write("Hero.prefab.meta", "guid: fedcba9876543210fedcba9876543210\n")?;
        scratch.write("Later.png.meta", "guid: 11111111111111111111111111111111\n")?;
        let (repo, _) = scratch.open()?;
        let tree = RevisionTree::new(repo, Revision::Head, "Hero.prefab");
        assert_eq!(
            tree.read("Hero.prefab.meta").as_deref(),
            Some(&b"guid: 0123456789abcdef0123456789abcdef\n"[..])
        );
        assert!(tree.read("Later.png.meta").is_some());
        let mut found: Vec<(String, String)> = tree
            .search(&Search {
                regex: "^guid: [0-9a-f]{32}".into(),
                glob: "*.meta".into(),
            })
            .into_iter()
            .map(|found| (found.path, found.text))
            .collect();
        found.sort();
        assert_eq!(
            found,
            [
                ("Hero.prefab.meta".to_owned(), "guid: 0123456789abcdef0123456789abcdef".to_owned()),
                ("Later.png.meta".to_owned(), "guid: 11111111111111111111111111111111".to_owned()),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_search_is_kept_until_the_searches_of_its_folder_are_forgotten() -> anyhow::Result<()> {
        let scratch = ScratchRepo::new("tree-forget")?;
        scratch.write("A.png.meta", "guid: 0123456789abcdef0123456789abcdef
")?;
        let root = std::path::PathBuf::from(scratch.location());
        let search = Search {
            regex: "^guid: [0-9a-f]{32}".into(),
            glob: "*.meta".into(),
        };
        let paths = || -> Vec<String> {
            let mut paths: Vec<String> = WorkingTree::new(&root).search(&search).into_iter().map(|found| found.path).collect();
            paths.sort();
            paths
        };
        assert_eq!(paths(), ["A.png.meta"]);
        scratch.write("B.png.meta", "guid: 11111111111111111111111111111111
")?;
        assert_eq!(paths(), ["A.png.meta"]);
        forget_searches_in(&root);
        assert_eq!(paths(), ["A.png.meta", "B.png.meta"]);
        Ok(())
    }
}
