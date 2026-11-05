use std::ops::Range;

use crate::git::{Marked, Piece, Version, has_markers, split_conflicts};

const OPENING: &str = "<<<<<<<";
const BASE: &str = "|||||||";
const SEPARATOR: &str = "=======";
const CLOSING: &str = ">>>>>>>";
const LINES_ABOVE: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Taken {
    pub(crate) current: bool,
    pub(crate) incoming: bool,
}

impl Taken {
    const EACH: [Taken; 4] = [
        Taken {
            current: false,
            incoming: false,
        },
        Taken {
            current: true,
            incoming: false,
        },
        Taken {
            current: false,
            incoming: true,
        },
        Taken {
            current: true,
            incoming: true,
        },
    ];

    fn flipped(self, version: Version) -> Self {
        match version {
            Version::Current => Taken {
                current: !self.current,
                ..self
            },
            Version::Incoming => Taken {
                incoming: !self.incoming,
                ..self
            },
        }
    }

    fn is_nothing(self) -> bool {
        !self.current && !self.incoming
    }

    pub(crate) fn has(self, version: Version) -> bool {
        match version {
            Version::Current => self.current,
            Version::Incoming => self.incoming,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    Markers,
    Taken(Taken),
    ByHand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tint {
    Marker,
    Current,
    Incoming,
    ByHand,
}

#[derive(Clone, Debug)]
pub(crate) struct Region {
    range: Range<usize>,
    marked: Marked,
    shape: Shape,
}

impl Region {
    pub(crate) fn shape(&self) -> Shape {
        self.shape
    }

    pub(crate) fn labels(&self) -> (&str, &str) {
        (&self.marked.current_label, &self.marked.incoming_label)
    }

    pub(crate) fn taken(&self) -> Taken {
        match self.shape {
            Shape::Taken(taken) => taken,
            Shape::Markers | Shape::ByHand => Taken::default(),
        }
    }

    fn text_for(&self, taken: Taken) -> String {
        if taken.is_nothing() {
            return self.marked.block.clone();
        }
        let mut text = String::new();
        if taken.current {
            text.push_str(&self.marked.current);
        }
        if taken.incoming {
            text.push_str(&self.marked.incoming);
        }
        text
    }

    fn recognize(&mut self, text: &str) {
        self.shape = Taken::EACH
            .into_iter()
            .find(|taken| self.text_for(*taken) == text)
            .map_or(Shape::ByHand, |taken| {
                if taken.is_nothing() {
                    Shape::Markers
                } else {
                    Shape::Taken(taken)
                }
            });
    }

    fn tints(&self) -> Vec<(Range<usize>, Tint)> {
        let start = self.range.start;
        let shifted = |range: Range<usize>, tint| (range.start + start..range.end + start, tint);
        match self.shape {
            Shape::ByHand => vec![(self.range.clone(), Tint::ByHand)],
            Shape::Taken(taken) => {
                let current = if taken.current {
                    self.marked.current.len()
                } else {
                    0
                };
                let incoming = if taken.incoming {
                    self.marked.incoming.len()
                } else {
                    0
                };
                [
                    shifted(0..current, Tint::Current),
                    shifted(current..current + incoming, Tint::Incoming),
                ]
                .into_iter()
                .filter(|(range, _)| !range.is_empty())
                .collect()
            }
            Shape::Markers => block_tints(&self.marked.block)
                .into_iter()
                .map(|(range, tint)| shifted(range, tint))
                .collect(),
        }
    }
}

fn block_tints(block: &str) -> Vec<(Range<usize>, Tint)> {
    let mut tints = Vec::new();
    let mut tint = Tint::Current;
    let mut at = 0;
    for line in block.split_inclusive('\n') {
        let marker = [OPENING, BASE, SEPARATOR, CLOSING]
            .into_iter()
            .find(|marker| line.starts_with(marker));
        let line_tint = match marker {
            Some(BASE) => {
                tint = Tint::Marker;
                Tint::Marker
            }
            Some(SEPARATOR) => {
                tint = Tint::Incoming;
                Tint::Marker
            }
            Some(_) => Tint::Marker,
            None => tint,
        };
        tints.push((at..at + line.len(), line_tint));
        at += line.len();
    }
    tints
}

#[derive(Clone, Debug)]
pub(crate) struct MergeText {
    text: String,
    regions: Vec<Region>,
}

pub(crate) struct Replacement {
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

impl MergeText {
    pub(crate) fn new(text: &str) -> Option<Self> {
        let pieces = split_conflicts(text)?;
        let mut regions = Vec::new();
        let mut at = 0;
        for piece in pieces {
            match piece {
                Piece::Same(same) => at += same.len(),
                Piece::Conflict(marked) => {
                    let end = at + marked.block.len();
                    regions.push(Region {
                        range: at..end,
                        marked,
                        shape: Shape::Markers,
                    });
                    at = end;
                }
            }
        }
        Some(Self {
            text: text.to_owned(),
            regions,
        })
    }

    pub(crate) fn plain(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            regions: Vec::new(),
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn regions(&self) -> &[Region] {
        &self.regions
    }

    pub(crate) fn conflicts_left(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| has_markers(&self.text[region.range.clone()]))
            .count()
    }

    pub(crate) fn is_resolved(&self) -> bool {
        !has_markers(&self.text)
    }

    pub(crate) fn line_of(&self, region: usize) -> usize {
        self.regions.get(region).map_or(0, |region| {
            self.text[..region.range.start].matches('\n').count()
        })
    }

    pub(crate) fn tints(&self) -> Vec<(Range<usize>, Tint)> {
        self.regions.iter().flat_map(Region::tints).collect()
    }

    pub(crate) fn toggle(&mut self, region: usize, version: Version) -> Option<Replacement> {
        let chosen = self.regions.get(region)?;
        if chosen.shape == Shape::ByHand {
            return None;
        }
        let taken = chosen.taken().flipped(version);
        let replacement = Replacement {
            range: chosen.range.clone(),
            text: chosen.text_for(taken),
        };
        self.replace(replacement.range.clone(), &replacement.text);
        self.regions[region].shape = if taken.is_nothing() {
            Shape::Markers
        } else {
            Shape::Taken(taken)
        };
        Some(replacement)
    }

    pub(crate) fn edited(&mut self, text: &str) {
        if text == self.text {
            return;
        }
        let (start, old_end, new_end) = changed_span(&self.text, text);
        let delta = new_end as isize - old_end as isize;
        for region in &mut self.regions {
            let range = &mut region.range;
            if range.end <= start && start != range.start {
                continue;
            }
            if range.start >= old_end && !(range.start == start && old_end > start) {
                *range = shift(range.start, delta)..shift(range.end, delta);
                continue;
            }
            let end = if range.end >= old_end {
                shift(range.end, delta)
            } else {
                new_end
            };
            *range = range.start.min(start)..end.max(new_end);
        }
        self.text = text.to_owned();
        for region in &mut self.regions {
            let slice = self.text[region.range.clone()].to_owned();
            region.recognize(&slice);
        }
    }

    fn replace(&mut self, range: Range<usize>, with: &str) {
        let delta = with.len() as isize - range.len() as isize;
        self.text.replace_range(range.clone(), with);
        for region in &mut self.regions {
            if region.range == range {
                region.range = range.start..range.start + with.len();
            } else if region.range.start >= range.end {
                region.range = shift(region.range.start, delta)..shift(region.range.end, delta);
            }
        }
    }
}

fn shift(at: usize, delta: isize) -> usize {
    at.saturating_add_signed(delta)
}

fn changed_span(old: &str, new: &str) -> (usize, usize, usize) {
    let mut prefix = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(prefix) || !new.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let room = old.len().min(new.len()) - prefix;
    let mut suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - suffix) || !new.is_char_boundary(new.len() - suffix) {
        suffix -= 1;
    }
    (prefix, old.len() - suffix, new.len() - suffix)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Versions {
    pub(crate) patch: String,
    pub(crate) spans: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) current: Option<(usize, usize)>,
    pub(crate) incoming: Option<(usize, usize)>,
}

struct Hunk {
    current_start: usize,
    incoming_start: usize,
    current_count: usize,
    incoming_count: usize,
    lines: String,
}

impl Hunk {
    fn header(&self) -> String {
        let start = |start: usize, count: usize| if count == 0 { start - 1 } else { start };
        format!(
            "@@ -{},{} +{},{} @@\n",
            start(self.current_start, self.current_count),
            self.current_count,
            start(self.incoming_start, self.incoming_count),
            self.incoming_count
        )
    }

    fn push(&mut self, sign: char, line: &str) {
        self.lines.push(sign);
        self.lines.push_str(line.trim_end_matches(['\n', '\r']));
        self.lines.push('\n');
        match sign {
            '-' => self.current_count += 1,
            '+' => self.incoming_count += 1,
            _ => {
                self.current_count += 1;
                self.incoming_count += 1;
            }
        }
    }
}

pub(crate) fn versions(path: &str, text: &str) -> Option<Versions> {
    let pieces = split_conflicts(text)?;
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut spans = Vec::new();
    let mut shared: Vec<&str> = Vec::new();
    let (mut current_line, mut incoming_line) = (1, 1);
    for piece in &pieces {
        match piece {
            Piece::Same(same) => shared.extend(same.split_inclusive('\n')),
            Piece::Conflict(marked) => {
                let above = if hunks.is_empty() { shared.len() } else { LINES_ABOVE };
                let (before, lead) = shared.split_at(shared.len().saturating_sub(above));
                if let Some(previous) = hunks.last_mut() {
                    for line in before {
                        previous.push(' ', line);
                    }
                }
                current_line += before.len();
                incoming_line += before.len();
                let mut hunk = Hunk {
                    current_start: current_line,
                    incoming_start: incoming_line,
                    current_count: 0,
                    incoming_count: 0,
                    lines: String::new(),
                };
                for line in lead {
                    hunk.push(' ', line);
                }
                current_line += lead.len();
                incoming_line += lead.len();
                let current = marked.current.split_inclusive('\n').count();
                let incoming = marked.incoming.split_inclusive('\n').count();
                let span = |start: usize, count: usize| (count > 0).then(|| (start, start + count - 1));
                spans.push(Span {
                    current: span(current_line, current),
                    incoming: span(incoming_line, incoming),
                });
                for line in marked.current.split_inclusive('\n') {
                    hunk.push('-', line);
                }
                for line in marked.incoming.split_inclusive('\n') {
                    hunk.push('+', line);
                }
                current_line += current;
                incoming_line += incoming;
                hunks.push(hunk);
                shared.clear();
            }
        }
    }
    if let Some(last) = hunks.last_mut() {
        for line in &shared {
            last.push(' ', line);
        }
    }
    let mut patch = format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n");
    for hunk in &hunks {
        patch.push_str(&hunk.header());
        patch.push_str(&hunk.lines);
    }
    Some(Versions { patch, spans })
}

#[cfg(test)]
mod tests {
    use super::{MergeText, Shape, Span, Taken, Tint, versions};
    use crate::git::Version;

    const FILE: &str = "start\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> 3f2a91c (Fix)\nmiddle\n<<<<<<< HEAD\na\n=======\nb\nc\n>>>>>>> 3f2a91c (Fix)\nend\n";

    #[test]
    fn ticking_a_side_puts_its_lines_in_place_of_the_markers() {
        let mut merge = MergeText::new(FILE).expect("two conflicts");
        assert_eq!(merge.conflicts_left(), 2);
        let first = merge.toggle(0, Version::Incoming).expect("a choice");
        assert_eq!(first.range, 6..61);
        assert_eq!(first.text, "theirs\n");
        let both = merge.toggle(0, Version::Current).expect("a choice");
        assert_eq!(both.text, "ours\ntheirs\n");
        assert_eq!(
            merge.regions()[0].shape(),
            Shape::Taken(Taken {
                current: true,
                incoming: true
            })
        );
        merge.toggle(1, Version::Current);
        assert_eq!(merge.text(), "start\nours\ntheirs\nmiddle\na\nend\n");
        assert!(merge.is_resolved());
        merge.toggle(1, Version::Current);
        assert_eq!(merge.conflicts_left(), 1);
        assert_eq!(merge.line_of(1), 4);
    }

    #[test]
    fn typing_in_a_conflict_makes_it_hand_made_and_typing_elsewhere_moves_it() {
        let mut merge = MergeText::new(FILE).expect("two conflicts");
        merge.toggle(0, Version::Current);
        let typed = merge.text().replacen("start", "begin here", 1);
        merge.edited(&typed);
        assert_eq!(merge.regions()[0].shape(), Shape::Taken(Taken { current: true, incoming: false }));
        let typed = merge.text().replacen("ours", "ours, then mine", 1);
        merge.edited(&typed);
        assert_eq!(merge.regions()[0].shape(), Shape::ByHand);
        assert_eq!(merge.toggle(0, Version::Incoming).map(|edit| edit.text), None);
        let undone = merge.text().replacen("ours, then mine", "ours", 1);
        merge.edited(&undone);
        assert_eq!(merge.regions()[0].shape(), Shape::Taken(Taken { current: true, incoming: false }));
        let markers_gone = merge.text().replace("<<<<<<< HEAD\na\n=======\nb\nc\n>>>>>>> 3f2a91c (Fix)\n", "a and b\n");
        merge.edited(&markers_gone);
        assert_eq!(merge.regions()[1].shape(), Shape::ByHand);
        assert!(merge.is_resolved());
        assert_eq!(merge.text(), "begin here\nours\nmiddle\na and b\nend\n");
    }

    #[test]
    fn each_part_of_a_conflict_gets_its_color() {
        let mut merge = MergeText::new("<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n").expect("a conflict");
        assert_eq!(
            merge.tints(),
            [
                (0..10, Tint::Marker),
                (10..12, Tint::Current),
                (12..20, Tint::Marker),
                (20..22, Tint::Incoming),
                (22..32, Tint::Marker),
            ]
        );
        merge.toggle(0, Version::Incoming);
        merge.toggle(0, Version::Current);
        assert_eq!(merge.tints(), [(0..2, Tint::Current), (2..4, Tint::Incoming)]);
    }

    #[test]
    fn each_hunk_after_the_first_starts_three_lines_above_its_conflict() {
        let numbered = |from: usize, to: usize| -> String {
            (from..=to).map(|n| format!("{n}\n")).collect()
        };
        let text = format!(
            "{}<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n{}<<<<<<< a\nz\n=======\n>>>>>>> b\n",
            numbered(1, 10),
            numbered(12, 19)
        );
        let versions = versions("n.txt", &text).expect("two conflicts");
        let headers: Vec<&str> = versions
            .patch
            .lines()
            .filter(|line| line.starts_with("@@"))
            .collect();
        assert_eq!(headers, ["@@ -1,16 +1,16 @@", "@@ -17,4 +17,3 @@"]);
        assert_eq!(
            versions.spans,
            [
                Span {
                    current: Some((11, 11)),
                    incoming: Some((11, 11))
                },
                Span {
                    current: Some((20, 20)),
                    incoming: None
                },
            ]
        );
    }

    #[test]
    fn the_two_versions_line_up_with_one_hunk_per_conflict() {
        let versions = versions("f.txt", FILE).expect("two conflicts");
        assert_eq!(
            versions.patch,
            "diff --git a/f.txt b/f.txt\n--- a/f.txt\n+++ b/f.txt\n\
             @@ -1,2 +1,2 @@\n start\n-ours\n+theirs\n\
             @@ -3,3 +3,4 @@\n middle\n-a\n+b\n+c\n end\n"
        );
        assert_eq!(
            versions.spans,
            [
                Span {
                    current: Some((2, 2)),
                    incoming: Some((2, 2))
                },
                Span {
                    current: Some((4, 4)),
                    incoming: Some((4, 5))
                },
            ]
        );
    }
}
