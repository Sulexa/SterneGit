use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

use sternegit_reader_protocol::{Found, Search};

use crate::git::{Repo, Revision, matches_in_commit, matches_in_working_tree};

pub(crate) trait Tree: Send + Sync {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn search(&self, search: &Search) -> Vec<Found>;
}

type Searched = Mutex<HashMap<(String, String), Vec<Found>>>;

fn remembered(searched: &Searched, search: &Search, find: impl FnOnce() -> Vec<Found>) -> Vec<Found> {
    let key = (search.regex.clone(), search.glob.clone());
    if let Some(found) = searched.lock().ok().and_then(|searched| searched.get(&key).cloned()) {
        return found;
    }
    let found = find();
    if let Ok(mut searched) = searched.lock() {
        searched.insert(key, found.clone());
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
    searched: Searched,
}

impl WorkingTree {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            searched: Mutex::default(),
        }
    }
}

impl Tree for WorkingTree {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        stays_inside(path).then(|| fs::read(self.root.join(path)).ok()).flatten()
    }

    fn search(&self, search: &Search) -> Vec<Found> {
        remembered(&self.searched, search, || {
            found(matches_in_working_tree(&self.root, &search.regex, &search.glob))
        })
    }
}

pub(crate) struct RevisionTree {
    repo: Repo,
    revision: Revision,
    file: String,
    working: WorkingTree,
    searched: Searched,
}

impl RevisionTree {
    pub(crate) fn new(repo: Repo, revision: Revision, file: impl Into<String>) -> Self {
        let working = WorkingTree::new(repo.location());
        Self {
            repo,
            revision,
            file: file.into(),
            working,
            searched: Mutex::default(),
        }
    }

    fn commit(&self) -> Option<String> {
        match self.revision {
            Revision::Head => Some("HEAD".to_owned()),
            Revision::Commit(id) => Some(id.to_string()),
            _ => None,
        }
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
        match self.commit() {
            Some(_) => self
                .repo
                .file_bytes(self.revision, OsStr::new(path))
                .ok()
                .or_else(|| self.working.read(path)),
            None => self.working.read(path),
        }
    }

    fn search(&self, search: &Search) -> Vec<Found> {
        let Some(commit) = self.commit() else {
            return self.working.search(search);
        };
        remembered(&self.searched, search, || {
            let mut answers = found(matches_in_commit(self.repo.location(), &commit, &search.regex, &search.glob));
            let in_commit: HashSet<String> = answers.iter().map(|answer| answer.path.clone()).collect();
            answers.extend(
                self.working
                    .search(search)
                    .into_iter()
                    .filter(|answer| !in_commit.contains(&answer.path)),
            );
            answers
        })
    }
}

#[cfg(test)]
mod tests {
    use sternegit_reader_protocol::Search;

    use super::{RevisionTree, Tree, WorkingTree};
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
}
