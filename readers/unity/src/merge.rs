use std::collections::HashMap;

use sternegit_reader_sdk::{Choice, Conflict, Merged, Pick};

use crate::{
    lookup::lookup_for,
    names::{component_title, nicify},
    scene::object_names,
    yaml::documents,
};

const HEADER: &str = "--- !u!";
const WHOLE: &str = "";
const WHOLE_LABEL: &str = "Whole object";
const GONE: &str = "Deleted";
const SHOWN_CHARS: usize = 80;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Block {
    key: String,
    lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Doc {
    id: String,
    header: String,
    type_line: Option<String>,
    blocks: Vec<Block>,
}

struct File {
    preamble: Vec<String>,
    docs: Vec<Doc>,
}

fn id_of(header: &str) -> String {
    header
        .split_once('&')
        .map(|(_, rest)| rest.split_whitespace().next().unwrap_or_default().to_owned())
        .unwrap_or_else(|| header.to_owned())
}

fn starts_block(line: &str) -> Option<String> {
    let rest = line.strip_prefix("  ")?;
    if rest.starts_with([' ', '-']) || rest.is_empty() {
        return None;
    }
    rest.split_once(':').map(|(key, _)| key.trim().to_owned())
}

fn parse(text: &str) -> File {
    let mut preamble = Vec::new();
    let mut docs: Vec<Doc> = Vec::new();
    for line in text.lines() {
        if line.starts_with(HEADER) {
            docs.push(Doc {
                id: id_of(line),
                header: line.to_owned(),
                type_line: None,
                blocks: Vec::new(),
            });
            continue;
        }
        let Some(doc) = docs.last_mut() else {
            preamble.push(line.to_owned());
            continue;
        };
        if doc.type_line.is_none() && doc.blocks.is_empty() && !line.starts_with(' ') {
            doc.type_line = Some(line.to_owned());
            continue;
        }
        match starts_block(line) {
            Some(key) => doc.blocks.push(Block {
                key,
                lines: vec![line.to_owned()],
            }),
            None => match doc.blocks.last_mut() {
                Some(block) => block.lines.push(line.to_owned()),
                None => doc.blocks.push(Block {
                    key: String::new(),
                    lines: vec![line.to_owned()],
                }),
            },
        }
    }
    File { preamble, docs }
}

fn write(preamble: &[String], docs: &[Doc]) -> String {
    let mut out = String::new();
    for line in preamble {
        out.push_str(line);
        out.push('\n');
    }
    for doc in docs {
        out.push_str(&doc.header);
        out.push('\n');
        if let Some(type_line) = &doc.type_line {
            out.push_str(type_line);
            out.push('\n');
        }
        for line in doc.blocks.iter().flat_map(|block| &block.lines) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

trait Keyed: Clone + PartialEq {
    fn key(&self) -> &str;
}

impl Keyed for Block {
    fn key(&self) -> &str {
        &self.key
    }
}

impl Keyed for Doc {
    fn key(&self) -> &str {
        &self.id
    }
}

enum Step<T> {
    Take(T),
    Both { base: Option<T>, current: Option<T>, incoming: Option<T> },
}

fn find<'a, T: Keyed>(list: &'a [T], key: &str) -> Option<&'a T> {
    list.iter().find(|item| item.key() == key)
}

fn three_way<T: Keyed>(base: &[T], current: &[T], incoming: &[T], merged: &mut usize) -> Vec<Step<T>> {
    let mut steps: Vec<(String, Step<T>)> = Vec::new();
    for ours in current {
        let key = ours.key();
        let old = find(base, key);
        let theirs = find(incoming, key);
        let step = match (old, theirs) {
            (None, None) => Some(Step::Take(ours.clone())),
            (Some(old), None) if old == ours => {
                *merged += 1;
                None
            }
            (Some(_), None) => Some(Step::Both {
                base: old.cloned(),
                current: Some(ours.clone()),
                incoming: None,
            }),
            (_, Some(theirs)) if theirs == ours => Some(Step::Take(ours.clone())),
            (Some(old), Some(theirs)) if old == ours => {
                *merged += 1;
                Some(Step::Take(theirs.clone()))
            }
            (Some(old), Some(theirs)) if old == theirs => {
                *merged += 1;
                Some(Step::Take(ours.clone()))
            }
            (_, Some(theirs)) => Some(Step::Both {
                base: old.cloned(),
                current: Some(ours.clone()),
                incoming: Some(theirs.clone()),
            }),
        };
        if let Some(step) = step {
            steps.push((key.to_owned(), step));
        }
    }
    for (ix, theirs) in incoming.iter().enumerate() {
        let key = theirs.key();
        if find(current, key).is_some() {
            continue;
        }
        let step = match find(base, key) {
            Some(old) if old == theirs => {
                *merged += 1;
                continue;
            }
            Some(old) => Step::Both {
                base: Some(old.clone()),
                current: None,
                incoming: Some(theirs.clone()),
            },
            None => {
                *merged += 1;
                Step::Take(theirs.clone())
            }
        };
        let after = incoming[..ix]
            .iter()
            .rev()
            .find_map(|earlier| steps.iter().position(|(placed, _)| placed == earlier.key()));
        let at = after.map_or(steps.len(), |at| at + 1);
        steps.insert(at, (key.to_owned(), step));
    }
    steps.into_iter().map(|(_, step)| step).collect()
}

struct Names {
    objects: HashMap<i64, (String, String)>,
}

impl Names {
    fn of(texts: &[&str]) -> Self {
        let mut objects = HashMap::new();
        for text in texts {
            let docs = documents(text);
            let (lookup, _) = lookup_for(&docs);
            objects.extend(object_names(&docs, &lookup.assets));
        }
        Self { objects }
    }

    fn name_and_part(&self, doc: &Doc) -> (String, String) {
        let type_name = doc.type_line.as_deref().unwrap_or_default().trim_end_matches(':').to_owned();
        let part = component_title(&type_name);
        let name = doc
            .id
            .parse::<i64>()
            .ok()
            .and_then(|id| self.objects.get(&id))
            .map_or_else(|| part.clone(), |(name, _)| name.clone());
        (name, part)
    }
}

fn shown(block: Option<&Block>) -> String {
    let Some(block) = block else {
        return GONE.to_owned();
    };
    let first = block.lines.first().map(String::as_str).unwrap_or_default();
    let value = first.split_once(':').map_or(first, |(_, value)| value).trim();
    let text = if value.is_empty() {
        block.lines.iter().skip(1).map(|line| line.trim()).collect::<Vec<_>>().join(" ")
    } else {
        value.to_owned()
    };
    if text.chars().count() > SHOWN_CHARS {
        format!("{}…", text.chars().take(SHOWN_CHARS).collect::<String>())
    } else {
        text
    }
}

fn whole(doc: Option<&Doc>) -> String {
    match doc {
        Some(_) => "Changed".to_owned(),
        None => GONE.to_owned(),
    }
}

fn picked<'a, T>(choices: &[Choice], object: &str, field: &str, current: &'a Option<T>, incoming: &'a Option<T>) -> Option<Option<&'a T>> {
    let choice = choices.iter().find(|choice| choice.object == object && choice.field == field)?;
    Some(match choice.pick {
        Pick::Current => current.as_ref(),
        Pick::Incoming => incoming.as_ref(),
    })
}

pub(crate) fn merge(base: Option<&str>, current: &str, incoming: &str, choices: &[Choice]) -> Merged {
    let base = parse(base.unwrap_or_default());
    let ours = parse(current);
    let theirs = parse(incoming);
    let names = Names::of(&[current, incoming]);
    let mut merged = 0;
    let mut conflicts = Vec::new();
    let mut docs = Vec::new();
    for step in three_way(&base.docs, &ours.docs, &theirs.docs, &mut merged) {
        match step {
            Step::Take(doc) => docs.push(doc),
            Step::Both {
                base: old,
                current: Some(ours_doc),
                incoming: Some(theirs_doc),
            } if ours_doc.header == theirs_doc.header && ours_doc.type_line == theirs_doc.type_line => {
                let old_blocks = old.as_ref().map_or_else(Vec::new, |old| old.blocks.clone());
                let mut blocks = Vec::new();
                for block_step in three_way(&old_blocks, &ours_doc.blocks, &theirs_doc.blocks, &mut merged) {
                    match block_step {
                        Step::Take(block) => blocks.push(block),
                        Step::Both { current, incoming, .. } => {
                            let field = current.as_ref().or(incoming.as_ref()).map(|block| block.key.clone()).unwrap_or_default();
                            match picked(choices, &ours_doc.id, &field, &current, &incoming) {
                                Some(chosen) => blocks.extend(chosen.cloned()),
                                None => {
                                    let (name, part) = names.name_and_part(&ours_doc);
                                    conflicts.push(Conflict {
                                        object: ours_doc.id.clone(),
                                        name,
                                        part,
                                        label: nicify(&field),
                                        field,
                                        current: shown(current.as_ref()),
                                        incoming: shown(incoming.as_ref()),
                                    });
                                    blocks.extend(current);
                                }
                            }
                        }
                    }
                }
                docs.push(Doc { blocks, ..ours_doc });
            }
            Step::Both { current, incoming, .. } => {
                let shown_doc = current.as_ref().or(incoming.as_ref()).cloned().unwrap_or_else(|| unreachable!());
                match picked(choices, &shown_doc.id, WHOLE, &current, &incoming) {
                    Some(chosen) => docs.extend(chosen.cloned()),
                    None => {
                        let (name, part) = names.name_and_part(&shown_doc);
                        conflicts.push(Conflict {
                            object: shown_doc.id.clone(),
                            name,
                            part,
                            field: WHOLE.to_owned(),
                            label: WHOLE_LABEL.to_owned(),
                            current: whole(current.as_ref()),
                            incoming: whole(incoming.as_ref()),
                        });
                        docs.extend(current);
                    }
                }
            }
        }
    }
    let text = conflicts.is_empty().then(|| write(&ours.preamble, &docs));
    Merged { text, conflicts, merged }
}

#[cfg(test)]
mod tests {
    use sternegit_reader_sdk::{Choice, Pick};

    use super::merge;

    const HEAD: &str = "%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n";

    fn prefab(speed: &str, height: &str, scale: &str, extra: &str) -> String {
        format!(
            "{HEAD}--- !u!1 &100\nGameObject:\n  m_Component:\n  - component: {{fileID: 400}}\n  - component: {{fileID: 500}}\n  m_Name: Slime\n--- !u!4 &400\nTransform:\n  m_GameObject: {{fileID: 100}}\n  m_LocalScale: {{x: 1, y: {scale}, z: 1}}\n--- !u!114 &500\nMonoBehaviour:\n  m_GameObject: {{fileID: 100}}\n  moveSpeed: {speed}\n  jumpHeight: {height}\n{extra}"
        )
    }

    #[test]
    fn changes_on_one_side_merge_on_their_own() {
        let base = prefab("2.5", "1.2", "0.25", "");
        let current = prefab("3", "1.2", "0.25", "");
        let incoming = prefab("2.5", "1.6", "0.25", "--- !u!1 &110\nGameObject:\n  m_Name: Crown\n");
        let merged = merge(Some(&base), &current, &incoming, &[]);
        assert!(merged.conflicts.is_empty());
        let text = merged.text.expect("nothing left to choose");
        assert!(text.contains("moveSpeed: 3\n  jumpHeight: 1.6\n"), "{text}");
        assert!(text.ends_with("--- !u!1 &110\nGameObject:\n  m_Name: Crown\n"), "{text}");
        assert_eq!(merged.merged, 3);
    }

    #[test]
    fn a_field_both_sides_changed_differently_is_left_to_choose() {
        let base = prefab("2.5", "1.2", "0.25", "");
        let current = prefab("3", "1.2", "0.25", "");
        let incoming = prefab("4", "1.2", "0.3", "");
        let merged = merge(Some(&base), &current, &incoming, &[]);
        assert_eq!(merged.text, None);
        let conflict = &merged.conflicts[0];
        assert_eq!((conflict.object.as_str(), conflict.field.as_str(), conflict.label.as_str()), ("500", "moveSpeed", "Move Speed"));
        assert_eq!((conflict.name.as_str(), conflict.part.as_str()), ("Slime", "Script"));
        assert_eq!((conflict.current.as_str(), conflict.incoming.as_str()), ("3", "4"));
        let chosen = merge(
            Some(&base),
            &current,
            &incoming,
            &[Choice {
                object: "500".into(),
                field: "moveSpeed".into(),
                pick: Pick::Incoming,
            }],
        );
        let text = chosen.text.expect("every conflict chosen");
        assert!(text.contains("moveSpeed: 4\n") && text.contains("y: 0.3"), "{text}");
    }

    #[test]
    fn an_object_deleted_on_one_side_and_changed_on_the_other_is_chosen_whole() {
        let base = prefab("2.5", "1.2", "0.25", "--- !u!1 &110\nGameObject:\n  m_Name: Crown\n");
        let current = prefab("2.5", "1.2", "0.25", "--- !u!1 &110\nGameObject:\n  m_Name: Big Crown\n");
        let incoming = prefab("2.5", "1.2", "0.25", "");
        let merged = merge(Some(&base), &current, &incoming, &[]);
        let conflict = &merged.conflicts[0];
        assert_eq!((conflict.label.as_str(), conflict.current.as_str(), conflict.incoming.as_str()), ("Whole object", "Changed", "Deleted"));
        let kept = merge(
            Some(&base),
            &current,
            &incoming,
            &[Choice {
                object: "110".into(),
                field: String::new(),
                pick: Pick::Incoming,
            }],
        );
        assert!(!kept.text.expect("chosen").contains("Crown"));
    }
}
