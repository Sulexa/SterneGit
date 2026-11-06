use std::{
    io,
    path::{Path, PathBuf},
};

use crate::settings::Folder;

const FILE_NAME: &str = "tabs";

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct SavedTabs {
    pub(crate) paths: Vec<PathBuf>,
    pub(crate) active: usize,
}

impl SavedTabs {
    pub(crate) fn load() -> Self {
        file().map_or_else(Self::default, |file| Self::read_from(&file))
    }

    pub(crate) fn save(&self) {
        if let Some(file) = file() {
            let _ = self.write_to(&file);
        }
    }

    pub(crate) fn still_on_disk(&self) -> Self {
        self.keeping(|path| path.is_dir())
    }

    fn read_from(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .map_or_else(|_| Self::default(), |text| Self::decode(&text))
    }

    fn write_to(&self, file: &Path) -> io::Result<()> {
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let unfinished = file.with_extension("unfinished");
        std::fs::write(&unfinished, self.encode())?;
        std::fs::rename(&unfinished, file)
    }

    fn encode(&self) -> String {
        let writable = self.keeping(|path| path.to_str().is_some_and(|text| !text.contains('\n')));
        let mut text = format!("{}\n", writable.active);
        for path in writable.paths.iter().filter_map(|path| path.to_str()) {
            text.push_str(path);
            text.push('\n');
        }
        text
    }

    fn decode(text: &str) -> Self {
        let mut lines = text.lines();
        let Some(active) = lines.next().and_then(|line| line.trim().parse().ok()) else {
            return Self::default();
        };
        let saved = Self {
            paths: lines.map(PathBuf::from).collect(),
            active,
        };
        saved.keeping(|path| path.is_absolute())
    }

    fn keeping(&self, keep: impl Fn(&Path) -> bool) -> Self {
        let kept: Vec<(usize, &PathBuf)> = self
            .paths
            .iter()
            .enumerate()
            .filter(|(_, path)| keep(path))
            .collect();
        let active = kept
            .iter()
            .filter(|(ix, _)| *ix < self.active)
            .count()
            .min(kept.len().saturating_sub(1));
        Self {
            paths: kept.into_iter().map(|(_, path)| path.clone()).collect(),
            active,
        }
    }
}

fn file() -> Option<PathBuf> {
    Some(Folder::State.path()?.join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::SavedTabs;

    fn folder(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn saved(names: &[&str], active: usize) -> SavedTabs {
        SavedTabs {
            paths: names.iter().map(|name| folder(name)).collect(),
            active,
        }
    }

    fn without(removed: &'static [&'static str]) -> impl Fn(&Path) -> bool {
        move |path| !removed.iter().any(|name| path == folder(name))
    }

    #[test]
    fn saved_tabs_read_back_in_the_same_order_with_the_same_active_tab() {
        let tabs = saved(&["a", "b", "c"], 1);
        assert_eq!(SavedTabs::decode(&tabs.encode()), tabs);
    }

    #[test]
    fn saved_tabs_survive_a_trip_through_the_file() {
        let file = folder(&format!("sternegit-saved-tabs-{}", std::process::id())).join("tabs");
        let tabs = saved(&["a", "b"], 1);
        tabs.write_to(&file).unwrap();
        let read = SavedTabs::read_from(&file);
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
        assert_eq!(read, tabs);
    }

    #[test]
    fn no_file_means_no_tabs() {
        let missing = folder("sternegit-no-such-folder").join("tabs");
        assert_eq!(SavedTabs::read_from(&missing), SavedTabs::default());
    }

    #[test]
    fn a_file_that_does_not_start_with_the_active_tab_means_no_tabs() {
        let text = format!("hello\n{}\n", folder("a").display());
        assert_eq!(SavedTabs::decode(&text), SavedTabs::default());
    }

    #[test]
    fn blank_and_relative_lines_are_skipped() {
        let text = format!(
            "2\n{}\n\nrelative\n{}\n",
            folder("a").display(),
            folder("b").display()
        );
        assert_eq!(SavedTabs::decode(&text), saved(&["a", "b"], 1));
    }

    #[test]
    fn an_active_tab_past_the_end_becomes_the_last_tab() {
        assert_eq!(saved(&["a", "b"], 9).keeping(|_| true), saved(&["a", "b"], 1));
    }

    #[test]
    fn removing_a_folder_left_of_the_active_tab_keeps_the_same_tab_active() {
        let kept = saved(&["a", "b", "c"], 2).keeping(without(&["a"]));
        assert_eq!(kept, saved(&["b", "c"], 1));
    }

    #[test]
    fn removing_the_active_folder_activates_the_tab_that_takes_its_place() {
        let kept = saved(&["a", "b", "c"], 1).keeping(without(&["b"]));
        assert_eq!(kept, saved(&["a", "c"], 1));
    }

    #[test]
    fn removing_the_active_last_folder_activates_the_new_last_tab() {
        let kept = saved(&["a", "b", "c"], 2).keeping(without(&["c"]));
        assert_eq!(kept, saved(&["a", "b"], 1));
    }

    #[test]
    fn removing_every_folder_leaves_no_tabs() {
        let kept = saved(&["a", "b"], 1).keeping(|_| false);
        assert_eq!(kept, SavedTabs::default());
    }
}
