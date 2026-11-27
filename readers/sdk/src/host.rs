#[cfg(target_arch = "wasm32")]
pub(crate) use guest::{finish, log, now, read_file, search, take};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Folder, with_files};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{log, now, read_file, search};

#[cfg(target_arch = "wasm32")]
mod guest {
    use crate::Level;

    mod imports {
        #[link(wasm_import_module = "sternegit:1")]
        unsafe extern "C" {
            pub(super) fn take(into: *mut u8);
            pub(super) fn finish(out: *const u8, len: usize);
            pub(super) fn log(level: i32, text: *const u8, len: usize);
            pub(super) fn now() -> i64;
            pub(super) fn read_file(path: *const u8, len: usize) -> i32;
            pub(super) fn search(query: *const u8, len: usize) -> i32;
        }
    }

    pub(crate) fn take(len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        unsafe { imports::take(bytes.as_mut_ptr()) };
        bytes
    }

    pub(crate) fn finish(output: &[u8]) {
        unsafe { imports::finish(output.as_ptr(), output.len()) }
    }

    pub(crate) fn log(level: Level, text: &str) {
        unsafe { imports::log(level.code(), text.as_ptr(), text.len()) }
    }

    pub(crate) fn now() -> i64 {
        unsafe { imports::now() }
    }

    fn taken(len: i32) -> Option<Vec<u8>> {
        usize::try_from(len).ok().map(take)
    }

    pub(crate) fn read_file(path: &str) -> Option<Vec<u8>> {
        taken(unsafe { imports::read_file(path.as_ptr(), path.len()) })
    }

    pub(crate) fn search(query: &[u8]) -> Vec<u8> {
        taken(unsafe { imports::search(query.as_ptr(), query.len()) }).unwrap_or_default()
    }


}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        cell::RefCell,
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use regex::Regex;
    use sternegit_reader_protocol::{Found, Search};

    use crate::{Level, files::glob_matches};

    pub struct Folder {
        root: PathBuf,
    }

    impl Folder {
        pub fn new(root: impl Into<PathBuf>) -> Self {
            Self { root: root.into() }
        }

        fn paths(&self) -> Vec<String> {
            let mut found = Vec::new();
            walk(&self.root, &self.root, &mut found);
            found.sort();
            found
        }
    }

    fn walk(root: &Path, folder: &Path, found: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(folder) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, found);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    thread_local! {
        static FILES: RefCell<Option<Folder>> = const { RefCell::new(None) };
    }

    pub fn with_files<R>(folder: Folder, run: impl FnOnce() -> R) -> R {
        FILES.with(|files| *files.borrow_mut() = Some(folder));
        let result = run();
        FILES.with(|files| *files.borrow_mut() = None);
        result
    }

    fn with_folder<R>(work: impl FnOnce(&Folder) -> R) -> Option<R> {
        FILES.with(|files| files.borrow().as_ref().map(work))
    }

    pub(crate) fn read_file(path: &str) -> Option<Vec<u8>> {
        with_folder(|folder| fs::read(folder.root.join(path)).ok()).flatten()
    }

    pub(crate) fn search(query: &[u8]) -> Vec<u8> {
        let Ok(query) = serde_json::from_slice::<Search>(query) else {
            return b"[]".to_vec();
        };
        let found: Vec<Found> = with_folder(|folder| {
            if folder.root.join(".git").exists() {
                return git_grep(&folder.root, &query);
            }
            let Ok(regex) = Regex::new(&format!("(?m){}", query.regex)) else {
                return Vec::new();
            };
            folder
                .paths()
                .into_iter()
                .filter(|path| glob_matches(&query.glob, path))
                .filter_map(|path| {
                    let text = fs::read_to_string(folder.root.join(&path)).ok()?;
                    Some(
                        regex
                            .find_iter(&text)
                            .map(|found| Found {
                                path: path.clone(),
                                text: found.as_str().to_owned(),
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .flatten()
                .collect()
        })
        .unwrap_or_default();
        serde_json::to_vec(&found).unwrap_or_default()
    }

    fn git_grep(root: &Path, query: &Search) -> Vec<Found> {
        let output = Command::new("git")
            .current_dir(root)
            .args(["grep", "--untracked", "-I", "-o", "-E", "-e", &query.regex, "--", &query.glob])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let (path, text) = line.split_once(':')?;
                Some(Found {
                    path: path.to_owned(),
                    text: text.to_owned(),
                })
            })
            .collect()
    }

    pub(crate) fn log(level: Level, text: &str) {
        eprintln!("{level:?} {text}");
    }

    pub(crate) fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
    }
}
