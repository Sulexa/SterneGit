use std::{
    io,
    path::{Path, PathBuf},
};

const SHIPPED_EXE: &str = "sternegit.exe";
const CONNECTORS: &str = "connectors";
const NEW: &str = "new";
const OLD: &str = "old";

struct Switched {
    target: PathBuf,
    set_aside: Option<PathBuf>,
}

pub(crate) fn swap_in(unpacked: &Path, exe: &Path) -> io::Result<()> {
    let pairs = shipped(unpacked, exe)?;
    let staged = pairs
        .iter()
        .try_for_each(|(source, target)| copy_all(source, &beside(target, NEW)));
    if let Err(error) = staged {
        pairs.iter().for_each(|(_, target)| remove_all(&beside(target, NEW)));
        return Err(error);
    }
    let mut switched = Vec::new();
    for (_, target) in &pairs {
        match switch(target) {
            Ok(done) => switched.push(done),
            Err(error) => {
                switched.into_iter().rev().for_each(undo);
                pairs.iter().for_each(|(_, target)| remove_all(&beside(target, NEW)));
                return Err(error);
            }
        }
    }
    Ok(())
}

pub(crate) fn tidy_after_swap(exe: &Path) {
    let (Some(folder), Some(exe_name)) = (exe.parent(), exe.file_name()) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let ours = [exe_name.to_string_lossy().into_owned(), CONNECTORS.to_owned()];
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let left_over = ours.iter().any(|shipped| {
            [NEW, OLD]
                .iter()
                .any(|suffix| name.starts_with(&format!("{shipped}.{suffix}")))
        });
        if left_over {
            remove_all(&entry.path());
        }
    }
}

fn shipped(unpacked: &Path, exe: &Path) -> io::Result<Vec<(PathBuf, PathBuf)>> {
    let folder = exe.parent().ok_or_else(|| io::Error::other("The exe has no folder"))?;
    let exe_name = exe.file_name().ok_or_else(|| io::Error::other("The exe has no name"))?;
    let mut pairs = Vec::new();
    for entry in std::fs::read_dir(unpacked)? {
        let entry = entry?;
        let name = entry.file_name();
        let target = if name.eq_ignore_ascii_case(SHIPPED_EXE) {
            folder.join(exe_name)
        } else {
            folder.join(&name)
        };
        pairs.push((entry.path(), target));
    }
    if !pairs.iter().any(|(_, target)| target == exe) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "The update holds no sternegit.exe"));
    }
    Ok(pairs)
}

fn switch(target: &Path) -> io::Result<Switched> {
    let new = beside(target, NEW);
    let set_aside = match target.symlink_metadata() {
        Ok(_) => {
            let old = beside(target, &format!("{OLD}-{}", std::process::id()));
            remove_all(&old);
            std::fs::rename(target, &old)?;
            Some(old)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if let Err(error) = std::fs::rename(&new, target) {
        if let Some(old) = &set_aside {
            let _ = std::fs::rename(old, target);
        }
        return Err(error);
    }
    Ok(Switched {
        target: target.to_owned(),
        set_aside,
    })
}

fn undo(switched: Switched) {
    remove_all(&switched.target);
    if let Some(old) = switched.set_aside {
        let _ = std::fs::rename(old, &switched.target);
    }
}

fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    path.with_file_name(name)
}

fn copy_all(source: &Path, target: &Path) -> io::Result<()> {
    remove_all(target);
    if source.is_dir() {
        std::fs::create_dir_all(target)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_all(&entry.path(), &target.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(source, target).map(|_| ())
    }
}

fn remove_all(path: &Path) {
    let _ = match path.symlink_metadata() {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(_) => Ok(()),
    };
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{swap_in, tidy_after_swap};

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-swap-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn put(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn portable_and_update(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = scratch(name);
        let portable = root.join("portable");
        put(&portable.join("sternegit.exe"), "old exe");
        put(&portable.join("connectors/github/github.wasm"), "old wasm");
        put(&portable.join("connectors/retired/retired.wasm"), "retired");
        put(&portable.join("notes.txt"), "the user's own file");
        let unpacked = root.join("unpacked");
        put(&unpacked.join("sternegit.exe"), "new exe");
        put(&unpacked.join("connectors/github/github.wasm"), "new wasm");
        (root, portable, unpacked)
    }

    #[test]
    fn the_new_exe_and_connectors_replace_the_old_ones_and_the_rest_stays() {
        let (root, portable, unpacked) = portable_and_update("replace");
        swap_in(&unpacked, &portable.join("sternegit.exe")).unwrap();
        let exe = read(&portable.join("sternegit.exe"));
        let wasm = read(&portable.join("connectors/github/github.wasm"));
        let retired = portable.join("connectors/retired").exists();
        let own = read(&portable.join("notes.txt"));
        tidy_after_swap(&portable.join("sternegit.exe"));
        let after_tidy = names(&portable);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!((exe.as_str(), wasm.as_str(), own.as_str()), ("new exe", "new wasm", "the user's own file"));
        assert!(!retired);
        assert_eq!(after_tidy, ["connectors", "notes.txt", "sternegit.exe"]);
    }

    #[test]
    fn a_renamed_exe_is_replaced_under_its_own_name() {
        let (root, portable, unpacked) = portable_and_update("renamed");
        std::fs::rename(portable.join("sternegit.exe"), portable.join("SterneGit-work.exe")).unwrap();
        swap_in(&unpacked, &portable.join("SterneGit-work.exe")).unwrap();
        let exe = read(&portable.join("SterneGit-work.exe"));
        let stray = portable.join("sternegit.exe").exists();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(exe, "new exe");
        assert!(!stray);
    }

    #[test]
    fn an_update_without_an_exe_changes_nothing() {
        let (root, portable, unpacked) = portable_and_update("no-exe");
        std::fs::remove_file(unpacked.join("sternegit.exe")).unwrap();
        let result = swap_in(&unpacked, &portable.join("sternegit.exe"));
        let left = names(&portable);
        let wasm = read(&portable.join("connectors/github/github.wasm"));
        let _ = std::fs::remove_dir_all(&root);
        assert!(result.is_err());
        assert_eq!(left, ["connectors", "notes.txt", "sternegit.exe"]);
        assert_eq!(wasm, "old wasm");
    }

    #[test]
    fn tidying_removes_only_what_a_swap_leaves_behind() {
        let root = scratch("tidy");
        for name in ["sternegit.exe", "sternegit.exe.old-41", "sternegit.exe.new", "connectors.old-41/a", "other.exe.old", "build.old"] {
            put(&root.join(name), "x");
        }
        tidy_after_swap(&root.join("sternegit.exe"));
        let left = names(&root);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(left, ["build.old", "other.exe.old", "sternegit.exe"]);
    }
}
