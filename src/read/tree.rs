use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Mutex,
};

use sternegit_reader_protocol::{Found, Search};

use crate::git::matches_in_working_tree;

pub(crate) trait Tree: Send + Sync {
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    fn search(&self, search: &Search) -> Vec<Found>;
}

pub(crate) struct WorkingTree {
    root: PathBuf,
    searched: Mutex<HashMap<(String, String), Vec<Found>>>,
}

impl WorkingTree {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            searched: Mutex::default(),
        }
    }

    fn inside(&self, path: &str) -> Option<PathBuf> {
        let relative = std::path::Path::new(path);
        let stays_inside = relative
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)));
        stays_inside.then(|| self.root.join(relative))
    }
}

impl Tree for WorkingTree {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(self.inside(path)?).ok()
    }

    fn search(&self, search: &Search) -> Vec<Found> {
        let key = (search.regex.clone(), search.glob.clone());
        if let Some(found) = self.searched.lock().ok().and_then(|searched| searched.get(&key).cloned()) {
            return found;
        }
        let found: Vec<Found> = matches_in_working_tree(&self.root, &search.regex, &search.glob)
            .unwrap_or_default()
            .into_iter()
            .map(|(path, text)| Found { path, text })
            .collect();
        if let Ok(mut searched) = self.searched.lock() {
            searched.insert(key, found.clone());
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::{Tree, WorkingTree};

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
}
