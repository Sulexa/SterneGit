use std::{collections::HashMap, ops::Range};

use gix::ObjectId;
use gpui_kit::SharedString;

use crate::{
    git::History,
    ui::{Matcher, Search},
};

const SHORTEST_ID: usize = 4;
const SHOWN_ID: usize = 7;

pub(crate) type Descriptions = HashMap<ObjectId, SharedString>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unseen {
    Description,
    Email,
    Id,
}

impl Unseen {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Unseen::Description => "in the description",
            Unseen::Email => "in the email",
            Unseen::Id => "in the full id",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    Shown,
    Unseen(Unseen),
}

pub(crate) struct CommitText<'a> {
    pub(crate) subject: &'a str,
    pub(crate) author: &'a str,
    pub(crate) email: &'a str,
    pub(crate) id: &'a str,
    pub(crate) refs: Vec<&'a str>,
    pub(crate) description: Option<&'a str>,
}

enum IdMatch {
    Prefix(String),
    Pattern,
    Never,
}

pub(crate) struct CommitFinder {
    matcher: Matcher,
    id: IdMatch,
}

impl CommitFinder {
    pub(crate) fn new(search: &Search) -> Option<Self> {
        let matcher = search.matcher().ok().flatten()?;
        let typed = search.text.trim();
        let id = if search.options.pattern {
            IdMatch::Pattern
        } else if typed.len() >= SHORTEST_ID && typed.chars().all(|c| c.is_ascii_hexdigit()) {
            IdMatch::Prefix(typed.to_ascii_lowercase())
        } else {
            IdMatch::Never
        };
        Some(Self { matcher, id })
    }

    pub(crate) fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        self.matcher.ranges(text)
    }

    pub(crate) fn shown_id_ranges(&self, id: &str) -> Vec<Range<usize>> {
        let shown = id.len().min(SHOWN_ID);
        match &self.id {
            IdMatch::Prefix(prefix) if id.starts_with(prefix.as_str()) => vec![0..prefix.len().min(shown)],
            IdMatch::Pattern => self
                .matcher
                .ranges(id)
                .into_iter()
                .filter(|range| range.start < shown)
                .map(|range| range.start..range.end.min(shown))
                .collect(),
            IdMatch::Prefix(_) | IdMatch::Never => Vec::new(),
        }
    }

    fn finds_id(&self, id: &str) -> bool {
        match &self.id {
            IdMatch::Prefix(prefix) => id.starts_with(prefix.as_str()),
            IdMatch::Pattern => self.matcher.finds(id),
            IdMatch::Never => false,
        }
    }

    pub(crate) fn place(&self, commit: &CommitText<'_>) -> Option<Place> {
        let shown = [commit.subject, commit.author]
            .into_iter()
            .chain(commit.refs.iter().copied())
            .any(|text| self.matcher.finds(text))
            || !self.shown_id_ranges(commit.id).is_empty();
        if shown {
            return Some(Place::Shown);
        }
        if commit.description.is_some_and(|description| self.matcher.finds(description)) {
            return Some(Place::Unseen(Unseen::Description));
        }
        if self.matcher.finds(commit.email) {
            return Some(Place::Unseen(Unseen::Email));
        }
        self.finds_id(commit.id).then_some(Place::Unseen(Unseen::Id))
    }
}

#[derive(Default)]
pub(crate) struct CommitsFound {
    pub(crate) rows: Vec<usize>,
    pub(crate) unseen: HashMap<usize, Unseen>,
}

impl CommitsFound {
    pub(crate) fn has(&self, row: usize) -> bool {
        self.rows.binary_search(&row).is_ok()
    }

    pub(crate) fn index_of(&self, row: usize) -> Option<usize> {
        self.rows.binary_search(&row).ok()
    }

    pub(crate) fn first_from(&self, row: usize) -> Option<usize> {
        let at = self.rows.partition_point(|found| *found < row);
        (at < self.rows.len()).then_some(at).or((!self.rows.is_empty()).then_some(0))
    }

    pub(crate) fn step_from(&self, row: usize, forward: bool) -> Option<usize> {
        let count = self.rows.len();
        if count == 0 {
            return None;
        }
        Some(match (self.index_of(row), forward) {
            (Some(at), true) => (at + 1) % count,
            (Some(at), false) => (at + count - 1) % count,
            (None, true) => self.first_from(row).unwrap_or(0),
            (None, false) => self
                .rows
                .partition_point(|found| *found < row)
                .checked_sub(1)
                .unwrap_or(count - 1),
        })
    }
}

pub(crate) fn find_commits(
    history: &History,
    finder: &CommitFinder,
    descriptions: &Descriptions,
) -> CommitsFound {
    let mut found = CommitsFound::default();
    for (row, commit) in history.commits.iter().enumerate() {
        if history.worktree_on_row(row).is_some() {
            continue;
        }
        let id = commit.id.to_string();
        let stash = history.stash_on(&commit.id).map(|stash| stash.name.as_str());
        let worktrees = history.detached_worktrees_on(&commit.id).map(|worktree| worktree.name.as_str());
        let refs = history
            .labels_on(&commit.id)
            .map(|label| label.name.as_str())
            .chain(stash)
            .chain(worktrees)
            .collect();
        let text = CommitText {
            subject: &commit.subject,
            author: &commit.author,
            email: &commit.author_email,
            id: &id,
            refs,
            description: descriptions
                .get(&commit.id)
                .map(SharedString::as_str)
                .filter(|description| !description.is_empty()),
        };
        match finder.place(&text) {
            Some(Place::Shown) => found.rows.push(row),
            Some(Place::Unseen(unseen)) => {
                found.rows.push(row);
                found.unseen.insert(row, unseen);
            }
            None => {}
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::SearchOptions;

    const ID: &str = "8ff3473ab1c2d3e4f5061728394a5b6c7d8e9f01";

    fn finder(text: &str, options: SearchOptions) -> CommitFinder {
        CommitFinder::new(&Search {
            text: text.into(),
            options,
        })
        .unwrap()
    }

    fn commit<'a>(description: Option<&'a str>) -> CommitText<'a> {
        CommitText {
            subject: "Render side-by-side diffs",
            author: "Sam Lee",
            email: "sam@sternegit.dev",
            id: ID,
            refs: vec!["feature/diff-view", "origin/feature/diff-view"],
            description,
        }
    }

    #[test]
    fn a_match_in_the_row_is_shown() {
        for text in ["side", "diff-view", "sam"] {
            assert_eq!(finder(text, SearchOptions::default()).place(&commit(None)), Some(Place::Shown));
        }
    }

    #[test]
    fn a_match_the_row_hides_says_where_it_is() {
        let commit = commit(Some("Uses the kit's split view"));
        assert_eq!(
            finder("kit", SearchOptions::default()).place(&commit),
            Some(Place::Unseen(Unseen::Description))
        );
        assert_eq!(
            finder("sternegit.dev", SearchOptions::default()).place(&commit),
            Some(Place::Unseen(Unseen::Email))
        );
        assert_eq!(finder("parser", SearchOptions::default()).place(&commit), None);
    }

    #[test]
    fn ids_match_from_their_first_characters() {
        let prefix = finder("8FF34", SearchOptions::default());
        assert_eq!(prefix.place(&commit(None)), Some(Place::Shown));
        assert_eq!(prefix.shown_id_ranges(ID), [0..5]);
        assert_eq!(finder("ab1c", SearchOptions::default()).place(&commit(None)), None);
    }

    #[test]
    fn a_long_id_prefix_lights_the_whole_short_id() {
        let prefix = finder("8ff3473ab1", SearchOptions::default());
        assert_eq!(prefix.shown_id_ranges(ID), [0..7]);
    }

    #[test]
    fn short_words_never_match_ids() {
        assert_eq!(finder("8ff", SearchOptions::default()).place(&commit(None)), None);
    }

    #[test]
    fn a_pattern_matching_past_the_short_id_is_unseen() {
        let options = SearchOptions {
            pattern: true,
            ..Default::default()
        };
        assert_eq!(
            finder("9f01$", options).place(&commit(None)),
            Some(Place::Unseen(Unseen::Id))
        );
    }

    #[test]
    fn results_step_from_a_row() {
        let found = CommitsFound {
            rows: vec![2, 5, 9],
            unseen: HashMap::new(),
        };
        assert_eq!(found.first_from(0), Some(0));
        assert_eq!(found.first_from(3), Some(1));
        assert_eq!(found.first_from(10), Some(0));
        assert!(found.has(5));
        assert_eq!(found.index_of(9), Some(2));
        assert_eq!(CommitsFound::default().first_from(0), None);
    }

    #[test]
    fn steps_wrap_and_start_beside_a_row_that_is_not_a_result() {
        let found = CommitsFound {
            rows: vec![2, 5, 9],
            unseen: HashMap::new(),
        };
        assert_eq!(found.step_from(9, true), Some(0));
        assert_eq!(found.step_from(2, false), Some(2));
        assert_eq!(found.step_from(6, true), Some(2));
        assert_eq!(found.step_from(6, false), Some(1));
        assert_eq!(found.step_from(0, false), Some(2));
        assert_eq!(found.step_from(0, true), Some(0));
        assert_eq!(CommitsFound::default().step_from(1, true), None);
    }
}
