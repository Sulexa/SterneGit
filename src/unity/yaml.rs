use std::borrow::Cow;
use std::str::Chars;

const DEEPEST_BLOCK: usize = 100;
const DEEPEST_FLOW: usize = 64;
const BLANKS: [char; 4] = [' ', '\t', '\r', '\n'];
const UNITY_TAG: [&str; 3] = ["%TAG", "!u!", "tag:unity3d.com,2011:"];
const REPLACEMENT: char = '\u{fffd}';

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    Scalar(String),
    Map(Vec<(String, Node)>),
    List(Vec<Node>),
}

impl Node {
    pub(crate) fn get(&self, key: &str) -> Option<&Node> {
        self.entries().iter().find(|(name, _)| name == key).map(|(_, value)| value)
    }

    pub(crate) fn at(&self, path: &[&str]) -> Option<&Node> {
        path.iter().try_fold(self, |node, key| node.get(key))
    }

    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Node::Scalar(text) => Some(text),
            Node::Map(_) | Node::List(_) => None,
        }
    }

    pub(crate) fn number(&self) -> Option<f64> {
        self.text()?.trim_matches(BLANKS).parse().ok()
    }

    pub(crate) fn integer(&self) -> Option<i64> {
        self.text()?.trim_matches(BLANKS).parse().ok()
    }

    pub(crate) fn entries(&self) -> &[(String, Node)] {
        match self {
            Node::Map(entries) => entries,
            Node::Scalar(_) | Node::List(_) => &[],
        }
    }

    pub(crate) fn items(&self) -> &[Node] {
        match self {
            Node::List(items) => items,
            Node::Scalar(_) | Node::Map(_) => &[],
        }
    }

    fn empty() -> Node {
        Node::Scalar(String::new())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Document {
    pub(crate) class_id: u32,
    pub(crate) file_id: i64,
    pub(crate) stripped: bool,
    pub(crate) type_name: String,
    pub(crate) body: Node,
    pub(crate) first_line: usize,
    pub(crate) last_line: usize,
}

pub(crate) fn is_unity_yaml(text: &str) -> bool {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    text.starts_with("%YAML")
        && text
            .lines()
            .take_while(|line| line.starts_with('%'))
            .any(|line| line.split_whitespace().eq(UNITY_TAG))
}

pub(crate) fn documents(text: &str) -> Vec<Document> {
    let lines: Vec<&str> = text.lines().collect();
    let starts: Vec<usize> =
        lines.iter().enumerate().filter(|(_, line)| is_header(line)).map(|(at, _)| at).collect();
    let ends = starts.iter().skip(1).copied().chain([lines.len()]);
    starts.iter().zip(ends).map(|(&start, end)| document(&lines, start, end)).collect()
}

pub(crate) fn plain(text: &str) -> Node {
    let lines: Vec<&str> = text.lines().collect();
    Node::Map(Block::read(&lines))
}

fn document(lines: &[&str], start: usize, end: usize) -> Document {
    let header = Header::read(lines.get(start).copied().unwrap_or_default());
    let (type_name, body) = typed(Block::read(lines.get(start + 1..end).unwrap_or_default()));
    Document {
        class_id: header.class_id,
        file_id: header.file_id,
        stripped: header.stripped,
        type_name,
        body,
        first_line: start + 1,
        last_line: end,
    }
}

fn typed(entries: Vec<(String, Node)>) -> (String, Node) {
    match entries.into_iter().next() {
        Some((name, Node::Scalar(text))) if text.is_empty() => (name, Node::Map(Vec::new())),
        Some((name, body)) => (name, body),
        None => (String::new(), Node::Map(Vec::new())),
    }
}

fn is_header(line: &str) -> bool {
    line.strip_prefix("---").is_some_and(|rest| rest.is_empty() || rest.starts_with(BLANKS))
}

struct Header {
    class_id: u32,
    file_id: i64,
    stripped: bool,
}

impl Header {
    fn read(line: &str) -> Header {
        let mut header = Header { class_id: 0, file_id: 0, stripped: false };
        for word in line.split_whitespace().skip(1) {
            if let Some(id) = word.strip_prefix("!u!") {
                header.class_id = id.parse().unwrap_or(0);
            } else if let Some(id) = word.strip_prefix('&') {
                header.file_id = id.parse().unwrap_or(0);
            } else if word == "stripped" {
                header.stripped = true;
            }
        }
        header
    }
}

enum Frame {
    Map { indent: usize, entries: Vec<(String, Node)>, waiting: Option<String> },
    List { indent: usize, items: Vec<Node>, waiting: bool },
}

impl Frame {
    fn map(indent: usize) -> Frame {
        Frame::Map { indent, entries: Vec::new(), waiting: None }
    }

    fn list(indent: usize) -> Frame {
        Frame::List { indent, items: Vec::new(), waiting: true }
    }

    fn indent(&self) -> usize {
        match self {
            Frame::Map { indent, .. } | Frame::List { indent, .. } => *indent,
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, Frame::List { .. })
    }

    fn is_waiting(&self) -> bool {
        match self {
            Frame::Map { waiting, .. } => waiting.is_some(),
            Frame::List { waiting, .. } => *waiting,
        }
    }

    fn closes_before(&self, col: usize, dash: bool) -> bool {
        self.indent() > col || (self.is_list() && self.indent() == col && !dash)
    }

    fn open_slot(&mut self, key: String) {
        self.settle();
        match self {
            Frame::Map { waiting, .. } => *waiting = Some(key),
            Frame::List { waiting, .. } => *waiting = true,
        }
    }

    fn fill(&mut self, node: Node) {
        match self {
            Frame::Map { entries, waiting, .. } => entries.push((waiting.take().unwrap_or_default(), node)),
            Frame::List { items, waiting, .. } => {
                *waiting = false;
                items.push(node);
            }
        }
    }

    fn add(&mut self, key: String, node: Node) {
        self.open_slot(key);
        self.fill(node);
    }

    fn settle(&mut self) {
        if self.is_waiting() {
            self.fill(Node::empty());
        }
    }

    fn into_node(self) -> Node {
        match self {
            Frame::Map { entries, .. } => Node::Map(entries),
            Frame::List { items, .. } => Node::List(items),
        }
    }
}

struct Block<'a> {
    lines: &'a [&'a str],
    next: usize,
    root: Frame,
    open: Vec<Frame>,
}

impl<'a> Block<'a> {
    fn read(lines: &'a [&'a str]) -> Vec<(String, Node)> {
        let mut block = Block { lines, next: 0, root: Frame::map(0), open: Vec::new() };
        while let Some(line) = lines.get(block.next).copied() {
            block.next += 1;
            let (indent, content) = split_indent(line);
            if !content.is_empty() {
                block.place(indent, content);
            }
        }
        block.finish()
    }

    fn top(&mut self) -> &mut Frame {
        self.open.last_mut().unwrap_or(&mut self.root)
    }

    fn place(&mut self, col: usize, content: &'a str) {
        self.close_deeper(col, dash_rest(content).is_some());
        let Some((col, content)) = self.enter_items(col, content) else {
            return;
        };
        match split_key(content) {
            Some((key, value)) => self.place_entry(col, key, value, content),
            None => self.place_scalar(col, content),
        }
    }

    fn close_deeper(&mut self, col: usize, dash: bool) {
        while self.open.last().is_some_and(|frame| frame.closes_before(col, dash)) {
            self.close_top();
        }
    }

    fn close_top(&mut self) {
        if let Some(mut frame) = self.open.pop() {
            frame.settle();
            let node = frame.into_node();
            self.top().fill(node);
        }
    }

    fn enter_items(&mut self, mut col: usize, mut content: &'a str) -> Option<(usize, &'a str)> {
        while let Some(rest) = dash_rest(content) {
            if !self.open_item(col, content) {
                return None;
            }
            let item = rest.trim_start_matches(' ');
            if item.is_empty() {
                return None;
            }
            col += content.len() - item.len();
            content = item;
        }
        Some((col, content))
    }

    fn open_item(&mut self, col: usize, raw: &str) -> bool {
        let top = self.top();
        if top.is_list() && top.indent() == col {
            top.open_slot(String::new());
            return true;
        }
        if !top.is_waiting() {
            top.open_slot(String::new());
        }
        self.open_frame(Frame::list(col), raw)
    }

    fn open_frame(&mut self, frame: Frame, raw: &str) -> bool {
        if self.open.len() < DEEPEST_BLOCK {
            self.open.push(frame);
            return true;
        }
        self.top().fill(Node::Scalar(raw.to_string()));
        false
    }

    fn place_entry(&mut self, col: usize, key: String, value: &'a str, raw: &'a str) {
        let top = self.top();
        if top.is_list() || (top.is_waiting() && top.indent() < col) {
            if !top.is_waiting() {
                top.open_slot(String::new());
            }
            if !self.open_frame(Frame::map(col), raw) {
                return;
            }
        }
        if value.is_empty() {
            self.top().open_slot(key);
        } else {
            let node = self.value(value, col);
            self.top().add(key, node);
        }
    }

    fn place_scalar(&mut self, col: usize, content: &'a str) {
        let top = self.top();
        let threshold = top.indent();
        if top.is_waiting() && threshold < col {
            let node = self.value(content, threshold);
            self.top().fill(node);
        } else if top.is_list() {
            let node = self.value(content, threshold);
            self.top().add(String::new(), node);
        } else {
            top.add(content.to_string(), Node::empty());
        }
    }

    fn value(&mut self, first: &'a str, threshold: usize) -> Node {
        let text = self.gather(first, threshold);
        scalar_or_flow(&text)
    }

    fn gather(&mut self, first: &'a str, threshold: usize) -> Cow<'a, str> {
        let Some(mut found) = self.continuation(threshold) else {
            return Cow::Borrowed(first);
        };
        let mut fold = Fold { text: first.to_string(), blanks: 0, escapes: first.starts_with('"') };
        loop {
            let (at, line) = found;
            fold.blanks += at - self.next;
            fold.push(line);
            self.next = at + 1;
            match self.continuation(threshold) {
                Some(more) => found = more,
                None => return Cow::Owned(fold.text),
            }
        }
    }

    fn continuation(&self, threshold: usize) -> Option<(usize, &'a str)> {
        let mut at = self.next;
        loop {
            let (indent, content) = split_indent(self.lines.get(at).copied()?);
            if !content.is_empty() {
                return (indent > threshold).then_some((at, content));
            }
            at += 1;
        }
    }

    fn finish(mut self) -> Vec<(String, Node)> {
        while !self.open.is_empty() {
            self.close_top();
        }
        self.root.settle();
        match self.root {
            Frame::Map { entries, .. } => entries,
            Frame::List { .. } => Vec::new(),
        }
    }
}

struct Fold {
    text: String,
    blanks: usize,
    escapes: bool,
}

impl Fold {
    fn push(&mut self, line: &str) {
        if self.escapes && ends_in_escape(&self.text) {
            self.text.pop();
        } else if self.blanks == 0 {
            self.text.push(' ');
        }
        for _ in 0..self.blanks {
            self.text.push('\n');
        }
        self.blanks = 0;
        self.text.push_str(line);
    }
}

fn ends_in_escape(text: &str) -> bool {
    text.bytes().rev().take_while(|&byte| byte == b'\\').count() % 2 == 1
}

fn split_indent(line: &str) -> (usize, &str) {
    let content = line.trim_start_matches(' ');
    (line.len() - content.len(), content.trim_end_matches(BLANKS))
}

fn dash_rest(content: &str) -> Option<&str> {
    let rest = content.strip_prefix('-')?;
    (rest.is_empty() || rest.starts_with(' ')).then_some(rest)
}

fn split_key(content: &str) -> Option<(String, &str)> {
    match content.as_bytes().first()? {
        b'{' | b'[' => None,
        b'\'' | b'"' => {
            let end = quoted_end(content)?;
            let rest = content.get(end..)?.trim_start_matches(' ').strip_prefix(':')?;
            let value = (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim_start_matches(' '))?;
            Some((unquote(content.get(..end)?), value))
        }
        _ => {
            let colon = content.find(": ").or_else(|| content.ends_with(':').then(|| content.len() - 1))?;
            let key = content.get(..colon)?.trim_end_matches(BLANKS);
            Some((key.to_string(), content.get(colon + 1..)?.trim_start_matches(' ')))
        }
    }
}

fn scalar_or_flow(text: &str) -> Node {
    match text.as_bytes().first() {
        Some(b'{' | b'[') => Flow { text, at: 0 }.node(0),
        Some(b'\'' | b'"') => Node::Scalar(unquote(text)),
        _ => Node::Scalar(text.to_string()),
    }
}

fn quoted_end(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let quote = *bytes.first()?;
    let mut at = 1;
    while let Some(&byte) = bytes.get(at) {
        if quote == b'"' && byte == b'\\' {
            at += 2;
            continue;
        }
        if byte == quote {
            if quote == b'\'' && bytes.get(at + 1) == Some(&b'\'') {
                at += 2;
                continue;
            }
            return Some(at + 1);
        }
        at += 1;
    }
    None
}

fn unquote(text: &str) -> String {
    match text.as_bytes().first() {
        Some(b'\'') => unquote_single(text.get(1..).unwrap_or_default()),
        Some(b'"') => unquote_double(text.get(1..).unwrap_or_default()),
        _ => text.to_string(),
    }
}

fn unquote_single(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find('\'') {
        out.push_str(&rest[..at]);
        match rest[at + 1..].strip_prefix('\'') {
            Some(after) => {
                out.push('\'');
                rest = after;
            }
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn unquote_double(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some(code) => out.push(escaped(code, &mut chars)),
                None => break,
            },
            _ => out.push(c),
        }
    }
    out
}

fn escaped(code: char, chars: &mut Chars) -> char {
    match code {
        '0' => '\0',
        'a' => '\u{7}',
        'b' => '\u{8}',
        't' => '\t',
        'n' => '\n',
        'v' => '\u{b}',
        'f' => '\u{c}',
        'r' => '\r',
        'e' => '\u{1b}',
        'N' => '\u{85}',
        '_' => '\u{a0}',
        'L' => '\u{2028}',
        'P' => '\u{2029}',
        'x' => code_point(chars, 2),
        'u' => code_point(chars, 4),
        'U' => code_point(chars, 8),
        other => other,
    }
}

fn code_point(chars: &mut Chars, digits: usize) -> char {
    let high = hex(chars, digits);
    let low = if (0xd800..0xdc00).contains(&high) { low_surrogate(chars) } else { None };
    let value = match low {
        Some(low) => 0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00),
        None => high,
    };
    char::from_u32(value).unwrap_or(REPLACEMENT)
}

fn low_surrogate(chars: &mut Chars) -> Option<u32> {
    let mut ahead = chars.clone();
    if ahead.next() != Some('\\') || ahead.next() != Some('u') {
        return None;
    }
    let low = hex(&mut ahead, 4);
    if !(0xdc00..0xe000).contains(&low) {
        return None;
    }
    *chars = ahead;
    Some(low)
}

fn hex(chars: &mut Chars, digits: usize) -> u32 {
    let mut value = 0u32;
    for _ in 0..digits.min(8) {
        match chars.clone().next().and_then(|c| c.to_digit(16)) {
            Some(digit) => {
                value = value * 16 + digit;
                chars.next();
            }
            None => break,
        }
    }
    value
}

struct Flow<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Flow<'a> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn skip_blanks(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.at += 1;
        }
    }

    fn node(&mut self, depth: usize) -> Node {
        self.skip_blanks();
        match self.peek() {
            Some(b'{' | b'[') if depth >= DEEPEST_FLOW => Node::Scalar(self.balanced().to_string()),
            Some(b'{') => self.map(depth + 1),
            Some(b'[') => self.list(depth + 1),
            Some(b'\'' | b'"') => Node::Scalar(self.quoted()),
            _ => Node::Scalar(self.plain(false)),
        }
    }

    fn map(&mut self, depth: usize) -> Node {
        self.at += 1;
        let mut entries = Vec::new();
        loop {
            self.skip_blanks();
            match self.peek() {
                None => break,
                Some(b'}') => {
                    self.at += 1;
                    break;
                }
                Some(b',' | b']') => self.at += 1,
                Some(_) => {
                    let key = self.key();
                    let value = self.entry_value(depth);
                    entries.push((key, value));
                }
            }
        }
        Node::Map(entries)
    }

    fn entry_value(&mut self, depth: usize) -> Node {
        self.skip_blanks();
        if self.peek() != Some(b':') {
            return Node::empty();
        }
        self.at += 1;
        self.skip_blanks();
        match self.peek() {
            None | Some(b',' | b'}' | b']') => Node::empty(),
            Some(_) => self.node(depth),
        }
    }

    fn list(&mut self, depth: usize) -> Node {
        self.at += 1;
        let mut items = Vec::new();
        loop {
            self.skip_blanks();
            match self.peek() {
                None => break,
                Some(b']') => {
                    self.at += 1;
                    break;
                }
                Some(b',' | b'}') => self.at += 1,
                Some(_) => items.push(self.node(depth)),
            }
        }
        Node::List(items)
    }

    fn key(&mut self) -> String {
        match self.peek() {
            Some(b'\'' | b'"') => self.quoted(),
            _ => self.plain(true),
        }
    }

    fn quoted(&mut self) -> String {
        let rest = self.text.get(self.at..).unwrap_or_default();
        let end = quoted_end(rest).unwrap_or(rest.len());
        self.at += end.max(1);
        unquote(rest.get(..end).unwrap_or_default())
    }

    fn plain(&mut self, key: bool) -> String {
        let text = self.text;
        let bytes = text.as_bytes();
        let start = self.at;
        while let Some(&byte) = bytes.get(self.at) {
            let ends_key = key && byte == b':' && ends_plain_key(bytes.get(self.at + 1).copied());
            if ends_key || matches!(byte, b',' | b'}' | b']') {
                break;
            }
            self.at += 1;
        }
        text.get(start..self.at).unwrap_or_default().trim_matches(BLANKS).to_string()
    }

    fn balanced(&mut self) -> &'a str {
        let text = self.text;
        let bytes = text.as_bytes();
        let start = self.at;
        let mut depth = 0usize;
        while let Some(&byte) = bytes.get(self.at) {
            match byte {
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        self.at += 1;
                        break;
                    }
                }
                b'\'' | b'"' => {
                    let rest = text.get(self.at..).unwrap_or_default();
                    self.at += quoted_end(rest).unwrap_or(rest.len()).max(1);
                    continue;
                }
                _ => {}
            }
            self.at += 1;
        }
        text.get(start..self.at).unwrap_or_default()
    }
}

fn ends_plain_key(next: Option<u8>) -> bool {
    matches!(next, None | Some(b' ' | b'\t' | b'\r' | b'\n' | b',' | b'}' | b']'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn text(source: &str) -> String {
        let mut out = String::new();
        for line in source.lines() {
            if let Some(kept) = line.trim_start().strip_prefix('|') {
                out.push_str(kept);
                out.push('\n');
            }
        }
        out
    }

    fn unity(body: &str) -> String {
        format!("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n{}", text(body))
    }

    fn only(source: &str) -> Document {
        let mut parsed = documents(&unity(source));
        assert_eq!(parsed.len(), 1, "{parsed:#?}");
        parsed.remove(0)
    }

    fn scalar(text: &str) -> Node {
        Node::Scalar(text.to_string())
    }

    fn entry(key: &str, value: Node) -> (String, Node) {
        (key.to_string(), value)
    }

    fn text_at<'a>(node: &'a Node, path: &[&str]) -> Option<&'a str> {
        node.at(path)?.text()
    }

    fn number_at(node: &Node, path: &[&str]) -> Option<f64> {
        node.at(path)?.number()
    }

    fn items_at<'a>(node: &'a Node, path: &[&str]) -> &'a [Node] {
        node.at(path).map(Node::items).unwrap_or_default()
    }

    fn depth(node: &Node) -> usize {
        let inner = node.entries().iter().map(|(_, value)| value).chain(node.items());
        1 + inner.map(depth).max().unwrap_or(0)
    }

    fn empty_list() -> Node {
        Node::List(Vec::new())
    }

    #[test]
    fn a_game_object_lists_its_components_at_the_same_indent_as_the_key() {
        let object = only(
            "
            |--- !u!1 &6106237841907316552
            |GameObject:
            |  m_ObjectHideFlags: 0
            |  m_CorrespondingSourceObject: {fileID: 0}
            |  serializedVersion: 6
            |  m_Component:
            |  - component: {fileID: 4823015530731772630}
            |  - component: {fileID: 2290457312861453377}
            |  m_Layer: 0
            |  m_Name: Player
            |  m_TagString: Untagged
            |  m_IsActive: 1
            ",
        );
        assert_eq!(object.class_id, 1);
        assert_eq!(object.file_id, 6106237841907316552);
        assert!(!object.stripped);
        assert_eq!(object.type_name, "GameObject");
        let components = items_at(&object.body, &["m_Component"]).iter();
        let ids: Vec<_> = components.filter_map(|item| text_at(item, &["component", "fileID"])).collect();
        assert_eq!(ids, ["4823015530731772630", "2290457312861453377"]);
        assert_eq!(object.body.get("m_Layer"), Some(&scalar("0")));
        assert_eq!(text_at(&object.body, &["m_Name"]), Some("Player"));
        assert_eq!(number_at(&object.body, &["m_IsActive"]), Some(1.0));
        let keys: Vec<_> = object.body.entries().iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "m_ObjectHideFlags",
                "m_CorrespondingSourceObject",
                "serializedVersion",
                "m_Component",
                "m_Layer",
                "m_Name",
                "m_TagString",
                "m_IsActive",
            ]
        );
    }

    #[test]
    fn a_transform_reads_its_flow_mappings_and_keeps_number_text() {
        let transform = only(
            "
            |--- !u!4 &4823015530731772630
            |Transform:
            |  m_GameObject: {fileID: 6106237841907316552}
            |  m_LocalRotation: {x: 0, y: 0.70710677, z: 0, w: 0.70710677}
            |  m_LocalPosition: {x: 1.5, y: -0, z: 1e-05}
            |  m_LocalScale: {x: 0.99999994, y: 1, z: 1}
            |  m_Children: []
            |  m_Father: {fileID: 0}
            ",
        );
        assert_eq!(transform.type_name, "Transform");
        let position = vec![entry("x", scalar("1.5")), entry("y", scalar("-0")), entry("z", scalar("1e-05"))];
        assert_eq!(transform.body.get("m_LocalPosition"), Some(&Node::Map(position)));
        assert_eq!(number_at(&transform.body, &["m_LocalPosition", "z"]), Some(0.00001));
        let y = number_at(&transform.body, &["m_LocalPosition", "y"]);
        assert!(y.is_some_and(|y| y == 0.0 && y.is_sign_negative()));
        assert_eq!(text_at(&transform.body, &["m_LocalScale", "x"]), Some("0.99999994"));
        assert_eq!(number_at(&transform.body, &["m_LocalRotation", "w"]), Some(0.70710677));
        assert_eq!(transform.body.get("m_Children"), Some(&empty_list()));
        assert_eq!(text_at(&transform.body, &["m_Father", "fileID"]), Some("0"));
    }

    #[test]
    fn a_mono_behaviour_reads_lists_of_flow_mappings_and_nested_maps() {
        let behaviour = only(
            "
            |--- !u!114 &2290457312861453377
            |MonoBehaviour:
            |  m_GameObject: {fileID: 6106237841907316552}
            |  m_Enabled: 1
            |  m_Script: {fileID: 11500000, guid: c3a1e9f07b2d4c5e8f6a9b0c1d2e3f4a, type: 3}
            |  m_Name:
            |  m_EditorClassIdentifier: Assembly-CSharp::Game.Player
            |  speed: 4.5
            |  targets:
            |  - {fileID: 11400000, guid: c3a1e9f07b2d4c5e8f6a9b0c1d2e3f4a, type: 2}
            |  - {fileID: 0}
            |  settings:
            |    jump:
            |      height: 2
            |      curve: {serializedVersion: 2, m_Curve: [], m_PreInfinity: 2}
            |    label: Hello: world
            |    texture: Assets/Art/slime.png
            |  waypoints: [{x: 0, y: 1}, {x: 2, y: 3}]
            |  ids: [0, 1, 2]
            |  type: {class: Wrapper, ns: , asm: Assembly-CSharp}
            ",
        );
        let body = &behaviour.body;
        assert_eq!(behaviour.class_id, 114);
        assert_eq!(text_at(body, &["m_Script", "guid"]), Some("c3a1e9f07b2d4c5e8f6a9b0c1d2e3f4a"));
        assert_eq!(body.get("m_Name"), Some(&scalar("")));
        assert_eq!(text_at(body, &["m_EditorClassIdentifier"]), Some("Assembly-CSharp::Game.Player"));
        let targets = items_at(body, &["targets"]);
        assert_eq!(targets.len(), 2);
        assert_eq!(text_at(&targets[0], &["type"]), Some("2"));
        assert_eq!(text_at(&targets[1], &["fileID"]), Some("0"));
        assert_eq!(number_at(body, &["settings", "jump", "height"]), Some(2.0));
        assert_eq!(body.at(&["settings", "jump", "curve", "m_Curve"]), Some(&empty_list()));
        assert_eq!(text_at(body, &["settings", "jump", "curve", "m_PreInfinity"]), Some("2"));
        assert_eq!(text_at(body, &["settings", "label"]), Some("Hello: world"));
        assert_eq!(text_at(body, &["settings", "texture"]), Some("Assets/Art/slime.png"));
        let waypoints = items_at(body, &["waypoints"]).iter();
        let heights: Vec<_> = waypoints.filter_map(|point| text_at(point, &["y"])).collect();
        assert_eq!(heights, ["1", "3"]);
        assert_eq!(body.get("ids"), Some(&Node::List(vec![scalar("0"), scalar("1"), scalar("2")])));
        assert_eq!(body.at(&["type", "ns"]), Some(&scalar("")));
        assert_eq!(text_at(body, &["type", "asm"]), Some("Assembly-CSharp"));
    }

    #[test]
    fn prefab_modifications_are_block_maps_inside_list_items() {
        let instance = only(
            "
            |--- !u!1001 &100100000
            |PrefabInstance:
            |  m_ObjectHideFlags: 0
            |  serializedVersion: 2
            |  m_Modification:
            |    serializedVersion: 3
            |    m_TransformParent: {fileID: 0}
            |    m_Modifications:
            |    - target: {fileID: 400000, guid: 9f2c, type: 3}
            |      propertyPath: m_LocalPosition.x
            |      value: 6
            |      objectReference: {fileID: 0}
            |    - target: {fileID: 100000, guid: 9f2c, type: 3}
            |      propertyPath: m_Name
            |      value:
            |      objectReference: {fileID: 0}
            |    m_RemovedComponents: []
            |  m_SourcePrefab: {fileID: 100100000, guid: 9f2c, type: 3}
            ",
        );
        let modifications = items_at(&instance.body, &["m_Modification", "m_Modifications"]);
        assert_eq!(modifications.len(), 2);
        assert_eq!(text_at(&modifications[0], &["propertyPath"]), Some("m_LocalPosition.x"));
        assert_eq!(number_at(&modifications[0], &["value"]), Some(6.0));
        assert_eq!(text_at(&modifications[0], &["target", "guid"]), Some("9f2c"));
        assert_eq!(modifications[1].get("value"), Some(&scalar("")));
        assert_eq!(modifications[1].entries().len(), 4);
        assert_eq!(instance.body.at(&["m_Modification", "m_RemovedComponents"]), Some(&empty_list()));
        assert_eq!(text_at(&instance.body, &["m_SourcePrefab", "fileID"]), Some("100100000"));
    }

    #[test]
    fn material_properties_are_single_key_maps_inside_lists() {
        let material = only(
            "
            |--- !u!21 &2100000
            |Material:
            |  serializedVersion: 8
            |  m_Name: Slime
            |  m_SavedProperties:
            |    serializedVersion: 3
            |    m_TexEnvs:
            |    - _BumpMap:
            |        m_Texture: {fileID: 0}
            |        m_Scale: {x: 1, y: 1}
            |        m_Offset: {x: 0, y: 0}
            |    - _MainTex:
            |        m_Texture: {fileID: 2800000, guid: 6e1f, type: 3}
            |        m_Scale: {x: 2, y: 2}
            |        m_Offset: {x: 0, y: 0}
            |    - _Detail: {m_Texture: {fileID: 2800000, guid: 6e1f, type: 3}, m_Offset: {x: 0, y: 0.25}}
            |    m_Ints: []
            |    m_Floats:
            |    - _Glossiness: 0.5
            |    - _Metallic: 0
            |    m_Colors:
            |    - _Color: {r: 1, g: 0.5, b: 1, a: 1}
            |  m_BuildTextureStacks: []
            ",
        );
        let properties = material.body.get("m_SavedProperties").cloned().unwrap_or(Node::empty());
        let textures = items_at(&properties, &["m_TexEnvs"]);
        assert_eq!(textures.len(), 3);
        assert_eq!(text_at(&textures[1], &["_MainTex", "m_Texture", "guid"]), Some("6e1f"));
        assert_eq!(number_at(&textures[1], &["_MainTex", "m_Scale", "x"]), Some(2.0));
        assert_eq!(text_at(&textures[2], &["_Detail", "m_Texture", "type"]), Some("3"));
        assert_eq!(number_at(&textures[2], &["_Detail", "m_Offset", "y"]), Some(0.25));
        let glossiness = Node::Map(vec![entry("_Glossiness", scalar("0.5"))]);
        let metallic = Node::Map(vec![entry("_Metallic", scalar("0"))]);
        assert_eq!(items_at(&properties, &["m_Floats"]), [glossiness, metallic]);
        assert_eq!(number_at(&items_at(&properties, &["m_Colors"])[0], &["_Color", "g"]), Some(0.5));
        assert_eq!(properties.get("m_Ints"), Some(&empty_list()));
        assert_eq!(material.body.get("m_BuildTextureStacks"), Some(&empty_list()));
    }

    #[test]
    fn animation_curves_nest_lists_inside_list_items() {
        let clip = only(
            "
            |--- !u!74 &7400000
            |AnimationClip:
            |  m_Name: Bounce
            |  m_PositionCurves:
            |  - curve:
            |      serializedVersion: 2
            |      m_Curve:
            |      - serializedVersion: 3
            |        time: 0
            |        value: {x: 0, y: 1, z: 0}
            |        inSlope: {x: 0, y: 0, z: 0}
            |      - serializedVersion: 3
            |        time: 0.5
            |        value: {x: 0, y: 2, z: 0}
            |      m_PreInfinity: 2
            |      m_PostInfinity: 2
            |    path: Body
            |  - curve:
            |      serializedVersion: 2
            |      m_Curve: []
            |    path: Body/Hat
            |  m_SampleRate: 60
            ",
        );
        let curves = items_at(&clip.body, &["m_PositionCurves"]);
        assert_eq!(curves.len(), 2);
        let keys = items_at(&curves[0], &["curve", "m_Curve"]);
        assert_eq!(keys.len(), 2);
        assert_eq!(number_at(&keys[1], &["time"]), Some(0.5));
        assert_eq!(text_at(&keys[1], &["value", "y"]), Some("2"));
        assert_eq!(text_at(&keys[0], &["inSlope", "x"]), Some("0"));
        assert_eq!(text_at(&curves[0], &["curve", "m_PostInfinity"]), Some("2"));
        assert_eq!(text_at(&curves[0], &["path"]), Some("Body"));
        assert_eq!(curves[1].at(&["curve", "m_Curve"]), Some(&empty_list()));
        assert_eq!(text_at(&curves[1], &["path"]), Some("Body/Hat"));
        assert_eq!(number_at(&clip.body, &["m_SampleRate"]), Some(60.0));
    }

    #[test]
    fn a_list_indented_deeper_than_its_key_works_too() {
        let root = plain(&text(
            "
            |a:
            |    - x
            |    -   y: 1
            |        z: 2
            |    -
            |      w: 3
            |    -
            |    - - inner
            |      - more
            |b: 4
            ",
        ));
        let items = items_at(&root, &["a"]);
        assert_eq!(items.len(), 5);
        assert_eq!(items[0], scalar("x"));
        assert_eq!(items[1], Node::Map(vec![entry("y", scalar("1")), entry("z", scalar("2"))]));
        assert_eq!(text_at(&items[2], &["w"]), Some("3"));
        assert_eq!(items[3], scalar(""));
        assert_eq!(items[4], Node::List(vec![scalar("inner"), scalar("more")]));
        assert_eq!(text_at(&root, &["b"]), Some("4"));
    }

    #[test]
    fn a_list_at_the_same_indent_as_its_key_ends_at_the_next_key() {
        let root = plain("names:\n- one\n- two\ncount: 2\nlast:\n- {a: 1}\n");
        assert_eq!(root.get("names"), Some(&Node::List(vec![scalar("one"), scalar("two")])));
        assert_eq!(text_at(&root, &["count"]), Some("2"));
        assert_eq!(items_at(&root, &["last"]).len(), 1);
        assert_eq!(root.entries().len(), 3);
    }

    #[test]
    fn single_quoted_json_turns_doubled_quotes_into_one() {
        let graph = only(
            r#"
            |--- !u!114 &11400000
            |MonoBehaviour:
            |  _json: '{"graph":{"elements":[{"$type":"Unity.VisualScripting.Update","name":''hello''}]}}'
            |  quote: 'it''s: fine'
            |  'quoted key': ''''
            "#,
        );
        assert_eq!(
            text_at(&graph.body, &["_json"]),
            Some(r#"{"graph":{"elements":[{"$type":"Unity.VisualScripting.Update","name":'hello'}]}}"#)
        );
        assert_eq!(text_at(&graph.body, &["quote"]), Some("it's: fine"));
        assert_eq!(text_at(&graph.body, &["quoted key"]), Some("'"));
    }

    #[test]
    fn quoted_and_plain_scalars_fold_their_continuation_lines() {
        let label = only(
            r#"
            |--- !u!114 &1
            |MonoBehaviour:
            |  single: 'The quick brown fox
            |    jumps over
            |
            |    the lazy dog'
            |  double: "Caf\u00E9 \"menu\"\tand a line that wraps \
            |    \ here\nnext \U0001F600 \uD83D\uDE00"
            |  plain: a long plain
            |    text that wraps
            |
            |  after: 1
            |  items:
            |  - first long
            |    item
            |  - second
            "#,
        );
        let body = &label.body;
        assert_eq!(text_at(body, &["single"]), Some("The quick brown fox jumps over\nthe lazy dog"));
        assert_eq!(
            text_at(body, &["double"]),
            Some("Café \"menu\"\tand a line that wraps  here\nnext \u{1F600} \u{1F600}")
        );
        assert_eq!(text_at(body, &["plain"]), Some("a long plain text that wraps"));
        assert_eq!(text_at(body, &["after"]), Some("1"));
        assert_eq!(items_at(body, &["items"]), [scalar("first long item"), scalar("second")]);
    }

    #[test]
    fn a_stripped_header_keeps_its_negative_file_id() {
        let object = only(
            "
            |--- !u!1 &-8123456789012345678 stripped
            |GameObject:
            |  m_CorrespondingSourceObject: {fileID: 100000, guid: 9f2c, type: 3}
            |  m_PrefabInstance: {fileID: 1001}
            ",
        );
        assert_eq!(object.class_id, 1);
        assert_eq!(object.file_id, -8123456789012345678);
        assert!(object.stripped);
        assert_eq!(text_at(&object.body, &["m_PrefabInstance", "fileID"]), Some("1001"));
    }

    #[test]
    fn scene_roots_carry_the_largest_class_and_file_ids() {
        let roots = only(
            "
            |--- !u!1660057539 &9223372036854775807
            |SceneRoots:
            |  m_ObjectHideFlags: 0
            |  m_Roots:
            |  - {fileID: 1234}
            |  - {fileID: 5678}
            ",
        );
        assert_eq!(roots.class_id, 1660057539);
        assert_eq!(roots.file_id, i64::MAX);
        assert_eq!(roots.type_name, "SceneRoots");
        assert_eq!(text_at(&items_at(&roots.body, &["m_Roots"])[1], &["fileID"]), Some("5678"));
    }

    #[test]
    fn documents_know_their_first_and_last_lines() {
        let source = unity(
            "
            |--- !u!1 &1
            |GameObject:
            |  m_Name: A
            |--- !u!4 &2
            |Transform:
            |  m_Children: []
            |  m_Father: {fileID: 0}
            |--- !u!1 &3 stripped
            |GameObject:
            ",
        );
        let parsed = documents(&source);
        let spans: Vec<_> = parsed.iter().map(|doc| (doc.first_line, doc.last_line)).collect();
        assert_eq!(spans, [(3, 5), (6, 9), (10, 11)]);
        assert_eq!(parsed.iter().map(|doc| doc.file_id).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(parsed.iter().map(|doc| doc.class_id).collect::<Vec<_>>(), [1, 4, 1]);
        assert_eq!(parsed[2].type_name, "GameObject");
        assert_eq!(parsed[2].body, Node::Map(Vec::new()));
    }

    #[test]
    fn windows_line_endings_read_like_unix_ones() {
        let source = unity(
            "
            |--- !u!1 &1
            |GameObject:
            |  m_Name: A
            |  m_Component:
            |  - component: {fileID: 4}
            ",
        )
        .replace('\n', "\r\n");
        let parsed = documents(&source);
        assert_eq!(parsed.len(), 1);
        assert_eq!(text_at(&parsed[0].body, &["m_Name"]), Some("A"));
        assert_eq!(items_at(&parsed[0].body, &["m_Component"]).len(), 1);
        assert_eq!(parsed[0].last_line, 7);
        assert!(is_unity_yaml(&source));
    }

    #[test]
    fn a_meta_file_reads_as_one_map() {
        let meta = plain(&text(
            "
            |fileFormatVersion: 2
            |guid: 9a3c51e07d2b4f6c8e1a0b2c3d4e5f60
            |TextureImporter:
            |  internalIDToNameTable: []
            |  externalObjects: {}
            |  serializedVersion: 12
            |  mipmaps:
            |    mipMapMode: 0
            |  spritePixelsToUnits: 16
            |  spriteSheet:
            |    sprites: []
            |  platformSettings:
            |  - serializedVersion: 3
            |    buildTarget: DefaultTexturePlatform
            |  userData:
            |  assetBundleName:
            ",
        ));
        assert_eq!(number_at(&meta, &["fileFormatVersion"]), Some(2.0));
        assert_eq!(text_at(&meta, &["guid"]), Some("9a3c51e07d2b4f6c8e1a0b2c3d4e5f60"));
        assert_eq!(meta.at(&["TextureImporter", "internalIDToNameTable"]), Some(&empty_list()));
        assert_eq!(meta.at(&["TextureImporter", "externalObjects"]), Some(&Node::Map(Vec::new())));
        assert_eq!(text_at(&meta, &["TextureImporter", "mipmaps", "mipMapMode"]), Some("0"));
        assert_eq!(meta.at(&["TextureImporter", "spriteSheet", "sprites"]), Some(&empty_list()));
        let platforms = items_at(&meta, &["TextureImporter", "platformSettings"]);
        assert_eq!(text_at(&platforms[0], &["buildTarget"]), Some("DefaultTexturePlatform"));
        assert_eq!(meta.at(&["TextureImporter", "userData"]), Some(&scalar("")));
        assert_eq!(meta.at(&["TextureImporter", "assetBundleName"]), Some(&scalar("")));
        assert_eq!(meta.entries().len(), 3);
    }

    #[test]
    fn empty_values_and_empty_collections_stay_distinct() {
        let root = plain(concat!(
            "name: \n",
            "list: []\n",
            "map: {}\n",
            "quoted: ''\n",
            "double: \"\"\n",
            "blank:\n",
            "next: [ ]\n",
            "flow: {a: , b: [], c: {}}\n",
        ));
        assert_eq!(root.get("name"), Some(&scalar("")));
        assert_eq!(root.get("list"), Some(&empty_list()));
        assert_eq!(root.get("map"), Some(&Node::Map(Vec::new())));
        assert_eq!(root.get("quoted"), Some(&scalar("")));
        assert_eq!(root.get("double"), Some(&scalar("")));
        assert_eq!(root.get("blank"), Some(&scalar("")));
        assert_eq!(root.get("next"), Some(&empty_list()));
        let flow = vec![entry("a", scalar("")), entry("b", empty_list()), entry("c", Node::Map(Vec::new()))];
        assert_eq!(root.get("flow"), Some(&Node::Map(flow)));
        assert_eq!(root.get("missing"), None);
        assert_eq!(root.at(&[]), Some(&root));
        assert!(scalar("x").entries().is_empty());
        assert!(scalar("x").items().is_empty());
        assert_eq!(root.text(), None);
    }

    #[test]
    fn plain_scalars_keep_their_text_exactly() {
        let source = text(
            "
            |hex: 0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9
            |path: Assets/Art/slime.png
            |url: https:example.com/a?b=c:d
            |text: Hello: world
            |odd: it's
            ",
        );
        let root = plain(&(source + "trailing: spaced   \n"));
        let hex = "0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9";
        assert_eq!(text_at(&root, &["hex"]), Some(hex));
        assert_eq!(number_at(&root, &["hex"]), None);
        assert_eq!(text_at(&root, &["path"]), Some("Assets/Art/slime.png"));
        assert_eq!(text_at(&root, &["url"]), Some("https:example.com/a?b=c:d"));
        assert_eq!(text_at(&root, &["text"]), Some("Hello: world"));
        assert_eq!(text_at(&root, &["odd"]), Some("it's"));
        assert_eq!(text_at(&root, &["trailing"]), Some("spaced"));
    }

    #[test]
    fn numbers_read_unity_float_spellings() {
        assert_eq!(scalar("1e-05").number(), Some(0.00001));
        assert_eq!(scalar("-0").number().map(f64::is_sign_negative), Some(true));
        assert_eq!(scalar("42").number(), Some(42.0));
        assert_eq!(scalar("-17").number(), Some(-17.0));
        assert_eq!(scalar("0.99999994").number(), Some(0.99999994));
        assert_eq!(scalar("Infinity").number(), Some(f64::INFINITY));
        assert_eq!(scalar("-Infinity").number(), Some(f64::NEG_INFINITY));
        assert!(scalar("NaN").number().is_some_and(f64::is_nan));
        assert_eq!(scalar("").number(), None);
        assert_eq!(scalar("0a1b").number(), None);
        assert_eq!(empty_list().number(), None);
    }

    #[test]
    fn unity_yaml_is_told_apart_from_other_text() {
        assert!(is_unity_yaml("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n--- !u!1 &1\nGameObject:\n"));
        assert!(is_unity_yaml("\u{feff}%YAML 1.1\r\n%TAG !u! tag:unity3d.com,2011:\r\n"));
        assert!(!is_unity_yaml("%YAML 1.1\n--- !u!1 &1\n%TAG !u! tag:unity3d.com,2011:\n"));
        assert!(!is_unity_yaml("fileFormatVersion: 2\nguid: 9a3c\n"));
        assert!(!is_unity_yaml("%TAG !u! tag:unity3d.com,2011:\n"));
        assert!(!is_unity_yaml(""));
    }

    #[test]
    fn text_without_headers_has_no_documents() {
        assert!(documents("fileFormatVersion: 2\n").is_empty());
        assert!(documents("").is_empty());
        let lone = documents("---");
        assert_eq!(lone.len(), 1);
        assert_eq!((lone[0].class_id, lone[0].file_id, lone[0].first_line, lone[0].last_line), (0, 0, 1, 1));
        assert_eq!(lone[0].type_name, "");
        assert_eq!(lone[0].body, Node::Map(Vec::new()));
    }

    #[test]
    fn garbage_gives_a_best_effort_result_without_panicking() {
        let samples = [
            "--- !u!x &y\n: : :\n- - -\n'unclosed\n  \"also\n{[}]\n",
            "--- !u!99999999999 &99999999999999999999 stripped extra\nA:\n      deep: 1\n  shallow: 2\n- x\n",
            "a: {b: [c, {d: 'e''\n  f: \"\\u12\\UFFFFFFFF\\x\\\n",
            "key: \"\\uD800 lone \\uDC00 low \\\n\n  \\",
            "- \n-\n  - \n    -\n- 'x'\n  - y\n  z: 1\n",
            "---\n---\n--- !u!1\n%YAML\n\t\ttab: 1\n日本: 語\n  é: [日, 本]\n",
            "a:\n  b:\n c: 1\n   d: 2\n    - e\n",
            ": \n:\n'': ''\n\"\": x\n{}: 1\n[]: 2\n",
        ];
        for sample in samples {
            assert!(!format!("{:?}{:?}", documents(sample), plain(sample)).is_empty());
        }
        let pieces = [
            "-", " ", "  ", "    ", ": ", ":", "'", "''", "\"", "\\", "\\u", "{", "}", "[", "]", ",", "\n",
            "\r\n", "a", "é", "日", "--- !u!1 &", "%YAML", "0", "#", "&", "|",
        ];
        let mut state = 0x9e3779b97f4a7c15u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..3000 {
            let length = next() % 120;
            let picks = (0..length).map(|_| next() % pieces.len() as u64);
            let sample: String = picks.map(|pick| pieces[pick as usize]).collect();
            let shown = format!("{:?}{:?}{}", documents(&sample), plain(&sample), is_unity_yaml(&sample));
            assert!(!shown.is_empty());
        }
    }

    #[test]
    fn deep_nesting_is_capped_instead_of_overflowing_the_stack() {
        let levels = 20_000;
        let maps = format!("a: {}{}\n", "{k: ".repeat(levels), "}".repeat(levels));
        let lists = format!("b: {}x{}\n", "[".repeat(levels), "]".repeat(levels));
        let dashes = format!("{}x\n", "- ".repeat(levels));
        let stairs: String = (0..3_000).map(|level| format!("{}k:\n", " ".repeat(level))).collect();
        let items: String = (0..3_000).map(|level| format!("{}-\n", " ".repeat(level))).collect();
        for source in [maps, lists, dashes, stairs, items] {
            let root = plain(&source);
            assert!(depth(&root) <= DEEPEST_BLOCK + DEEPEST_FLOW + 4, "depth {}", depth(&root));
            assert_eq!(root.clone(), root);
            assert!(!format!("{root:?}").is_empty());
        }
    }

    #[test]
    fn fifty_thousand_documents_parse_in_well_under_a_second() {
        let mut source = String::from("%YAML 1.1\n%TAG !u! tag:unity3d.com,2011:\n");
        for id in 0..50_000 {
            source.push_str(&format!("--- !u!1 &{id}\nGameObject:\n  m_Component:\n"));
            source.push_str(&format!("  - component: {{fileID: {}}}\n  m_Name: Object {id}\n", id + 1));
            source.push_str("  m_LocalPosition: {x: 0, y: 1.5, z: -2}\n  m_Children: []\n");
        }
        let started = Instant::now();
        let parsed = documents(&source);
        let spent = started.elapsed();
        assert_eq!(parsed.len(), 50_000);
        let last = &parsed[49_999];
        assert_eq!(text_at(&last.body, &["m_Name"]), Some("Object 49999"));
        let component = &items_at(&last.body, &["m_Component"])[0];
        assert_eq!(text_at(component, &["component", "fileID"]), Some("50000"));
        assert_eq!(last.last_line, 2 + 50_000 * 7);
        assert!(spent < Duration::from_secs(3), "took {spent:?}");
    }
}
