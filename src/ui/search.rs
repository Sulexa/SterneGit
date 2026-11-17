use std::ops::Range;

use fancy_regex::Regex;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchOptions {
    pub(crate) match_case: bool,
    pub(crate) whole_words: bool,
    pub(crate) pattern: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Search {
    pub(crate) text: String,
    pub(crate) options: SearchOptions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BadPattern;

#[derive(Clone, Debug)]
pub(crate) struct Matcher(Regex);

impl Search {
    pub(crate) fn matcher(&self) -> Result<Option<Matcher>, BadPattern> {
        if self.text.is_empty() {
            return Ok(None);
        }
        let options = self.options;
        let text = if options.pattern {
            self.text.clone()
        } else {
            fancy_regex::escape(&self.text).into_owned()
        };
        let bounded = if options.whole_words {
            format!(r"\b(?:{text})\b")
        } else {
            text
        };
        let cased = if options.match_case {
            bounded
        } else {
            format!("(?i){bounded}")
        };
        Regex::new(&cased).map(|regex| Some(Matcher(regex))).map_err(|_| BadPattern)
    }
}

impl Matcher {
    pub(crate) fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        self.0
            .find_iter(text)
            .map_while(Result::ok)
            .map(|found| found.range())
            .filter(|range| !range.is_empty())
            .collect()
    }

    pub(crate) fn finds(&self, text: &str) -> bool {
        self.0.is_match(text).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search(text: &str, options: SearchOptions) -> Search {
        Search {
            text: text.into(),
            options,
        }
    }

    fn ranges(text: &str, options: SearchOptions, line: &str) -> Vec<Range<usize>> {
        search(text, options)
            .matcher()
            .unwrap()
            .unwrap()
            .ranges(line)
    }

    #[test]
    fn plain_text_matches_any_case_by_default() {
        let line = "let args = diff_args(Args);";
        assert_eq!(
            ranges("args", SearchOptions::default(), line),
            [4..8, 16..20, 21..25]
        );
    }

    #[test]
    fn match_case_keeps_only_the_same_case() {
        let options = SearchOptions {
            match_case: true,
            ..Default::default()
        };
        assert_eq!(ranges("Args", options, "let args = diff_args(Args);"), [21..25]);
    }

    #[test]
    fn whole_words_skip_parts_of_longer_names() {
        let options = SearchOptions {
            whole_words: true,
            ..Default::default()
        };
        assert_eq!(ranges("args", options, "let args = diff_args(args);"), [4..8, 21..25]);
    }

    #[test]
    fn plain_text_takes_pattern_characters_literally() {
        assert_eq!(ranges("f(", SearchOptions::default(), "f(x) + g(f(y))"), [0..2, 9..11]);
    }

    #[test]
    fn a_pattern_is_read_as_a_regular_expression() {
        let options = SearchOptions {
            pattern: true,
            ..Default::default()
        };
        assert_eq!(ranges(r"fileID: \d+", options, "{fileID: 42, guid: 9}"), [1..11]);
    }

    #[test]
    fn a_pattern_that_does_not_parse_is_bad() {
        let options = SearchOptions {
            pattern: true,
            ..Default::default()
        };
        assert_eq!(search("fn (", options).matcher().unwrap_err(), BadPattern);
    }

    #[test]
    fn empty_matches_are_left_out() {
        let options = SearchOptions {
            pattern: true,
            ..Default::default()
        };
        assert_eq!(ranges("x*", options, "axxb"), [1..3]);
    }

    #[test]
    fn nothing_typed_searches_nothing() {
        assert!(search("", SearchOptions::default()).matcher().unwrap().is_none());
    }

    #[test]
    fn finds_says_whether_anything_matches() {
        let matcher = search("diff", SearchOptions::default()).matcher().unwrap().unwrap();
        assert!(matcher.finds("Split the Diff parser"));
        assert!(!matcher.finds("Add a menu top bar"));
    }
}
