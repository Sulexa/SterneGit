const MOST_LINES_SPREAD: usize = 100_000;
const NO_NEWLINE_MARKER: &str = "\\ No newline at end of file\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    start: usize,
    count: usize,
}

impl Range {
    fn parse(text: &str) -> Option<Self> {
        let (start, count) = text.split_once(',').unwrap_or((text, "1"));
        Some(Self {
            start: start.parse().ok()?,
            count: count.parse().ok()?,
        })
    }

    fn first(self) -> usize {
        if self.count == 0 {
            self.start + 1
        } else {
            self.start
        }
    }

    fn last(self) -> usize {
        self.first() + self.count - 1
    }

    fn grown_above(self, lines: usize) -> Self {
        if lines == 0 {
            return self;
        }
        Self {
            start: self.first() - lines,
            count: self.count + lines,
        }
    }

    fn grown_below(self, lines: usize) -> Self {
        if lines == 0 {
            return self;
        }
        Self {
            start: self.first(),
            count: self.count + lines,
        }
    }

    fn written(self) -> String {
        format!("{},{}", self.start, self.count)
    }
}

struct HunkHeader<'a> {
    old: Range,
    new: Range,
    scope: &'a str,
}

impl<'a> HunkHeader<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        let rest = line.strip_prefix("@@ -")?;
        let (ranges, scope) = rest.split_once(" @@")?;
        let (old, new) = ranges.split_once(" +")?;
        Some(Self {
            old: Range::parse(old)?,
            new: Range::parse(new)?,
            scope,
        })
    }

    fn written(&self) -> String {
        format!(
            "@@ -{} +{} @@{}",
            self.old.written(),
            self.new.written(),
            self.scope.trim_end_matches(['\r', '\n'])
        )
    }
}

struct Hunk<'a> {
    header: HunkHeader<'a>,
    body: Vec<&'a str>,
}

pub(crate) fn with_whole_file(patch: &str, new_text: &str) -> Option<String> {
    let mut lines = patch.split_inclusive('\n').peekable();
    let mut head = Vec::new();
    while let Some(line) = lines.next_if(|line| !line.starts_with("@@ ")) {
        head.push(line);
    }
    let mut hunks: Vec<Hunk> = Vec::new();
    for line in lines {
        match HunkHeader::parse(line) {
            Some(header) => hunks.push(Hunk {
                header,
                body: Vec::new(),
            }),
            None => hunks.last_mut()?.body.push(line),
        }
    }
    if hunks.is_empty() {
        return None;
    }
    let carriage_returns = hunks
        .iter()
        .any(|hunk| hunk.body.iter().any(|line| line.ends_with("\r\n")));
    let file: Vec<&str> = new_text
        .split_inclusive('\n')
        .map(|line| {
            let line = line.strip_suffix('\n').unwrap_or(line);
            line.strip_suffix('\r').unwrap_or(line)
        })
        .collect();
    if file.len() > MOST_LINES_SPREAD {
        return None;
    }
    let ends_without_newline = !new_text.is_empty() && !new_text.ends_with('\n');
    let line_end = if carriage_returns { "\r\n" } else { "\n" };
    let context = |numbers: std::ops::RangeInclusive<usize>, out: &mut String| -> Option<()> {
        for number in numbers {
            out.push(' ');
            out.push_str(file.get(number - 1)?);
            out.push_str(line_end);
        }
        Some(())
    };
    let mut out: String = head.concat();
    let mut shown_up_to = 0;
    let last = hunks.len() - 1;
    for (ix, hunk) in hunks.iter().enumerate() {
        let above = hunk.header.new.first().checked_sub(shown_up_to + 1)?;
        let below = if ix == last {
            file.len().checked_sub(hunk.header.new.last())?
        } else {
            0
        };
        let header = HunkHeader {
            old: hunk.header.old.grown_above(above).grown_below(below),
            new: hunk.header.new.grown_above(above).grown_below(below),
            scope: hunk.header.scope,
        };
        out.push_str(&header.written());
        out.push('\n');
        context(shown_up_to + 1..=hunk.header.new.first() - 1, &mut out)?;
        out.extend(hunk.body.iter().copied());
        if below > 0 {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            context(hunk.header.new.last() + 1..=file.len(), &mut out)?;
            if ends_without_newline {
                out.push_str(NO_NEWLINE_MARKER);
            }
        }
        shown_up_to = hunk.header.new.last();
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::with_whole_file;

    const HEAD: &str = "diff --git a/a.txt b/a.txt\nindex 1111111..2222222 100644\n--- a/a.txt\n+++ b/a.txt\n";

    fn numbered(lines: std::ops::RangeInclusive<usize>) -> String {
        lines.map(|n| format!("line {n}\n")).collect()
    }

    #[test]
    fn each_hunk_takes_the_unchanged_lines_above_it_and_the_last_one_those_below() {
        let new_text = numbered(1..=3) + "changed 4\n" + &numbered(5..=12) + "changed 13\n" + &numbered(14..=20);
        let patch = format!(
            "{HEAD}@@ -3,3 +3,3 @@ fn first\n line 3\n-line 4\n+changed 4\n line 5\n@@ -12,3 +12,3 @@ fn second\n line 12\n-line 13\n+changed 13\n line 14\n"
        );
        let expected = format!(
            "{HEAD}@@ -1,5 +1,5 @@ fn first\n line 1\n line 2\n line 3\n-line 4\n+changed 4\n line 5\n@@ -6,15 +6,15 @@ fn second\n line 6\n line 7\n line 8\n line 9\n line 10\n line 11\n line 12\n-line 13\n+changed 13\n line 14\n line 15\n line 16\n line 17\n line 18\n line 19\n line 20\n"
        );
        assert_eq!(with_whole_file(&patch, &new_text).as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn added_and_removed_lines_keep_both_sides_counted_right() {
        let new_text = numbered(1..=4) + "added\n" + &numbered(5..=6);
        let patch = format!("{HEAD}@@ -4,0 +5 @@ scope\n+added\n");
        let expected = format!(
            "{HEAD}@@ -1,6 +1,7 @@ scope\n line 1\n line 2\n line 3\n line 4\n+added\n line 5\n line 6\n"
        );
        assert_eq!(with_whole_file(&patch, &new_text).as_deref(), Some(expected.as_str()));
        let new_text = numbered(1..=2) + &numbered(4..=5);
        let patch = format!("{HEAD}@@ -3 +2,0 @@\n-line 3\n");
        let expected = format!("{HEAD}@@ -1,5 +1,4 @@\n line 1\n line 2\n-line 3\n line 4\n line 5\n");
        assert_eq!(with_whole_file(&patch, &new_text).as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn a_file_without_a_last_newline_keeps_saying_so() {
        let new_text = "changed 1\nline 2\nline 3";
        let patch = format!("{HEAD}@@ -1 +1 @@\n-line 1\n+changed 1\n");
        let expected = format!(
            "{HEAD}@@ -1,3 +1,3 @@\n-line 1\n+changed 1\n line 2\n line 3\n\\ No newline at end of file\n"
        );
        assert_eq!(with_whole_file(&patch, new_text).as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn carriage_returns_follow_the_patch() {
        let new_text = "one\r\ntwo\r\nthree\r\n";
        let patch = format!("{HEAD}@@ -2 +2 @@\n-2\n+two\n");
        let expected = format!("{HEAD}@@ -1,3 +1,3 @@\n one\n-2\n+two\n three\n");
        assert_eq!(with_whole_file(&patch, new_text).as_deref(), Some(expected.as_str()));
        let patch = format!("{HEAD}@@ -2 +2 @@\r\n-2\r\n+two\r\n");
        let expected = format!("{HEAD}@@ -1,3 +1,3 @@\n one\r\n-2\r\n+two\r\n three\r\n");
        assert_eq!(with_whole_file(&patch, new_text).as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn a_patch_that_disagrees_with_the_file_is_left_alone() {
        let patch = format!("{HEAD}@@ -40 +40 @@\n-old\n+new\n");
        assert_eq!(with_whole_file(&patch, "short\n"), None);
        assert_eq!(with_whole_file("Binary files a/x and b/x differ\n", "x"), None);
    }
}
