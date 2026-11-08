use gpui_kit::SharedString;

use crate::{
    git::{RefKind, RefLabel},
    ui::Folded,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Section {
    Refs(RefKind),
    Worktrees,
    Stashes,
}

impl Section {
    pub(super) const IN_ORDER: [Section; 5] = [
        Section::Refs(RefKind::LocalBranch),
        Section::Worktrees,
        Section::Refs(RefKind::RemoteBranch),
        Section::Stashes,
        Section::Refs(RefKind::Tag),
    ];
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RefGroup {
    Section(Section),
    Folder(RefKind, SharedString),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RefTreeRow {
    Section {
        section: Section,
        count: usize,
        open: bool,
    },
    Folder {
        kind: RefKind,
        path: SharedString,
        depth: usize,
        open: bool,
    },
    Ref {
        ix: usize,
        depth: usize,
    },
    Stash {
        ix: usize,
    },
    Worktree {
        ix: usize,
    },
}

pub(crate) struct Listed {
    pub(crate) stashes: usize,
    pub(crate) worktrees: usize,
}

pub(crate) fn ref_tree(
    refs: &[RefLabel],
    listed: Listed,
    folded: &Folded<RefGroup>,
) -> Vec<RefTreeRow> {
    let mut rows = Vec::new();
    let mut first = 0;
    for section in Section::IN_ORDER {
        let start = first;
        let count = match section {
            Section::Refs(kind) => refs[start..].partition_point(|label| label.kind == kind),
            Section::Worktrees => listed.worktrees,
            Section::Stashes => listed.stashes,
        };
        if count == 0 {
            continue;
        }
        let open = folded.is_open(&RefGroup::Section(section));
        rows.push(RefTreeRow::Section {
            section,
            count,
            open,
        });
        match section {
            Section::Refs(kind) => {
                first += count;
                if open {
                    add_section(&mut rows, kind, &refs[start..first], start, folded);
                }
            }
            Section::Stashes if open => {
                rows.extend((0..listed.stashes).map(|ix| RefTreeRow::Stash { ix }))
            }
            Section::Worktrees if open => {
                rows.extend((0..listed.worktrees).map(|ix| RefTreeRow::Worktree { ix }))
            }
            Section::Stashes | Section::Worktrees => {}
        }
    }
    rows
}

fn add_section(
    rows: &mut Vec<RefTreeRow>,
    kind: RefKind,
    refs: &[RefLabel],
    first: usize,
    folded: &Folded<RefGroup>,
) {
    let mut folders: Vec<(&str, bool)> = Vec::new();
    for (offset, label) in refs.iter().enumerate() {
        let parents = folder_paths(&label.name);
        let shared = folders
            .iter()
            .zip(parents.clone())
            .take_while(|((folder, _), parent)| folder == parent)
            .count();
        folders.truncate(shared);
        for parent in parents.skip(shared) {
            let open = if folders.iter().all(|(_, open)| *open) {
                add_folder(rows, kind, parent, folders.len(), folded)
            } else {
                false
            };
            folders.push((parent, open));
        }
        if folders.iter().all(|(_, open)| *open) {
            rows.push(RefTreeRow::Ref {
                ix: first + offset,
                depth: folders.len(),
            });
        }
    }
}

fn add_folder(
    rows: &mut Vec<RefTreeRow>,
    kind: RefKind,
    path: &str,
    depth: usize,
    folded: &Folded<RefGroup>,
) -> bool {
    let path = SharedString::from(path.to_owned());
    let open = folded.is_open(&RefGroup::Folder(kind, path.clone()));
    rows.push(RefTreeRow::Folder {
        kind,
        path,
        depth,
        open,
    });
    open
}

fn folder_paths(name: &str) -> impl Iterator<Item = &str> + Clone {
    name.match_indices('/').map(|(end, _)| &name[..end])
}

pub(crate) fn last_segment(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, last)| last)
}

#[cfg(test)]
mod tests {
    use gpui_kit::SharedString;

    use super::{Listed, RefGroup, RefTreeRow, Section, last_segment, ref_tree};
    use crate::{
        git::{
            RefKind::{self, LocalBranch, RemoteBranch, Tag},
            RefLabel,
            fixtures::id,
        },
        ui::Folded,
    };

    fn labels(names: &[(&str, RefKind)]) -> Vec<RefLabel> {
        names
            .iter()
            .map(|&(name, kind)| RefLabel {
                name: name.to_owned().into(),
                kind,
                target: id(1),
                is_head: false,
                upstream: None,
                from_upstream: None,
                worktree: None,
            })
            .collect()
    }

    fn section(kind: RefKind, count: usize, open: bool) -> RefTreeRow {
        RefTreeRow::Section {
            section: Section::Refs(kind),
            count,
            open,
        }
    }

    fn stashes(count: usize, open: bool) -> RefTreeRow {
        RefTreeRow::Section {
            section: Section::Stashes,
            count,
            open,
        }
    }

    fn stash(ix: usize) -> RefTreeRow {
        RefTreeRow::Stash { ix }
    }

    fn folder(kind: RefKind, path: &str, depth: usize, open: bool) -> RefTreeRow {
        RefTreeRow::Folder {
            kind,
            path: SharedString::from(path.to_owned()),
            depth,
            open,
        }
    }

    fn leaf(ix: usize, depth: usize) -> RefTreeRow {
        RefTreeRow::Ref { ix, depth }
    }

    fn folded(groups: impl IntoIterator<Item = RefGroup>) -> Folded<RefGroup> {
        let mut folded = Folded::default();
        for group in groups {
            folded.toggle(group);
        }
        folded
    }

    fn sample() -> Vec<RefLabel> {
        labels(&[
            ("feature/blame", LocalBranch),
            ("feature/diff-view", LocalBranch),
            ("main", LocalBranch),
            ("topic/tabs", LocalBranch),
            ("origin/feature/diff-view", RemoteBranch),
            ("origin/main", RemoteBranch),
            ("v1", Tag),
        ])
    }

    fn listing(stashes: usize) -> Listed {
        Listed {
            stashes,
            worktrees: 0,
        }
    }

    #[test]
    fn names_split_into_folders_at_each_slash() {
        assert_eq!(
            ref_tree(&sample(), listing(0), &Folded::default()),
            [
                section(LocalBranch, 4, true),
                folder(LocalBranch, "feature", 0, true),
                leaf(0, 1),
                leaf(1, 1),
                leaf(2, 0),
                folder(LocalBranch, "topic", 0, true),
                leaf(3, 1),
                section(RemoteBranch, 2, true),
                folder(RemoteBranch, "origin", 0, true),
                folder(RemoteBranch, "origin/feature", 1, true),
                leaf(4, 2),
                leaf(5, 1),
                section(Tag, 1, true),
                leaf(6, 0),
            ]
        );
    }

    #[test]
    fn a_folded_section_keeps_its_header_and_count() {
        let folded = folded([
            RefGroup::Section(Section::Refs(LocalBranch)),
            RefGroup::Section(Section::Refs(Tag)),
        ]);
        assert_eq!(
            ref_tree(&sample(), listing(0), &folded),
            [
                section(LocalBranch, 4, false),
                section(RemoteBranch, 2, true),
                folder(RemoteBranch, "origin", 0, true),
                folder(RemoteBranch, "origin/feature", 1, true),
                leaf(4, 2),
                leaf(5, 1),
                section(Tag, 1, false),
            ]
        );
    }

    #[test]
    fn a_folded_folder_hides_everything_inside_it_and_nothing_after_it() {
        let folded = folded([RefGroup::Folder(RemoteBranch, "origin".into())]);
        let rows = ref_tree(&sample(), listing(0), &folded);
        assert_eq!(
            rows[7..],
            [
                section(RemoteBranch, 2, true),
                folder(RemoteBranch, "origin", 0, false),
                section(Tag, 1, true),
                leaf(6, 0),
            ]
        );
    }

    #[test]
    fn folding_twice_opens_again() {
        let group = RefGroup::Folder(LocalBranch, "feature".into());
        let folded = folded([group.clone(), group]);
        assert_eq!(
            ref_tree(&sample(), listing(0), &folded),
            ref_tree(&sample(), listing(0), &Folded::default())
        );
    }

    #[test]
    fn stashes_sit_between_remotes_and_tags_without_folders() {
        assert_eq!(
            ref_tree(&sample(), listing(2), &Folded::default())[12..],
            [
                stashes(2, true),
                stash(0),
                stash(1),
                section(Tag, 1, true),
                leaf(6, 0),
            ]
        );
    }

    #[test]
    fn folded_stashes_keep_their_header_and_count() {
        let folded = folded([RefGroup::Section(Section::Stashes)]);
        assert_eq!(
            ref_tree(&sample(), listing(2), &folded)[12..],
            [stashes(2, false), section(Tag, 1, true), leaf(6, 0)]
        );
    }

    #[test]
    fn names_that_only_start_alike_stay_apart() {
        let refs = labels(&[
            ("fix-ci", LocalBranch),
            ("fix/a", LocalBranch),
            ("fix/b/c", LocalBranch),
            ("fixes", LocalBranch),
        ]);
        assert_eq!(
            ref_tree(&refs, listing(0), &Folded::default()),
            [
                section(LocalBranch, 4, true),
                leaf(0, 0),
                folder(LocalBranch, "fix", 0, true),
                leaf(1, 1),
                folder(LocalBranch, "fix/b", 1, true),
                leaf(2, 2),
                leaf(3, 0),
            ]
        );
    }

    #[test]
    fn rows_show_the_last_part_of_a_name() {
        assert_eq!(last_segment("origin/feature/diff-view"), "diff-view");
        assert_eq!(last_segment("main"), "main");
    }
}
