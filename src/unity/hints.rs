use std::collections::HashMap;

use sternegit_reader_protocol::{Note, Scope};

const HINT: &str = "  # ";

#[derive(Clone, Copy, Default)]
pub(crate) struct Labels<'a> {
    pub(crate) scopes: &'a [Scope],
    pub(crate) notes: &'a [Note],
}

struct Side<'a> {
    scopes: &'a [Scope],
    notes: HashMap<u32, &'a str>,
}

impl<'a> Side<'a> {
    fn of(labels: Labels<'a>) -> Self {
        Self {
            scopes: labels.scopes,
            notes: labels.notes.iter().map(|note| (note.line, note.text.as_str())).collect(),
        }
    }

    fn scope_at(&self, line: usize) -> Option<String> {
        let line = u32::try_from(line).ok()?;
        self.scopes
            .iter()
            .find(|scope| scope.lines.0 <= line && line <= scope.lines.1)
            .map(|scope| scope.label.clone())
    }

    fn note_at(&self, line: usize) -> Option<&'a str> {
        self.notes.get(&u32::try_from(line).ok()?).copied()
    }
}

pub(crate) fn annotate(patch: &str, before: Labels, after: Labels) -> String {
    let before = Side::of(before);
    let after = Side::of(after);
    let mut out = String::with_capacity(patch.len() + patch.len() / 8);
    let lines: Vec<&str> = patch.split_inclusive('\n').collect();
    let mut ix = 0;
    while ix < lines.len() {
        let line = lines[ix];
        if let Some(header) = line.strip_prefix("@@ ")
            && let Some((ranges, _)) = header.split_once(" @@")
        {
            let (first_old, first_new) = starts(ranges);
            let mut end = ix + 1;
            while end < lines.len() && !lines[end].starts_with("@@ ") && !lines[end].starts_with("diff --git") {
                end += 1;
            }
            let body = &lines[ix + 1..end];
            out.push_str("@@ ");
            out.push_str(ranges);
            out.push_str(" @@");
            if let Some(scope) = first_scope(body, first_old, first_new, &before, &after) {
                out.push(' ');
                out.push_str(&scope);
            }
            out.push('\n');
            let (mut old, mut new) = (first_old, first_new);
            for line in body {
                let note = match line.as_bytes().first() {
                    Some(b'-') => {
                        old += 1;
                        before.note_at(old - 1)
                    }
                    Some(b'+') => {
                        new += 1;
                        after.note_at(new - 1)
                    }
                    Some(b' ') => {
                        old += 1;
                        new += 1;
                        after.note_at(new - 1).or_else(|| before.note_at(old - 1))
                    }
                    _ => None,
                };
                out.push_str(&with_note(line, note));
            }
            ix = end;
            continue;
        }
        out.push_str(line);
        ix += 1;
    }
    out
}

fn first_scope(body: &[&str], mut old: usize, mut new: usize, before: &Side, after: &Side) -> Option<String> {
    for line in body {
        match line.as_bytes().first() {
            Some(b'+') => return after.scope_at(new),
            Some(b'-') => return before.scope_at(old),
            Some(b' ') => {
                old += 1;
                new += 1;
            }
            _ => {}
        }
    }
    None
}

fn with_note(line: &str, note: Option<&str>) -> String {
    let Some(note) = note else {
        return line.to_owned();
    };
    let (text, ending) = match line.strip_suffix("\r\n") {
        Some(text) => (text, "\r\n"),
        None => match line.strip_suffix('\n') {
            Some(text) => (text, "\n"),
            None => (line, ""),
        },
    };
    format!("{text}{HINT}{note}{ending}")
}

fn starts(ranges: &str) -> (usize, usize) {
    let mut parts = ranges.split_whitespace();
    let read = |part: Option<&str>| {
        part.map(|part| part.trim_start_matches(['-', '+']))
            .and_then(|part| part.split(',').next())
            .and_then(|start| start.parse::<usize>().ok())
            .unwrap_or(1)
    };
    let old = read(parts.next());
    let new = read(parts.next());
    (old, new)
}

#[cfg(test)]
mod tests {
    use sternegit_reader_protocol::{Note, Scope};

    use super::{Labels, annotate};

    fn scope(first: u32, last: u32, label: &str) -> Scope {
        Scope {
            lines: (first, last),
            label: label.into(),
            object: None,
        }
    }

    fn note(line: u32, text: &str) -> Note {
        Note { line, text: text.into() }
    }

    #[test]
    fn a_hunk_is_named_after_the_scope_of_its_first_change_and_lines_get_their_notes() {
        let scopes = [scope(3, 7, "Slime"), scope(8, 12, "Slime › Rigidbody 2D")];
        let before_notes = [note(10, "Slime")];
        let after_notes = [note(10, "Slime"), note(12, "slime.png")];
        let patch = "diff --git a/S.prefab b/S.prefab\n--- a/S.prefab\n+++ b/S.prefab\n@@ -10,3 +10,3 @@ Rigidbody2D:\n   m_GameObject: {fileID: 100}\n-  m_Mass: 1\n+  m_Mass: 1.4\n   m_Sprite: {fileID: 21300000, guid: aaaa, type: 3}\n";
        let annotated = annotate(
            patch,
            Labels {
                scopes: &scopes,
                notes: &before_notes,
            },
            Labels {
                scopes: &scopes,
                notes: &after_notes,
            },
        );
        assert!(annotated.contains("@@ -10,3 +10,3 @@ Slime › Rigidbody 2D\n"), "{annotated}");
        assert!(annotated.contains("   m_GameObject: {fileID: 100}  # Slime\n"), "{annotated}");
        assert!(annotated.contains("type: 3}  # slime.png\n"), "{annotated}");
        assert!(annotated.contains("+  m_Mass: 1.4\n"), "{annotated}");
    }

    #[test]
    fn a_hunk_with_no_scope_keeps_its_header() {
        let patch = "@@ -1,1 +1,1 @@\n-a\n+b\n";
        assert_eq!(annotate(patch, Labels::default(), Labels::default()), patch);
    }
}
