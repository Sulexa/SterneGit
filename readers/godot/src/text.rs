#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Section {
    pub(crate) tag: String,
    pub(crate) attributes: Vec<(String, String)>,
    pub(crate) properties: Vec<(String, String)>,
    pub(crate) first_line: u32,
    pub(crate) last_line: u32,
}

impl Section {
    pub(crate) fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
    }
}

pub(crate) fn unquoted(value: &str) -> &str {
    value.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')).unwrap_or(value)
}

fn balance(text: &str) -> i32 {
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for c in text.chars() {
        if quoted {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => quoted = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
    }
    if quoted { depth + 1 } else { depth }
}

fn header(line: &str) -> Option<(String, Vec<(String, String)>)> {
    let inner = line.trim().strip_prefix('[')?.strip_suffix(']')?;
    let (tag, rest) = inner.split_once(' ').unwrap_or((inner, ""));
    Some((tag.to_owned(), split_pairs(rest, ' ')))
}

pub(crate) fn split_top(text: &str, separator: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    for c in text.chars() {
        if quoted {
            current.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => quoted = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
        if c == separator && depth == 0 {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_owned());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_owned());
    }
    parts
}

fn split_pairs(text: &str, separator: char) -> Vec<(String, String)> {
    split_top(text, separator)
        .into_iter()
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

pub(crate) fn is_godot_text(text: &str) -> bool {
    let first = text.trim_start_matches('\u{feff}').lines().next().unwrap_or_default();
    first.starts_with("[gd_scene") || first.starts_with("[gd_resource")
}

pub(crate) fn sections(text: &str) -> Vec<Section> {
    let mut found: Vec<Section> = Vec::new();
    let mut pending: Option<(String, String, u32)> = None;
    for (ix, line) in text.lines().enumerate() {
        let number = ix as u32 + 1;
        if let Some((key, value, _)) = pending.as_mut() {
            value.push('\n');
            value.push_str(line);
            if balance(value) <= 0 {
                let (key, value) = (std::mem::take(key), std::mem::take(value));
                if let Some(section) = found.last_mut() {
                    section.properties.push((key, value));
                    section.last_line = number;
                }
                pending = None;
            }
            continue;
        }
        if line.starts_with('[') {
            if let Some((tag, attributes)) = header(line) {
                found.push(Section {
                    tag,
                    attributes,
                    properties: Vec::new(),
                    first_line: number,
                    last_line: number,
                });
            }
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        if found.is_empty() {
            continue;
        }
        if balance(value) > 0 {
            pending = Some((key.trim().to_owned(), value.to_owned(), number));
            continue;
        }
        if let Some(section) = found.last_mut() {
            section.properties.push((key.trim().to_owned(), value.trim().to_owned()));
            section.last_line = number;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{sections, split_top};

    #[test]
    fn sections_keep_their_attributes_properties_and_lines() {
        let text = "[gd_scene load_steps=2 format=3]\n\n[node name=\"Player\" type=\"CharacterBody2D\"]\nspeed = 300.0\npoints = PackedVector2Array(0, 0,\n10, 10)\n\n[node name=\"Sprite\" type=\"Sprite2D\" parent=\".\"]\n";
        let found = sections(text);
        assert_eq!(found.len(), 3);
        assert_eq!(found[1].attribute("name"), Some("\"Player\""));
        assert_eq!(found[1].properties[1], ("points".to_owned(), "PackedVector2Array(0, 0,\n10, 10)".to_owned()));
        assert_eq!((found[1].first_line, found[1].last_line), (3, 6));
        assert_eq!(found[2].attribute("parent"), Some("\".\""));
    }

    #[test]
    fn splitting_ignores_separators_inside_strings_and_brackets() {
        assert_eq!(split_top("1, \"a, b\", Vector2(3, 4)", ','), ["1", "\"a, b\"", "Vector2(3, 4)"]);
    }
}
