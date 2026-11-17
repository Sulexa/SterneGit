use std::{collections::HashMap, ops::Range, rc::Rc};

use super::{DiffFile, DiffSide};

/// Finds the byte ranges a search matches in one line of source text, without
/// its line ending.
pub type DiffMatcher = Rc<dyn Fn(&str) -> Vec<Range<usize>>>;

/// One run of text the search matched. A line both sides share is one result
/// shown on both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Hit {
    pub(crate) file: usize,
    pub(crate) original: Option<usize>,
    pub(crate) modified: Option<usize>,
    pub(crate) range: Range<usize>,
}

impl Hit {
    /// The side and line index the result is read from: the new side when
    /// both sides show it.
    pub(crate) fn line(&self) -> (DiffSide, usize) {
        match (self.modified, self.original) {
            (Some(ix), _) => (DiffSide::Modified, ix),
            (None, ix) => (DiffSide::Original, ix.unwrap_or_default()),
        }
    }

    fn order(&self) -> (usize, usize, bool, usize) {
        let (side, ix) = self.line();
        (self.file, ix, side == DiffSide::Original, self.range.start)
    }
}

/// Every result of a search, in patch order: within a run of changes, the
/// removed lines' results come before the added lines'.
#[derive(Default)]
pub(crate) struct Found {
    hits: Vec<Hit>,
    lines: HashMap<(usize, DiffSide, usize), Vec<usize>>,
}

impl Found {
    pub(crate) fn new(files: &[DiffFile], matcher: &DiffMatcher) -> Self {
        let mut found = Self::default();
        for (file_ix, file) in files.iter().enumerate() {
            for line in file.patch_lines() {
                let original = line.line(DiffSide::Original);
                let modified = line.line(DiffSide::Modified);
                let (side, ix) = match (modified, original) {
                    (Some(ix), _) => (DiffSide::Modified, ix),
                    (None, Some(ix)) => (DiffSide::Original, ix),
                    (None, None) => continue,
                };
                for range in matcher(file.lines(side)[ix].text()) {
                    if range.is_empty() {
                        continue;
                    }
                    let hit = found.hits.len();
                    for (side, line) in [(DiffSide::Original, original), (DiffSide::Modified, modified)] {
                        if let Some(line) = line {
                            found.lines.entry((file_ix, side, line)).or_default().push(hit);
                        }
                    }
                    found.hits.push(Hit {
                        file: file_ix,
                        original,
                        modified,
                        range,
                    });
                }
            }
        }
        found
    }

    pub(crate) fn hits(&self) -> &[Hit] {
        &self.hits
    }

    /// The results on one line: each one's source range and index.
    pub(crate) fn on_line(
        &self,
        file: usize,
        side: DiffSide,
        ix: usize,
    ) -> impl Iterator<Item = (Range<usize>, usize)> + '_ {
        self.lines
            .get(&(file, side, ix))
            .into_iter()
            .flatten()
            .map(|hit| (self.hits[*hit].range.clone(), *hit))
    }

    /// The result standing where `hit` stood before the search ran again:
    /// the same one when it is still there, else the next, else the last.
    pub(crate) fn nearest(&self, hit: &Hit) -> Option<usize> {
        let wanted = hit.order();
        self.hits
            .iter()
            .position(|candidate| candidate.order() >= wanted)
            .or_else(|| self.hits.len().checked_sub(1))
    }
}
