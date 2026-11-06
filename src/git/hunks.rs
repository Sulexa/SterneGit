use std::collections::BTreeSet;

use anyhow::Result;

use super::{Repo, command::run_with_input};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineSide {
    Old,
    New,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineAt {
    pub(crate) side: LineSide,
    pub(crate) line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LineSpan {
    pub(crate) from: LineAt,
    pub(crate) to: LineAt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LineRef {
    hunk: usize,
    line: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Part(BTreeSet<LineRef>);

impl Part {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn in_hunk(&self, hunk: usize) -> Part {
        Part(
            self.0
                .iter()
                .filter(|at| at.hunk == hunk)
                .copied()
                .collect(),
        )
    }

    fn touches(&self, hunk: usize) -> bool {
        self.0.iter().any(|at| at.hunk == hunk)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PartAction {
    Stage,
    Unstage,
    Discard,
}

impl PartAction {
    fn args(self) -> &'static [&'static str] {
        match self {
            PartAction::Stage => &["apply", "--cached", "--recount", "--whitespace=nowarn", "-"],
            PartAction::Unstage => &[
                "apply",
                "--cached",
                "--reverse",
                "--recount",
                "--whitespace=nowarn",
                "-",
            ],
            PartAction::Discard => &[
                "apply",
                "--reverse",
                "--recount",
                "--whitespace=nowarn",
                "-",
            ],
        }
    }

    fn reverses(self) -> bool {
        self != PartAction::Stage
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mark {
    Context,
    Added,
    Deleted,
}

impl Mark {
    fn byte(self) -> u8 {
        match self {
            Mark::Context => b' ',
            Mark::Added => b'+',
            Mark::Deleted => b'-',
        }
    }

    fn is_change(self) -> bool {
        self != Mark::Context
    }
}

#[derive(Clone, Debug)]
struct Line {
    mark: Mark,
    text: Vec<u8>,
    old: Option<usize>,
    new: Option<usize>,
    missing_newline: Option<Vec<u8>>,
}

impl Line {
    fn number(&self, side: LineSide) -> Option<usize> {
        match side {
            LineSide::Old => self.old,
            LineSide::New => self.new,
        }
    }

    fn is_at(&self, at: LineAt) -> bool {
        self.number(at.side) == Some(at.line)
    }
}

#[derive(Clone, Debug)]
struct Hunk {
    old_start: usize,
    new_start: usize,
    scope: Vec<u8>,
    lines: Vec<Line>,
}

impl Hunk {
    fn open(header: &[u8]) -> Option<Self> {
        let rest = header.strip_prefix(b"@@ -")?;
        let (old, rest) = split_at_byte(rest, b' ')?;
        let rest = rest.strip_prefix(b"+")?;
        let (new, rest) = split_at_byte(rest, b' ')?;
        let scope = rest.strip_prefix(b"@@")?;
        Some(Self {
            old_start: range_start(old)?,
            new_start: range_start(new)?,
            scope: scope.to_vec(),
            lines: Vec::new(),
        })
    }

    fn first_line(&self, side: LineSide) -> usize {
        let (start, count) = match side {
            LineSide::Old => (self.old_start, self.count(LineSide::Old)),
            LineSide::New => (self.new_start, self.count(LineSide::New)),
        };
        if count == 0 { start + 1 } else { start }
    }

    fn count(&self, side: LineSide) -> usize {
        self.lines
            .iter()
            .filter(|line| line.number(side).is_some())
            .count()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Hunks {
    head: Vec<u8>,
    hunks: Vec<Hunk>,
}

impl Hunks {
    pub(crate) fn parse(patch: &[u8]) -> Option<Self> {
        let mut head = Vec::new();
        let mut hunks: Vec<Hunk> = Vec::new();
        let (mut old, mut new) = (0, 0);
        for raw in patch.split_inclusive(|&byte| byte == b'\n') {
            if raw.starts_with(b"@@ ") {
                let hunk = Hunk::open(raw)?;
                (old, new) = (hunk.old_start, hunk.new_start);
                hunks.push(hunk);
                continue;
            }
            let Some(hunk) = hunks.last_mut() else {
                if !raw.starts_with(b"old mode ") && !raw.starts_with(b"new mode ") {
                    head.extend_from_slice(raw);
                }
                continue;
            };
            let (mark, text) = match raw.first() {
                Some(b' ') => (Mark::Context, &raw[1..]),
                Some(b'+') => (Mark::Added, &raw[1..]),
                Some(b'-') => (Mark::Deleted, &raw[1..]),
                Some(b'\n' | b'\r') => (Mark::Context, raw),
                Some(b'\\') => {
                    hunk.lines.last_mut()?.missing_newline = Some(raw.to_vec());
                    continue;
                }
                _ => return None,
            };
            let numbered = |kept: bool, counter: &mut usize| {
                kept.then(|| {
                    *counter += 1;
                    *counter - 1
                })
            };
            hunk.lines.push(Line {
                mark,
                text: text.to_vec(),
                old: numbered(mark != Mark::Added, &mut old),
                new: numbered(mark != Mark::Deleted, &mut new),
                missing_newline: None,
            });
        }
        (!hunks.is_empty()).then_some(Self { head, hunks })
    }

    pub(crate) fn len(&self) -> usize {
        self.hunks.len()
    }

    pub(crate) fn hunk(&self, hunk: usize) -> Part {
        Part(
            self.changes()
                .filter(|(at, _)| at.hunk == hunk)
                .map(|(at, _)| at)
                .collect(),
        )
    }

    pub(crate) fn hunk_at(&self, at: LineAt) -> Option<usize> {
        self.lines()
            .find(|(_, line)| line.is_at(at))
            .map(|(found, _)| found.hunk)
    }

    pub(crate) fn line_at(&self, at: LineAt) -> Part {
        Part(
            self.changes()
                .filter(|(_, line)| line.is_at(at))
                .map(|(found, _)| found)
                .collect(),
        )
    }

    pub(crate) fn within(&self, span: LineSpan) -> Part {
        let covered = self.covered(span);
        Part(
            self.lines()
                .enumerate()
                .filter(|(row, (_, line))| line.mark.is_change() && covered.includes(*row, line))
                .map(|(_, (at, _))| at)
                .collect(),
        )
    }

    pub(crate) fn covers(&self, span: LineSpan, at: LineAt) -> bool {
        let covered = self.covered(span);
        self.lines()
            .enumerate()
            .any(|(ix, (_, line))| line.is_at(at) && covered.includes(ix, line))
    }

    pub(crate) fn patch(&self, part: &Part, action: PartAction) -> Option<Vec<u8>> {
        let reverse = action.reverses();
        let mut patch = self.head.clone();
        let mut shift = 0isize;
        let mut written = false;
        for (ix, hunk) in self.hunks.iter().enumerate() {
            if !part.touches(ix) {
                continue;
            }
            written = true;
            let mut body = Vec::new();
            let (mut old_count, mut new_count) = (0, 0);
            for (line_ix, line) in hunk.lines.iter().enumerate() {
                let picked = part.0.contains(&LineRef {
                    hunk: ix,
                    line: line_ix,
                });
                let Some(mark) = kept_mark(line.mark, picked, reverse) else {
                    continue;
                };
                if mark != Mark::Added {
                    old_count += 1;
                }
                if mark != Mark::Deleted {
                    new_count += 1;
                }
                body.push(mark.byte());
                body.extend_from_slice(&line.text);
                if !line.text.ends_with(b"\n") {
                    body.push(b'\n');
                }
                if let Some(marker) = &line.missing_newline {
                    body.extend_from_slice(marker);
                }
            }
            let (old_first, new_first) = if reverse {
                let new_first = hunk.first_line(LineSide::New);
                (offset(new_first, shift), new_first)
            } else {
                let old_first = hunk.first_line(LineSide::Old);
                (old_first, offset(old_first, shift))
            };
            shift += if reverse {
                old_count as isize - new_count as isize
            } else {
                new_count as isize - old_count as isize
            };
            patch.extend_from_slice(
                format!(
                    "@@ -{},{old_count} +{},{new_count} @@",
                    start_of(old_first, old_count),
                    start_of(new_first, new_count)
                )
                .as_bytes(),
            );
            patch.extend_from_slice(&hunk.scope);
            if !hunk.scope.ends_with(b"\n") {
                patch.push(b'\n');
            }
            patch.extend_from_slice(&body);
        }
        written.then_some(patch)
    }

    fn lines(&self) -> impl Iterator<Item = (LineRef, &Line)> {
        self.hunks.iter().enumerate().flat_map(|(hunk, lines)| {
            lines
                .lines
                .iter()
                .enumerate()
                .map(move |(line, found)| (LineRef { hunk, line }, found))
        })
    }

    fn changes(&self) -> impl Iterator<Item = (LineRef, &Line)> {
        self.lines().filter(|(_, line)| line.mark.is_change())
    }

    fn covered(&self, span: LineSpan) -> Covered {
        if span.from.side == span.to.side {
            let (low, high) = ordered(span.from.line, span.to.line);
            return Covered::Side(span.from.side, low, high);
        }
        let place = |at: LineAt| self.lines().position(|(_, line)| line.is_at(at));
        match (place(span.from), place(span.to)) {
            (Some(from), Some(to)) => {
                let (low, high) = ordered(from, to);
                Covered::Rows(low, high)
            }
            _ => Covered::Nothing,
        }
    }
}

impl Repo {
    pub(crate) fn apply_part(&self, action: PartAction, patch: &[u8]) -> Result<()> {
        run_with_input(&self.location, action.args(), patch).map(drop)
    }
}

enum Covered {
    Side(LineSide, usize, usize),
    Rows(usize, usize),
    Nothing,
}

impl Covered {
    fn includes(&self, row: usize, line: &Line) -> bool {
        match *self {
            Covered::Side(side, low, high) => line
                .number(side)
                .is_some_and(|number| (low..=high).contains(&number)),
            Covered::Rows(low, high) => (low..=high).contains(&row),
            Covered::Nothing => false,
        }
    }
}

fn kept_mark(mark: Mark, picked: bool, reverse: bool) -> Option<Mark> {
    match (mark, picked, reverse) {
        (Mark::Context, _, _) => Some(Mark::Context),
        (mark, true, _) => Some(mark),
        (Mark::Added, false, false) | (Mark::Deleted, false, true) => None,
        (Mark::Deleted, false, false) | (Mark::Added, false, true) => Some(Mark::Context),
    }
}

fn offset(first: usize, shift: isize) -> usize {
    first.saturating_add_signed(shift).max(1)
}

fn start_of(first: usize, count: usize) -> usize {
    if count == 0 { first - 1 } else { first }
}

fn ordered(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

fn split_at_byte(bytes: &[u8], byte: u8) -> Option<(&[u8], &[u8])> {
    let at = bytes.iter().position(|&found| found == byte)?;
    Some((&bytes[..at], &bytes[at + 1..]))
}

fn range_start(range: &[u8]) -> Option<usize> {
    let start = range.split(|&byte| byte == b',').next()?;
    std::str::from_utf8(start).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Hunks, LineAt, LineSide, LineSpan, PartAction};
    use crate::git::{FileChange, FilePatch, Repo, Side, scratch::ScratchRepo};

    const TWO_HUNKS: &str = "diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
@@ -1,4 +1,5 @@ intro
 one
-two
+TWO
+two and a half
 three
 four
@@ -10,3 +11,3 @@ tail
 ten
-eleven
+ELEVEN
 twelve
";

    const HEAD: &str = "diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
";

    fn at(side: LineSide, line: usize) -> LineAt {
        LineAt { side, line }
    }

    fn span(from: LineAt, to: LineAt) -> LineSpan {
        LineSpan { from, to }
    }

    fn two_hunks() -> Hunks {
        Hunks::parse(TWO_HUNKS.as_bytes()).expect("the patch parses")
    }

    fn text(patch: Option<Vec<u8>>) -> String {
        String::from_utf8(patch.expect("something to apply")).expect("UTF-8")
    }

    fn with_head(body: &str) -> String {
        let lines: String = body.lines().map(|line| format!("{line}\n")).collect();
        format!("{HEAD}{lines}")
    }

    #[test]
    fn a_hunk_on_its_own_keeps_its_place_in_the_file() {
        let hunks = two_hunks();
        assert_eq!(hunks.len(), 2);
        assert_eq!(
            text(hunks.patch(&hunks.hunk(1), PartAction::Stage)),
            with_head("@@ -10,3 +10,3 @@ tail\n ten\n-eleven\n+ELEVEN\n twelve")
        );
        assert_eq!(
            text(hunks.patch(&hunks.hunk(1), PartAction::Discard)),
            with_head("@@ -11,3 +11,3 @@ tail\n ten\n-eleven\n+ELEVEN\n twelve")
        );
    }

    #[test]
    fn staging_one_added_line_keeps_the_deleted_one_as_context() {
        let hunks = two_hunks();
        let picked = hunks.within(span(at(LineSide::New, 2), at(LineSide::New, 2)));
        assert_eq!(picked.len(), 1);
        assert_eq!(
            text(hunks.patch(&picked, PartAction::Stage)),
            with_head("@@ -1,4 +1,5 @@ intro\n one\n two\n+TWO\n three\n four")
        );
    }

    #[test]
    fn discarding_one_added_line_keeps_the_other_added_line_as_context() {
        let hunks = two_hunks();
        let picked = hunks.line_at(at(LineSide::New, 2));
        assert_eq!(
            text(hunks.patch(&picked, PartAction::Discard)),
            with_head("@@ -1,4 +1,5 @@ intro\n one\n+TWO\n two and a half\n three\n four")
        );
    }

    #[test]
    fn hunks_after_a_partial_one_shift_by_what_it_adds() {
        let hunks = two_hunks();
        let mut picked = hunks.within(span(at(LineSide::Old, 2), at(LineSide::Old, 2)));
        picked.0.extend(hunks.hunk(1).0);
        assert_eq!(
            text(hunks.patch(&picked, PartAction::Stage)),
            with_head(
                "@@ -1,4 +1,3 @@ intro\n one\n-two\n three\n four\n@@ -10,3 +9,3 @@ tail\n ten\n-eleven\n+ELEVEN\n twelve"
            )
        );
    }

    #[test]
    fn a_selection_across_sides_covers_the_rows_between_in_patch_order() {
        let hunks = two_hunks();
        let selection = span(at(LineSide::Old, 2), at(LineSide::New, 2));
        assert_eq!(hunks.within(selection).len(), 2);
        assert!(hunks.covers(selection, at(LineSide::New, 2)));
        assert!(!hunks.covers(selection, at(LineSide::New, 3)));
        assert_eq!(hunks.within(selection).in_hunk(1).len(), 0);
        let everything = span(at(LineSide::Old, 2), at(LineSide::New, 12));
        assert_eq!(hunks.within(everything).len(), 5);
    }

    #[test]
    fn a_line_belongs_to_the_hunk_that_shows_it() {
        let hunks = two_hunks();
        assert_eq!(hunks.hunk_at(at(LineSide::New, 13)), Some(1));
        assert_eq!(hunks.hunk_at(at(LineSide::Old, 1)), Some(0));
        assert_eq!(hunks.hunk_at(at(LineSide::New, 7)), None);
        assert!(hunks.line_at(at(LineSide::Old, 1)).is_empty());
        assert_eq!(hunks.line_at(at(LineSide::Old, 11)).len(), 1);
    }

    #[test]
    fn a_missing_newline_marker_follows_its_line() {
        let patch = with_head(
            "@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+B\n\\ No newline at end of file",
        );
        let hunks = Hunks::parse(patch.as_bytes()).expect("the patch parses");
        assert_eq!(text(hunks.patch(&hunks.hunk(0), PartAction::Stage)), patch);
        let only_the_addition = hunks.line_at(at(LineSide::New, 2));
        assert_eq!(
            text(hunks.patch(&only_the_addition, PartAction::Discard)),
            with_head("@@ -1,1 +1,2 @@\n a\n+B\n\\ No newline at end of file")
        );
    }

    #[test]
    fn a_patch_without_hunks_or_with_several_files_has_no_parts() {
        let binary = format!("{HEAD}Binary files a/a.txt and b/a.txt differ\n");
        assert!(Hunks::parse(binary.as_bytes()).is_none());
        let two_files = format!("{TWO_HUNKS}{HEAD}");
        assert!(Hunks::parse(two_files.as_bytes()).is_none());
    }

    fn numbered(count: usize) -> String {
        (1..=count).map(|n| format!("line {n}\n")).collect()
    }

    fn edited(count: usize, edits: &[(usize, &str)]) -> String {
        (1..=count)
            .map(|n| match edits.iter().find(|(at, _)| *at == n) {
                Some((_, text)) => (*text).to_owned(),
                None => format!("line {n}\n"),
            })
            .collect()
    }

    fn hunks_of(repo: &Repo, change: &FileChange, side: Side) -> Result<Hunks> {
        match repo.file_patch(change, side)? {
            FilePatch::Diff {
                hunks: Some(hunks), ..
            } => Ok(hunks),
            _ => anyhow::bail!("expected hunks"),
        }
    }

    fn only(repo: &Repo, side: Side) -> Result<FileChange> {
        let status = repo.status()?;
        let files = status.files(side);
        assert_eq!(files.len(), 1, "{side:?}");
        Ok(files[0].clone())
    }

    fn two_edits(label: &str) -> Result<ScratchRepo> {
        let scratch = ScratchRepo::new(label)?;
        scratch.write("a.txt", &numbered(20))?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write(
            "a.txt",
            &edited(
                20,
                &[(2, "line 2, edited\nline 2b\n"), (18, "line 18, edited\n")],
            ),
        )?;
        Ok(scratch)
    }

    #[test]
    fn a_hunk_stages_unstages_and_discards_on_its_own() -> Result<()> {
        let scratch = two_edits("hunk")?;
        let (repo, _) = scratch.open()?;
        let hunks = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        assert_eq!(hunks.len(), 2);
        let first = hunks.patch(&hunks.hunk(0), PartAction::Stage);
        repo.apply_part(PartAction::Stage, &first.expect("a hunk"))?;

        let in_index = hunks_of(&repo, &only(&repo, Side::Staged)?, Side::Staged)?;
        assert_eq!(in_index.len(), 1);
        assert_eq!(in_index.hunk_at(at(LineSide::New, 2)), Some(0));
        let left = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        assert_eq!(left.len(), 1);
        assert_eq!(left.hunk_at(at(LineSide::New, 19)), Some(0));

        let back = in_index.patch(&in_index.hunk(0), PartAction::Unstage);
        repo.apply_part(PartAction::Unstage, &back.expect("a hunk"))?;
        assert!(repo.status()?.staged.is_empty());

        let all = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        let last = all.patch(&all.hunk(1), PartAction::Discard);
        repo.apply_part(PartAction::Discard, &last.expect("a hunk"))?;
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("a.txt"))?,
            edited(20, &[(2, "line 2, edited\nline 2b\n")])
        );
        Ok(())
    }

    #[test]
    fn selected_lines_stage_and_discard_without_the_rest_of_their_hunk() -> Result<()> {
        let scratch = two_edits("lines")?;
        let (repo, _) = scratch.open()?;
        let hunks = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        let second_added = hunks.line_at(at(LineSide::New, 3));
        let patch = hunks.patch(&second_added, PartAction::Stage);
        repo.apply_part(PartAction::Stage, &patch.expect("a line"))?;
        assert_eq!(
            scratch.git(&["show", ":a.txt"])?,
            edited(20, &[(2, "line 2\nline 2b\n")])
        );

        let hunks = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        let edit = hunks.line_at(at(LineSide::New, 2));
        let patch = hunks.patch(&edit, PartAction::Discard);
        repo.apply_part(PartAction::Discard, &patch.expect("a line"))?;
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("a.txt"))?,
            edited(20, &[(2, "line 2b\n"), (18, "line 18, edited\n")])
        );
        Ok(())
    }

    #[test]
    fn a_latin_1_hunk_keeps_its_bytes() -> Result<()> {
        let scratch = ScratchRepo::new("hunk-latin-1")?;
        std::fs::write(scratch.dir.join("notes.txt"), b"caf\xE9\n")?;
        scratch.git(&["add", "notes.txt"])?;
        scratch.commit("first")?;
        std::fs::write(scratch.dir.join("notes.txt"), b"caf\xE9 cr\xE8me\n")?;
        let (repo, _) = scratch.open()?;
        let hunks = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        let patch = hunks.patch(&hunks.hunk(0), PartAction::Stage);
        repo.apply_part(PartAction::Stage, &patch.expect("a hunk"))?;
        assert!(repo.status()?.unstaged.is_empty());
        Ok(())
    }

    #[test]
    fn a_file_without_a_final_newline_stages_its_last_line() -> Result<()> {
        let scratch = ScratchRepo::new("hunk-no-newline")?;
        scratch.write("a.txt", "a\nb")?;
        scratch.git(&["add", "a.txt"])?;
        scratch.commit("first")?;
        scratch.write("a.txt", "a\nB")?;
        let (repo, _) = scratch.open()?;
        let hunks = hunks_of(&repo, &only(&repo, Side::Unstaged)?, Side::Unstaged)?;
        let patch = hunks.patch(&hunks.hunk(0), PartAction::Stage);
        repo.apply_part(PartAction::Stage, &patch.expect("a hunk"))?;
        assert_eq!(scratch.git(&["show", ":a.txt"])?, "a\nB");
        Ok(())
    }

    #[test]
    fn discarding_files_puts_back_what_is_staged_and_deletes_the_untracked_ones() -> Result<()> {
        let scratch = ScratchRepo::new("discard-file")?;
        scratch.write("kept.txt", "one\n")?;
        scratch.write("other.txt", "one\n")?;
        scratch.git(&["add", "kept.txt", "other.txt"])?;
        scratch.commit("first")?;
        scratch.write("kept.txt", "one\ntwo\n")?;
        scratch.git(&["add", "kept.txt"])?;
        scratch.write("kept.txt", "one\ntwo\nthree\n")?;
        scratch.write("other.txt", "one\nchanged\n")?;
        scratch.write("fresh.txt", "new\n")?;
        scratch.write("loose.txt", "new\n")?;
        let (repo, _) = scratch.open()?;
        repo.discard(&repo.status()?.unstaged)?;
        let status = repo.status()?;
        assert!(status.unstaged.is_empty());
        assert_eq!(status.staged.len(), 1);
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("kept.txt"))?,
            "one\ntwo\n"
        );
        assert_eq!(
            std::fs::read_to_string(scratch.dir.join("other.txt"))?,
            "one\n"
        );
        assert!(!scratch.dir.join("fresh.txt").exists());
        assert!(!scratch.dir.join("loose.txt").exists());
        Ok(())
    }
}
