const OPENING: &str = "<<<<<<<";
const BASE: &str = "|||||||";
const SEPARATOR: &str = "=======";
const CLOSING: &str = ">>>>>>>";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Piece {
    Same(String),
    Conflict(Marked),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Marked {
    pub(crate) block: String,
    pub(crate) current: String,
    pub(crate) incoming: String,
    pub(crate) current_label: String,
    pub(crate) incoming_label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Current,
    Base,
    Incoming,
}

pub(crate) fn split_conflicts(text: &str) -> Option<Vec<Piece>> {
    let mut pieces = Vec::new();
    let mut same = String::new();
    let mut open: Option<(Part, Marked)> = None;
    for line in text.split_inclusive('\n') {
        match open.as_mut() {
            None => match marker(line, OPENING) {
                Some(label) => {
                    if !same.is_empty() {
                        pieces.push(Piece::Same(std::mem::take(&mut same)));
                    }
                    open = Some((
                        Part::Current,
                        Marked {
                            block: line.to_owned(),
                            current_label: label,
                            ..Marked::default()
                        },
                    ));
                }
                None if is_marker(line, BASE) || is_marker(line, CLOSING) => return None,
                None => same.push_str(line),
            },
            Some((part, marked)) => {
                marked.block.push_str(line);
                if is_marker(line, OPENING) {
                    return None;
                } else if *part == Part::Current && is_marker(line, BASE) {
                    *part = Part::Base;
                } else if *part != Part::Incoming && is_marker(line, SEPARATOR) {
                    *part = Part::Incoming;
                } else if *part == Part::Incoming && is_marker(line, CLOSING) {
                    marked.incoming_label = marker(line, CLOSING).unwrap_or_default();
                    let (_, marked) = open.take()?;
                    pieces.push(Piece::Conflict(marked));
                } else {
                    match part {
                        Part::Current => marked.current.push_str(line),
                        Part::Base => {}
                        Part::Incoming => marked.incoming.push_str(line),
                    }
                }
            }
        }
    }
    if open.is_some() {
        return None;
    }
    if !same.is_empty() {
        pieces.push(Piece::Same(same));
    }
    Some(pieces)
}

pub(crate) fn has_markers(text: &str) -> bool {
    text.split_inclusive('\n')
        .any(|line| is_marker(line, OPENING) || is_marker(line, CLOSING))
}

fn is_marker(line: &str, prefix: &str) -> bool {
    marker(line, prefix).is_some()
}

fn marker(line: &str, prefix: &str) -> Option<String> {
    let content = line.trim_end_matches(['\n', '\r']);
    let rest = content.strip_prefix(prefix)?;
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Marked, Piece, has_markers, split_conflicts};

    const TWO_WAY: &str = "start\n<<<<<<< HEAD\nours\n=======\ntheirs\nmore\n>>>>>>> 3f2a91c (Fix it)\nend\n";

    #[test]
    fn a_conflict_splits_into_both_sides_between_the_shared_lines() {
        assert_eq!(
            split_conflicts(TWO_WAY),
            Some(vec![
                Piece::Same("start\n".into()),
                Piece::Conflict(Marked {
                    block: "<<<<<<< HEAD\nours\n=======\ntheirs\nmore\n>>>>>>> 3f2a91c (Fix it)\n"
                        .into(),
                    current: "ours\n".into(),
                    incoming: "theirs\nmore\n".into(),
                    current_label: "HEAD".into(),
                    incoming_label: "3f2a91c (Fix it)".into(),
                }),
                Piece::Same("end\n".into()),
            ])
        );
        assert!(has_markers(TWO_WAY));
    }

    #[test]
    fn the_common_ancestor_of_diff3_is_left_out_of_both_sides() {
        let text = "<<<<<<< ours\r\na\r\n||||||| base\r\nb\r\n=======\r\nc\r\n>>>>>>> theirs\r\n";
        let pieces = split_conflicts(text).expect("well-formed markers");
        let [Piece::Conflict(marked)] = pieces.as_slice() else {
            panic!("one conflict expected");
        };
        assert_eq!(
            (marked.current.as_str(), marked.incoming.as_str()),
            ("a\r\n", "c\r\n")
        );
        assert_eq!(marked.block, text);
    }

    #[test]
    fn markers_out_of_order_are_not_a_conflict() {
        assert_eq!(split_conflicts("a\n>>>>>>> theirs\n"), None);
        assert_eq!(split_conflicts("<<<<<<< ours\na\n"), None);
        assert_eq!(split_conflicts("<<<<<<< a\n<<<<<<< b\n"), None);
        let shared = "plain\n======= in shared text\n";
        assert_eq!(
            split_conflicts(shared),
            Some(vec![Piece::Same(shared.into())])
        );
        assert!(!has_markers("<<<<<<<< eight\nfine\n"));
    }
}
