use std::ops::Range;

use gpui::SharedString;

const PRIMITIVE_TYPES: &[&str] = &[
    "bool", "boolean", "byte", "char", "double", "f32", "f64", "float", "i8", "i16", "i32", "i64",
    "i128", "int", "isize", "long", "short", "str", "u8", "u16", "u32", "u64", "u128", "uint",
    "usize", "void",
];
const SELF_WORDS: &[&str] = &["self", "Self", "this"];
const BOOLEANS: &[&str] = &["true", "false"];
const NOTHING: &[&str] = &["null", "nil"];
const PROSE: &[&str] = &[
    "astro",
    "diff",
    "ejs",
    "erb",
    "html",
    "markdown",
    "markdown_inline",
    "svelte",
    "text",
    "toml",
    "yaml",
];

pub(crate) fn follows_code_rules(language: &str) -> bool {
    !PROSE.contains(&language)
}

pub(crate) fn coherent_names(
    text: &str,
    captured: &[(Range<usize>, SharedString)],
) -> Vec<(Range<usize>, SharedString)> {
    let mut named: Vec<(Range<usize>, SharedString)> = Vec::with_capacity(captured.len());
    let mut next = 0;
    let mut ix = 0;
    while ix < text.len() {
        while captured.get(next).is_some_and(|(range, _)| range.end <= ix) {
            next += 1;
        }
        let base = captured
            .get(next)
            .filter(|(range, _)| range.start <= ix)
            .map(|(range, name)| (range.end, name.clone()));
        if let Some((end, name)) = &base {
            let quoted = if is_quoted(name) {
                Some(name.clone())
            } else {
                (name.as_ref() == "constant" && touches_string(captured, next))
                    .then(|| SharedString::new_static("string"))
            };
            if let Some(quoted) = quoted {
                push(&mut named, ix..*end, quoted);
                ix = *end;
                continue;
            }
        }
        let Some(ch) = text[ix..].chars().next() else {
            break;
        };
        if ch.is_alphabetic() || ch == '_' {
            let end = word_end(text, ix);
            let (name, end) = word_name(&text[ix..end], &text[end..], end, base.map(|(_, name)| name));
            if let Some(name) = name {
                push(&mut named, ix..end, name);
            }
            ix = end;
        } else if ch.is_ascii_digit() {
            let end = number_end(text, ix);
            push(&mut named, ix..end, SharedString::new_static("number"));
            ix = end;
        } else {
            let end = ix + ch.len_utf8();
            if let Some((_, name)) = base {
                push(&mut named, ix..end, name);
            }
            ix = end;
        }
    }
    named
}

fn is_quoted(name: &str) -> bool {
    name.starts_with("string") || name.starts_with("comment")
}

fn touches_string(captured: &[(Range<usize>, SharedString)], ix: usize) -> bool {
    let range = &captured[ix].0;
    let before = ix.checked_sub(1).is_some_and(|before| {
        captured[before].0.end == range.start && captured[before].1.starts_with("string")
    });
    let after = captured
        .get(ix + 1)
        .is_some_and(|(next, name)| next.start == range.end && name.starts_with("string"));
    before || after
}

fn push(named: &mut Vec<(Range<usize>, SharedString)>, range: Range<usize>, name: SharedString) {
    match named.last_mut() {
        Some((last, last_name)) if last.end == range.start && *last_name == name => {
            last.end = range.end
        }
        _ => named.push((range, name)),
    }
}

fn word_end(text: &str, start: usize) -> usize {
    text[start..]
        .find(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map_or(text.len(), |offset| start + offset)
}

fn number_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        let byte = bytes[end];
        let fraction = byte == b'.' && bytes.get(end + 1).is_some_and(u8::is_ascii_digit);
        if byte.is_ascii_alphanumeric() || byte == b'_' || fraction {
            end += 1;
        } else {
            break;
        }
    }
    end
}

fn word_name(
    word: &str,
    after: &str,
    end: usize,
    base: Option<SharedString>,
) -> (Option<SharedString>, usize) {
    let named = |name: &'static str| (Some(SharedString::new_static(name)), end);
    if SELF_WORDS.contains(&word) {
        return named("keyword");
    }
    if PRIMITIVE_TYPES.contains(&word) {
        return named("type");
    }
    if BOOLEANS.contains(&word) {
        return named("boolean");
    }
    if NOTHING.contains(&word) {
        return named("constant");
    }
    if base
        .as_ref()
        .is_some_and(|name| name.as_ref() == "keyword" || name.as_ref() == "preproc")
    {
        return (base, end);
    }
    if is_screaming(word) {
        return named("constant");
    }
    if word.starts_with(char::is_uppercase) {
        return named("type");
    }
    if after.starts_with('(') {
        return named("function");
    }
    if after.starts_with('!') && !after.starts_with("!=") {
        return (Some(SharedString::new_static("function")), end + 1);
    }
    (base, end)
}

fn is_screaming(word: &str) -> bool {
    word.chars().count() > 1
        && word.chars().any(char::is_uppercase)
        && !word.chars().any(char::is_lowercase)
}
