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
        let Some(mut found) = self.continuation(threshold, is_open_quote(first)) else {
            return Cow::Borrowed(first);
        };
        let mut fold = Fold { text: first.to_string(), blanks: 0, escapes: first.starts_with('"') };
        loop {
            let (at, line) = found;
            fold.blanks += at - self.next;
            fold.push(line);
            self.next = at + 1;
            match self.continuation(threshold, is_open_quote(&fold.text)) {
                Some(more) => found = more,
                None => return Cow::Owned(fold.text),
            }
        }
    }

    fn continuation(&self, threshold: usize, inside_quotes: bool) -> Option<(usize, &'a str)> {
        let mut at = self.next;
        loop {
            let (indent, content) = split_indent(self.lines.get(at).copied()?);
            if !content.is_empty() {
                return (inside_quotes || indent > threshold).then_some((at, content));
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

fn is_open_quote(text: &str) -> bool {
    matches!(text.as_bytes().first(), Some(b'\'' | b'"')) && quoted_end(text).is_none()
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
