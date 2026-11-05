use std::{
    fs,
    path::{Path, PathBuf},
};

const BUILD_OUTPUT: &str = "target";

#[test]
fn the_sources_carry_no_comments() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for file in rust_files(&root.join("src"))
        .into_iter()
        .chain(rust_files(&root.join("tests")))
        .chain(rust_files(&root.join("connectors")))
    {
        let source = fs::read_to_string(&file).expect("sources are UTF-8");
        let shown = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .display()
            .to_string();
        found.extend(
            comment_lines(&source)
                .into_iter()
                .map(|line| format!("{shown}:{line}")),
        );
    }
    assert!(found.is_empty(), "comments found at:\n{}", found.join("\n"));
}

#[test]
fn comment_markers_inside_literals_are_not_comments() {
    let source = r##"let url = "https://example.com/*"; let slash = '/'; let quote = '"';
let raw = r#"// "quoted" /* */"#; let bytes = b"//"; let escaped = "\" // still a string";
fn borrow<'a>(text: &'a str) -> &'a str { text }"##;
    assert!(comment_lines(source).is_empty());
}

#[test]
fn line_doc_and_block_comments_are_found_on_their_lines() {
    let source =
        "let a = 1; // note\nlet b = 2;\n/* one\n two */\n/// doc\n//! inner\nlet c = a / b;";
    assert_eq!(comment_lines(source), [1, 3, 5, 6]);
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
        if path.is_dir() && path.file_name().is_some_and(|name| name == BUILD_OUTPUT) {
            continue;
        } else if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

fn comment_lines(source: &str) -> Vec<usize> {
    let chars: Vec<char> = source.chars().collect();
    let mut lexer = Lexer {
        chars: &chars,
        at: 0,
        line: 1,
    };
    let mut lines = Vec::new();
    while let Some(&current) = chars.get(lexer.at) {
        match current {
            '\n' => {
                lexer.line += 1;
                lexer.at += 1;
            }
            '/' if matches!(lexer.peek(1), Some('/' | '*')) => {
                lines.push(lexer.line);
                lexer.skip_comment();
            }
            '"' => {
                lexer.at += 1;
                lexer.skip_string();
            }
            '\'' => lexer.skip_char_or_lifetime(),
            letter if letter.is_alphabetic() || letter == '_' => lexer.skip_word(),
            _ => lexer.at += 1,
        }
    }
    lines.dedup();
    lines
}

struct Lexer<'a> {
    chars: &'a [char],
    at: usize,
    line: usize,
}

impl Lexer<'_> {
    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn advance(&mut self) {
        if self.peek(0) == Some('\n') {
            self.line += 1;
        }
        self.at += 1;
    }

    fn skip_comment(&mut self) {
        if self.peek(1) == Some('/') {
            while self.peek(0).is_some_and(|current| current != '\n') {
                self.at += 1;
            }
            return;
        }
        self.at += 2;
        let mut depth = 1;
        while depth > 0 && self.peek(0).is_some() {
            match (self.peek(0), self.peek(1)) {
                (Some('/'), Some('*')) => {
                    depth += 1;
                    self.at += 2;
                }
                (Some('*'), Some('/')) => {
                    depth -= 1;
                    self.at += 2;
                }
                _ => self.advance(),
            }
        }
    }

    fn skip_string(&mut self) {
        while let Some(current) = self.peek(0) {
            match current {
                '\\' => {
                    self.at += 1;
                    self.advance();
                }
                '"' => {
                    self.at += 1;
                    return;
                }
                _ => self.advance(),
            }
        }
    }

    fn skip_raw_string(&mut self) {
        let hashes = (0..)
            .take_while(|&ahead| self.peek(ahead) == Some('#'))
            .count();
        self.at += hashes + 1;
        while self.peek(0).is_some() {
            let closes = self.peek(0) == Some('"')
                && (1..=hashes).all(|ahead| self.peek(ahead) == Some('#'));
            if closes {
                self.at += hashes + 1;
                return;
            }
            self.advance();
        }
    }

    fn skip_char_or_lifetime(&mut self) {
        match (self.peek(1), self.peek(2)) {
            (Some('\\'), _) => {
                self.at += 3;
                while self.peek(0).is_some_and(|current| current != '\'') {
                    self.at += 1;
                }
                self.at += 1;
            }
            (Some(_), Some('\'')) => self.at += 3,
            _ => self.at += 1,
        }
    }

    fn skip_word(&mut self) {
        let start = self.at;
        while self
            .peek(0)
            .is_some_and(|current| current.is_alphanumeric() || current == '_')
        {
            self.at += 1;
        }
        let word: String = self.chars[start..self.at].iter().collect();
        let opens_raw_string = matches!(word.as_str(), "r" | "br" | "cr")
            && matches!(self.peek(0), Some('"' | '#'))
            && (0..)
                .map(|ahead| self.peek(ahead))
                .find(|current| *current != Some('#'))
                == Some(Some('"'));
        if opens_raw_string {
            self.skip_raw_string();
        }
    }
}
