use sternegit_reader_protocol::{Found, Search};

use crate::host;

pub fn read_file(path: &str) -> Option<Vec<u8>> {
    host::read_file(path)
}

pub fn read_text(path: &str) -> Option<String> {
    read_file(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

pub fn search(regex: &str, glob: &str) -> Vec<Found> {
    let query = Search {
        regex: regex.to_owned(),
        glob: glob.to_owned(),
    };
    let query = serde_json::to_vec(&query).unwrap_or_default();
    serde_json::from_slice(&host::search(&query)).unwrap_or_default()
}

#[cfg_attr(target_arch = "wasm32", expect(dead_code))]
pub(crate) fn glob_matches(glob: &str, path: &str) -> bool {
    let glob = glob.strip_prefix("**/").unwrap_or(glob);
    if glob.contains('/') {
        return wildcard(glob, path);
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    wildcard(glob, name)
}

fn wildcard(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, tail)) => {
            let Some(rest) = text.strip_prefix(head) else {
                return false;
            };
            (0..=rest.len())
                .filter(|at| rest.is_char_boundary(*at))
                .any(|at| wildcard(tail, &rest[at..]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::glob_matches;

    #[test]
    fn a_glob_without_a_slash_matches_file_names_anywhere() {
        assert!(glob_matches("*.meta", "Assets/Art/Hero.png.meta"));
        assert!(glob_matches("**/*.meta", "Hero.png.meta"));
        assert!(!glob_matches("*.meta", "Assets/Hero.png"));
    }

    #[test]
    fn a_glob_with_a_slash_matches_the_whole_path() {
        assert!(glob_matches("Assets/*.prefab", "Assets/Player.prefab"));
        assert!(!glob_matches("Assets/*.prefab", "Other/Player.prefab"));
    }
}
