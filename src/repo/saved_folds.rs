use std::path::Path;

use gpui_kit::SharedString;
use serde_json::Value;

use super::ref_tree::{RefGroup, Section};
use crate::{
    git::RefKind,
    settings::{Fields, Folder, JsonFile},
    ui::Folded,
};

const FILE_NAME: &str = "opened.json";
const PULL_REQUESTS: &str = "pull requests";

pub(crate) struct LeftPaneFolds {
    pub(crate) refs: Folded<RefGroup>,
    pub(crate) pulls_open: bool,
}

impl Default for LeftPaneFolds {
    fn default() -> Self {
        Self {
            refs: Folded::all_folded_but([]),
            pulls_open: false,
        }
    }
}

impl LeftPaneFolds {
    pub(crate) fn load(repository: &Path) -> Self {
        file().map_or_else(Self::default, |file| Self::read(&file.read(), repository))
    }

    pub(crate) fn save(&self, repository: &Path) {
        if let Some(file) = file() {
            let _ = file.update(|fields| self.write(fields, repository));
        }
    }

    fn read(fields: &Fields, repository: &Path) -> Self {
        let names = repository
            .to_str()
            .and_then(|key| fields.get(key))
            .and_then(Value::as_array);
        Self::from_names(names.into_iter().flatten().filter_map(Value::as_str))
    }

    fn write(&self, fields: &mut Fields, repository: &Path) {
        let Some(key) = repository.to_str() else {
            return;
        };
        let names = self.names();
        if names.is_empty() {
            fields.remove(key);
        } else {
            fields.insert(key.to_owned(), names.into());
        }
    }

    fn from_names<'a>(names: impl Iterator<Item = &'a str>) -> Self {
        let mut pulls_open = false;
        let mut opened = Vec::new();
        for name in names {
            if name == PULL_REQUESTS {
                pulls_open = true;
            } else if let Some(group) = group_named(name) {
                opened.push(group);
            }
        }
        Self {
            refs: Folded::all_folded_but(opened),
            pulls_open,
        }
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.refs.toggled().map(group_name).collect();
        if self.pulls_open {
            names.push(PULL_REQUESTS.to_owned());
        }
        names.sort_unstable();
        names
    }
}

fn group_name(group: &RefGroup) -> String {
    match group {
        RefGroup::Section(section) => section_name(*section).to_owned(),
        RefGroup::Folder(kind, path) => format!("{}/{path}", section_name(Section::Refs(*kind))),
    }
}

fn group_named(name: &str) -> Option<RefGroup> {
    match name.split_once('/') {
        None => section_named(name).map(RefGroup::Section),
        Some((section, path)) => match section_named(section)? {
            Section::Refs(kind) if !path.is_empty() => {
                Some(RefGroup::Folder(kind, SharedString::from(path.to_owned())))
            }
            _ => None,
        },
    }
}

fn section_name(section: Section) -> &'static str {
    match section {
        Section::Refs(RefKind::LocalBranch) => "local branches",
        Section::Refs(RefKind::RemoteBranch) => "remote branches",
        Section::Refs(RefKind::Tag) => "tags",
        Section::Worktrees => "worktrees",
        Section::Stashes => "stashes",
    }
}

fn section_named(name: &str) -> Option<Section> {
    Section::IN_ORDER
        .into_iter()
        .find(|&section| section_name(section) == name)
}

fn file() -> Option<JsonFile> {
    JsonFile::in_folder(Folder::State, FILE_NAME)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gpui_kit::SharedString;
    use serde_json::json;

    use super::{LeftPaneFolds, RefGroup, Section};
    use crate::{
        git::RefKind::{self, LocalBranch, RemoteBranch, Tag},
        settings::Fields,
    };

    const REPO: &str = "/home/me/code/sternegit";

    fn fields(value: serde_json::Value) -> Fields {
        value.as_object().cloned().unwrap()
    }

    fn opened(names: &[&str]) -> LeftPaneFolds {
        LeftPaneFolds::from_names(names.iter().copied())
    }

    fn folder(kind: RefKind, path: &'static str) -> RefGroup {
        RefGroup::Folder(kind, SharedString::from(path))
    }

    fn section(section: Section) -> RefGroup {
        RefGroup::Section(section)
    }

    #[test]
    fn every_section_folder_and_the_pull_requests_read_back_as_written() {
        let names = [
            "local branches",
            "local branches/feature/ui",
            "pull requests",
            "remote branches",
            "remote branches/origin",
            "stashes",
            "tags",
            "tags/v1",
            "worktrees",
        ];
        assert_eq!(opened(&names).names(), names);
    }

    #[test]
    fn everything_starts_folded() {
        let folds = LeftPaneFolds::default();
        assert!(!folds.pulls_open);
        for every in Section::IN_ORDER {
            assert!(!folds.refs.is_open(&section(every)));
        }
        assert!(!folds.refs.is_open(&folder(RemoteBranch, "origin")));
    }

    #[test]
    fn read_names_open_the_matching_groups_and_nothing_else() {
        let read = opened(&["tags", "remote branches/origin", "pull requests"]);
        assert!(read.pulls_open);
        assert!(read.refs.is_open(&section(Section::Refs(Tag))));
        assert!(read.refs.is_open(&folder(RemoteBranch, "origin")));
        assert!(!read.refs.is_open(&section(Section::Refs(LocalBranch))));
        assert!(!read.refs.is_open(&folder(LocalBranch, "origin")));
        assert!(!read.refs.is_open(&section(Section::Stashes)));
    }

    #[test]
    fn unknown_names_and_folders_outside_ref_sections_are_skipped() {
        let read = opened(&["branches", "stashes/x", "worktrees/a", "tags/", "", "tags"]);
        assert_eq!(read.names(), ["tags"]);
    }

    #[test]
    fn a_name_saved_twice_still_reads_as_open() {
        assert_eq!(opened(&["tags", "tags"]).names(), ["tags"]);
    }

    #[test]
    fn each_repository_keeps_its_own_opened_groups() {
        let mut saved = fields(json!({ "/elsewhere": ["tags"] }));
        opened(&["stashes", "pull requests"]).write(&mut saved, Path::new(REPO));
        assert_eq!(
            saved,
            fields(json!({ "/elsewhere": ["tags"], REPO: ["pull requests", "stashes"] }))
        );
        assert_eq!(LeftPaneFolds::read(&saved, Path::new(REPO)).names(), ["pull requests", "stashes"]);
        assert_eq!(LeftPaneFolds::read(&saved, Path::new("/elsewhere")).names(), ["tags"]);
    }

    #[test]
    fn folding_everything_again_drops_the_repository_from_the_file() {
        let mut saved = fields(json!({ REPO: ["tags"], "/elsewhere": ["tags"] }));
        LeftPaneFolds::default().write(&mut saved, Path::new(REPO));
        assert_eq!(saved, fields(json!({ "/elsewhere": ["tags"] })));
    }

    #[test]
    fn a_repository_missing_from_the_file_or_saved_as_something_else_starts_folded() {
        let saved = fields(json!({ "/elsewhere": ["tags"], "/odd": "tags", "/mixed": [1, "tags"] }));
        assert!(LeftPaneFolds::read(&saved, Path::new(REPO)).names().is_empty());
        assert!(LeftPaneFolds::read(&saved, Path::new("/odd")).names().is_empty());
        assert_eq!(LeftPaneFolds::read(&saved, Path::new("/mixed")).names(), ["tags"]);
    }
}
