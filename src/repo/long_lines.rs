use std::borrow::Cow;

use crate::ui::counted;

const LONGEST_WHOLE_LINE: usize = 10_000;
const SHOWN_CHARACTERS: usize = 900;
const SHOWN_BEFORE_A_DIFFERENCE: usize = 100;
const HUNK_START: &str = "@@ ";
const FILE_START: &str = "diff ";

pub(crate) fn shown_lines(patch: &str) -> Cow<'_, str> {
    if patch.split('\n').all(|line| line.len() <= LONGEST_WHOLE_LINE) {
        return Cow::Borrowed(patch);
    }
    let lines: Vec<&str> = patch.split_inclusive('\n').collect();
    let mut shown = String::with_capacity(patch.len().min(lines.len() * LONGEST_WHOLE_LINE));
    for (line, from) in lines.iter().zip(cut_points(&lines)) {
        match from {
            Some(from) => push_cut(&mut shown, BodyLine::of(line), from),
            None => shown.push_str(line),
        }
    }
    Cow::Owned(shown)
}

struct BodyLine<'a> {
    sign: &'a str,
    content: &'a str,
    ending: &'a str,
}

impl<'a> BodyLine<'a> {
    fn of(line: &'a str) -> Self {
        let (sign, rest) = line.split_at(1);
        let content = rest.strip_suffix('\n').unwrap_or(rest);
        let content = content.strip_suffix('\r').unwrap_or(content);
        Self {
            sign,
            content,
            ending: &rest[content.len()..],
        }
    }

    fn is_long(&self) -> bool {
        self.content.len() > LONGEST_WHOLE_LINE && self.content.chars().nth(LONGEST_WHOLE_LINE).is_some()
    }
}

#[derive(Default)]
struct ChangeBlock {
    removed: Vec<usize>,
    added: Vec<usize>,
}

impl ChangeBlock {
    fn close(&mut self, lines: &[&str], points: &mut [Option<usize>]) {
        let pairs = self.removed.len().max(self.added.len());
        for ix in 0..pairs {
            let old = self.removed.get(ix).map(|&at| (at, BodyLine::of(lines[at])));
            let new = self.added.get(ix).map(|&at| (at, BodyLine::of(lines[at])));
            let from = match (&old, &new) {
                (Some((_, old)), Some((_, new))) => before_first_difference(old.content, new.content),
                _ => 0,
            };
            for (at, line) in [old, new].into_iter().flatten() {
                if line.is_long() {
                    points[at] = Some(from);
                }
            }
        }
        self.removed.clear();
        self.added.clear();
    }
}

fn cut_points(lines: &[&str]) -> Vec<Option<usize>> {
    let mut points = vec![None; lines.len()];
    let mut block = ChangeBlock::default();
    let mut in_hunk = false;
    for (at, line) in lines.iter().enumerate() {
        if line.starts_with(HUNK_START) || line.starts_with(FILE_START) {
            block.close(lines, &mut points);
            in_hunk = line.starts_with(HUNK_START);
            continue;
        }
        if !in_hunk {
            continue;
        }
        match line.as_bytes().first() {
            Some(b'-') => {
                if !block.added.is_empty() {
                    block.close(lines, &mut points);
                }
                block.removed.push(at);
            }
            Some(b'+') => block.added.push(at),
            Some(b'\\') => {}
            Some(_) => {
                block.close(lines, &mut points);
                if BodyLine::of(line).is_long() {
                    points[at] = Some(0);
                }
            }
            None => block.close(lines, &mut points),
        }
    }
    block.close(lines, &mut points);
    points
}

fn before_first_difference(old: &str, new: &str) -> usize {
    let mut same = old.bytes().zip(new.bytes()).take_while(|(old, new)| old == new).count();
    while !old.is_char_boundary(same) {
        same -= 1;
    }
    old[..same]
        .char_indices()
        .rev()
        .nth(SHOWN_BEFORE_A_DIFFERENCE - 1)
        .map_or(0, |(at, _)| at)
}

fn push_cut(shown: &mut String, line: BodyLine, from: usize) {
    let window = &line.content[from..];
    let end = window.char_indices().nth(SHOWN_CHARACTERS).map_or(window.len(), |(at, _)| at);
    shown.push_str(line.sign);
    if from > 0 {
        let left_out = line.content[..from].chars().count();
        shown.push_str(&format!("… {} … ", counted(left_out, "character", "characters")));
    }
    shown.push_str(&window[..end]);
    if end < window.len() {
        let left_out = window[end..].chars().count();
        shown.push_str(&format!(" … {}", counted(left_out, "more character", "more characters")));
    }
    shown.push_str(line.ending);
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{LONGEST_WHOLE_LINE, SHOWN_BEFORE_A_DIFFERENCE, SHOWN_CHARACTERS, shown_lines};

    const HEAD: &str = "diff --git a/f.asset b/f.asset\nindex 1..2 100644\n--- a/f.asset\n+++ b/f.asset\n";

    fn blob(length: usize, changed_at: Option<usize>) -> String {
        (0..length).map(|at| if Some(at) == changed_at { 'f' } else { '0' }).collect()
    }

    fn body_lines(shown: &str) -> Vec<&str> {
        shown.split_inclusive('\n').skip(5).collect()
    }

    #[test]
    fn a_patch_without_long_lines_is_shown_as_it_is() {
        let patch = format!("{HEAD}@@ -1,2 +1,2 @@\n a\n-b\n+c\n");
        assert!(matches!(shown_lines(&patch), Cow::Borrowed(shown) if shown == patch));
    }

    #[test]
    fn a_changed_long_line_shows_the_part_around_its_first_difference_on_both_sides() {
        let length = 50_000;
        let patch = format!(
            "{HEAD}@@ -1,2 +1,2 @@\n a\n-  data: {}\n+  data: {}\n",
            blob(length, None),
            blob(length, Some(30_000))
        );
        let shown = shown_lines(&patch);
        let [context, old, new] = body_lines(&shown)[..] else { panic!("{shown}") };
        assert_eq!(context, " a\n");
        let skipped = 30_000 + "  data: ".len() - SHOWN_BEFORE_A_DIFFERENCE;
        let more = 50_000 + "  data: ".len() - skipped - SHOWN_CHARACTERS;
        let expected = |sign: char, changed: bool| {
            let window: String = (0..SHOWN_CHARACTERS)
                .map(|at| if changed && at == SHOWN_BEFORE_A_DIFFERENCE { 'f' } else { '0' })
                .collect();
            format!(
                "{sign}… {} characters … {window} … {} more characters\n",
                crate::ui::thousands(skipped),
                crate::ui::thousands(more)
            )
        };
        assert_eq!(old, expected('-', false));
        assert_eq!(new, expected('+', true));
    }

    #[test]
    fn an_unchanged_or_unpaired_long_line_shows_its_start() {
        let patch = format!("{HEAD}@@ -1,1 +1,2 @@\n {}\n+{}\n", blob(20_000, None), blob(LONGEST_WHOLE_LINE + 1, None));
        let shown = shown_lines(&patch);
        let lines = body_lines(&shown);
        assert_eq!(lines[0], format!(" {} … 19,100 more characters\n", blob(SHOWN_CHARACTERS, None)));
        assert_eq!(lines[1], format!("+{} … 9,101 more characters\n", blob(SHOWN_CHARACTERS, None)));
    }

    #[test]
    fn a_line_at_the_limit_stays_whole() {
        let patch = format!("{HEAD}@@ -1 +1 @@\n-{}\n+b\n", blob(LONGEST_WHOLE_LINE, None));
        assert_eq!(shown_lines(&patch).len(), patch.len());
        assert_eq!(body_lines(&shown_lines(&patch)), body_lines(&patch));
    }

    #[test]
    fn cut_lines_keep_their_carriage_returns_and_the_lines_around_them() {
        let patch = format!("{HEAD}@@ -1,3 +1,3 @@\r\n a\r\n-{}\r\n\\ No newline at end of file\n+b\r\n z\r\n", blob(20_000, None));
        let shown = shown_lines(&patch);
        let lines = body_lines(&shown);
        assert_eq!(lines.len(), 5);
        assert!(lines[1].starts_with("-0000") && lines[1].ends_with(" more characters\r\n"), "{}", lines[1]);
        assert_eq!(lines[2], "\\ No newline at end of file\n");
        assert_eq!(lines[3], "+b\r\n");
        assert_eq!(lines[4], " z\r\n");
    }

    #[test]
    fn a_cut_never_splits_a_character() {
        let old = "é".repeat(20_000);
        let new = format!("{}x{}", "é".repeat(15_000), "é".repeat(4_999));
        let patch = format!("{HEAD}@@ -1 +1 @@\n-{old}\n+{new}\n");
        let shown = shown_lines(&patch);
        let lines = body_lines(&shown);
        assert!(lines[0].starts_with("-… 14,900 characters … ééé"), "{}", lines[0].chars().take(40).collect::<String>());
        assert!(lines[1].contains(&format!("{}x", "é".repeat(SHOWN_BEFORE_A_DIFFERENCE))));
        assert!(lines[1].ends_with(" … 4,200 more characters\n"));
    }

    #[test]
    fn header_lines_are_never_cut() {
        let long_name = blob(20_000, None);
        let patch = format!("diff --git a/{long_name} b/{long_name}\n--- a/{long_name}\n+++ b/{long_name}\n@@ -1 +1 @@\n-a\n+b\n");
        assert_eq!(shown_lines(&patch), patch);
    }
}
