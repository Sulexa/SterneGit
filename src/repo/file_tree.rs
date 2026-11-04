use std::{cmp::Ordering, collections::BTreeMap};

use gpui_kit::SharedString;

use crate::ui::{FilePlace, FileView, Folded};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileTreeRow {
    Folder {
        path: SharedString,
        name: SharedString,
        depth: usize,
        open: bool,
    },
    File {
        ix: usize,
        place: FilePlace,
    },
}

pub(crate) fn file_rows<'a>(
    view: FileView,
    paths: impl IntoIterator<Item = &'a str>,
    folded: &Folded<SharedString>,
) -> Vec<FileTreeRow> {
    let paths = paths.into_iter().enumerate();
    match view {
        FileView::List => paths
            .map(|(ix, _)| FileTreeRow::File {
                ix,
                place: FilePlace::Listed,
            })
            .collect(),
        FileView::Tree => {
            let mut root = Folder::default();
            for (ix, path) in paths {
                root.insert(path, ix);
            }
            let mut rows = Vec::new();
            root.add_rows(None, 0, folded, &mut rows);
            rows
        }
    }
}

pub(crate) fn is_inside(path: &str, folder: &str) -> bool {
    path.strip_prefix(folder)
        .is_some_and(|rest| rest.starts_with('/'))
}

#[derive(Default)]
struct Folder<'a> {
    folders: BTreeMap<&'a str, Folder<'a>>,
    files: Vec<(&'a str, usize)>,
}

impl<'a> Folder<'a> {
    fn insert(&mut self, path: &'a str, ix: usize) {
        match path.split_once('/') {
            Some((folder, rest)) => self.folders.entry(folder).or_default().insert(rest, ix),
            None => self.files.push((path, ix)),
        }
    }

    fn add_rows(
        &self,
        parent: Option<&str>,
        depth: usize,
        folded: &Folded<SharedString>,
        rows: &mut Vec<FileTreeRow>,
    ) {
        let mut folders = self.folders.iter().collect::<Vec<_>>();
        folders.sort_by(|(a, _), (b, _)| by_name(a, b));
        for (name, folder) in folders {
            let (name, folder) = folder.merged_with_only_subfolders(name);
            let path = match parent {
                Some(parent) => format!("{parent}/{name}"),
                None => name.clone(),
            };
            let open = folded.is_open(path.as_str());
            rows.push(FileTreeRow::Folder {
                path: path.clone().into(),
                name: name.into(),
                depth,
                open,
            });
            if open {
                folder.add_rows(Some(&path), depth + 1, folded, rows);
            }
        }
        let mut files = self.files.clone();
        files.sort_by(|(a, _), (b, _)| by_name(a, b));
        rows.extend(files.into_iter().map(|(_, ix)| FileTreeRow::File {
            ix,
            place: FilePlace::InTree { depth },
        }));
    }

    fn merged_with_only_subfolders(&self, name: &str) -> (String, &Self) {
        let mut name = name.to_owned();
        let mut folder = self;
        while let Some((child, inner)) = folder.only_subfolder() {
            name.push('/');
            name.push_str(child);
            folder = inner;
        }
        (name, folder)
    }

    fn only_subfolder(&self) -> Option<(&'a str, &Self)> {
        if !self.files.is_empty() || self.folders.len() != 1 {
            return None;
        }
        self.folders
            .first_key_value()
            .map(|(name, folder)| (*name, folder))
    }
}

fn by_name(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
        .then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use gpui_kit::SharedString;

    use super::{FileTreeRow, file_rows, is_inside};
    use crate::ui::{FilePlace, FileView, Folded};

    fn folder(path: &str, name: &str, depth: usize, open: bool) -> FileTreeRow {
        FileTreeRow::Folder {
            path: path.to_owned().into(),
            name: name.to_owned().into(),
            depth,
            open,
        }
    }

    fn file(ix: usize, depth: usize) -> FileTreeRow {
        FileTreeRow::File {
            ix,
            place: FilePlace::InTree { depth },
        }
    }

    fn tree(paths: &[&str], folded: &Folded<SharedString>) -> Vec<FileTreeRow> {
        file_rows(FileView::Tree, paths.iter().copied(), folded)
    }

    fn folded(paths: &[&str]) -> Folded<SharedString> {
        let mut folded = Folded::default();
        for path in paths {
            folded.toggle(SharedString::from(path.to_string()));
        }
        folded
    }

    const COMMIT: [&str; 5] = [
        "src/repo/file_diff_panel.rs",
        "src/repo/commit_files.rs",
        "src/repo/patch_view.rs",
        "docs/mockup/history.png",
        "Cargo.toml",
    ];

    #[test]
    fn a_list_keeps_the_order_files_came_in() {
        assert_eq!(
            file_rows(FileView::List, COMMIT, &Folded::default()),
            (0..5)
                .map(|ix| FileTreeRow::File {
                    ix,
                    place: FilePlace::Listed
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_tree_lists_folders_first_then_files_both_by_name() {
        assert_eq!(
            tree(
                &["b.rs", "src/z.rs", "A.md", "docs/a.md", "src/a.rs"],
                &Folded::default()
            ),
            [
                folder("docs", "docs", 0, true),
                file(3, 1),
                folder("src", "src", 0, true),
                file(4, 1),
                file(1, 1),
                file(2, 0),
                file(0, 0),
            ]
        );
    }

    #[test]
    fn a_folder_whose_only_child_is_a_folder_shares_its_row() {
        assert_eq!(
            tree(&COMMIT, &Folded::default()),
            [
                folder("docs/mockup", "docs/mockup", 0, true),
                file(3, 1),
                folder("src/repo", "src/repo", 0, true),
                file(1, 1),
                file(0, 1),
                file(2, 1),
                file(4, 0),
            ]
        );
    }

    #[test]
    fn a_folder_with_two_subfolders_keeps_its_own_row() {
        assert_eq!(
            tree(
                &["src/ui/toast.rs", "src/repo/patch_view.rs"],
                &Folded::default()
            ),
            [
                folder("src", "src", 0, true),
                folder("src/repo", "repo", 1, true),
                file(1, 2),
                folder("src/ui", "ui", 1, true),
                file(0, 2),
            ]
        );
    }

    #[test]
    fn a_folded_folder_hides_everything_inside_it() {
        assert_eq!(
            tree(&["src/repo/a.rs", "src/ui/b.rs", "c.rs"], &folded(&["src"])),
            [folder("src", "src", 0, false), file(2, 0)]
        );
        assert_eq!(
            tree(&COMMIT, &folded(&["src/repo"])),
            [
                folder("docs/mockup", "docs/mockup", 0, true),
                file(3, 1),
                folder("src/repo", "src/repo", 0, false),
                file(4, 0),
            ]
        );
    }

    #[test]
    fn names_sort_without_regard_to_case() {
        assert_eq!(
            tree(&["b.rs", "README.md", "a.rs"], &Folded::default()),
            [file(2, 0), file(0, 0), file(1, 0)]
        );
    }

    #[test]
    fn a_file_is_inside_a_folder_only_below_a_slash() {
        assert!(is_inside("src/git/patch.rs", "src/git"));
        assert!(is_inside("src/git/patch.rs", "src"));
        assert!(!is_inside("src/gitx/patch.rs", "src/git"));
        assert!(!is_inside("src/git", "src/git"));
    }
}
