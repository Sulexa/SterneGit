use std::collections::HashMap;

const PATH: &str = "{path}";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SideFiles(Vec<String>);

impl SideFiles {
    pub(crate) fn new(patterns: impl IntoIterator<Item = String>) -> Self {
        let mut patterns: Vec<String> = patterns
            .into_iter()
            .filter(|pattern| pattern.contains(PATH) && pattern != PATH)
            .collect();
        patterns.sort();
        patterns.dedup();
        Self(patterns)
    }

    fn of<'a>(&'a self, path: &'a str) -> impl Iterator<Item = String> + 'a {
        self.0.iter().map(move |pattern| pattern.replace(PATH, path))
    }

    pub(crate) fn owners(&self, paths: &[&str]) -> Vec<Option<usize>> {
        let at: HashMap<&str, usize> = paths.iter().enumerate().map(|(ix, path)| (*path, ix)).collect();
        let mut owners = vec![None; paths.len()];
        for (ix, path) in paths.iter().enumerate() {
            for side in self.of(path) {
                if let Some(&side) = at.get(side.as_str())
                    && side != ix
                    && owners[side].is_none()
                {
                    owners[side] = Some(ix);
                }
            }
        }
        let folded_into_a_side_file: Vec<usize> = (0..paths.len())
            .filter(|&side| owners[side].is_some_and(|owner| owners[owner].is_some()))
            .collect();
        for side in folded_into_a_side_file {
            owners[side] = None;
        }
        owners
    }

    pub(crate) fn listed(&self, paths: &[&str]) -> Vec<usize> {
        let owners = self.owners(paths);
        (0..paths.len()).filter(|&ix| owners[ix].is_none()).collect()
    }

    pub(crate) fn sides_of(&self, paths: &[&str], owner: usize) -> Vec<usize> {
        let owners = self.owners(paths);
        (0..paths.len()).filter(|&ix| owners[ix] == Some(owner)).collect()
    }
}

pub(crate) fn side_label<'a>(owner: &str, side: &'a str) -> &'a str {
    let name = side.rsplit('/').next().unwrap_or(side);
    let owner_name = owner.rsplit('/').next().unwrap_or(owner);
    name.strip_prefix(owner_name)
        .map(|rest| rest.trim_start_matches('.'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::{SideFiles, side_label};

    fn unity() -> SideFiles {
        SideFiles::new(["{path}.meta".to_owned()])
    }

    #[test]
    fn a_side_file_folds_into_its_file_when_both_are_listed() {
        let paths = ["Popup.prefab", "Popup.prefab.meta", "Icon.png"];
        assert_eq!(unity().listed(&paths), vec![0, 2]);
        assert_eq!(unity().sides_of(&paths, 0), vec![1]);
    }

    #[test]
    fn a_side_file_whose_file_is_not_listed_keeps_its_row() {
        assert_eq!(unity().listed(&["Accept.asset.meta", "Decline.asset", "Folder.meta"]), vec![0, 1, 2]);
    }

    #[test]
    fn a_side_file_listed_before_its_file_still_folds() {
        assert_eq!(unity().listed(&["A/Hero.prefab.meta", "A/Hero.prefab"]), vec![1]);
    }

    #[test]
    fn without_a_reader_nothing_folds() {
        let paths = ["Popup.prefab", "Popup.prefab.meta"];
        assert_eq!(SideFiles::default().listed(&paths), vec![0, 1]);
    }

    #[test]
    fn several_patterns_fold_several_side_files() {
        let sides = SideFiles::new(["{path}.meta".to_owned(), "{path}.import".to_owned()]);
        let paths = ["icon.png", "icon.png.import", "icon.png.meta"];
        assert_eq!(sides.listed(&paths), vec![0]);
        assert_eq!(sides.sides_of(&paths, 0), vec![1, 2]);
    }

    #[test]
    fn a_side_file_of_a_side_file_keeps_its_row() {
        assert_eq!(unity().listed(&["a.png", "a.png.meta", "a.png.meta.meta"]), vec![0, 2]);
    }

    #[test]
    fn a_pattern_without_the_path_is_ignored() {
        assert_eq!(SideFiles::new(["{path}".to_owned(), "README".to_owned()]), SideFiles::default());
    }

    #[test]
    fn a_side_file_is_labelled_by_what_it_adds_to_the_name() {
        assert_eq!(side_label("Assets/slime.png", "Assets/slime.png.meta"), "meta");
        assert_eq!(side_label("icon.png", "icon.png.import"), "import");
    }
}
