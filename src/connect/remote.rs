use sternegit_protocol::RepoRef;
use url::Url;

const NAME: &str = "name";
const SEVERAL_SEGMENTS: &str = "...";
const SCHEMES: [&str; 6] = ["https", "http", "ssh", "git", "git+ssh", "ssh+git"];
const GIT_SUFFIX: &str = ".git";

#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Literal(String),
    OwnerSegment,
    OwnerSegments,
    Name,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathPattern {
    pieces: Vec<Piece>,
}

impl PathPattern {
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let pieces = text
            .trim_matches('/')
            .split('/')
            .map(|segment| piece(text, segment))
            .collect::<Result<Vec<_>, _>>()?;
        let count = |wanted: fn(&Piece) -> bool| pieces.iter().filter(|piece| wanted(piece)).count();
        let names = count(|piece| *piece == Piece::Name);
        let several = count(|piece| *piece == Piece::OwnerSegments);
        let owners = count(|piece| matches!(piece, Piece::OwnerSegment | Piece::OwnerSegments));
        if names != 1 || owners == 0 || several > 1 {
            return Err(format!(
                "The remote path {text} needs one {{name}}, at least one owner part and at most one part ending in ..."
            ));
        }
        Ok(Self { pieces })
    }

    pub(crate) fn locate(&self, path: &str) -> Option<RepoRef> {
        let segments: Vec<&str> = path.split('/').collect();
        if segments.iter().any(|segment| segment.is_empty()) {
            return None;
        }
        let several = self.pieces.contains(&Piece::OwnerSegments);
        let fixed = self.pieces.len() - usize::from(several);
        let spare = segments.len().checked_sub(fixed)?;
        if (several && spare == 0) || (!several && spare != 0) {
            return None;
        }
        let mut owner = Vec::new();
        let mut name = None;
        let mut rest = segments.as_slice();
        for piece in &self.pieces {
            let taken = if *piece == Piece::OwnerSegments { spare } else { 1 };
            let (here, after) = rest.split_at(taken);
            rest = after;
            match piece {
                Piece::Literal(literal) if here[0] != literal => return None,
                Piece::Literal(_) => {}
                Piece::OwnerSegment | Piece::OwnerSegments => owner.extend_from_slice(here),
                Piece::Name => name = Some(here[0]),
            }
        }
        Some(RepoRef {
            owner: owner.join("/"),
            name: name?.to_owned(),
        })
    }
}

fn piece(pattern: &str, segment: &str) -> Result<Piece, String> {
    let refused = || format!("The remote path {pattern} has a part SterneGit can't read: {segment:?}");
    match segment.strip_prefix('{').and_then(|inner| inner.strip_suffix('}')) {
        Some(NAME) => Ok(Piece::Name),
        Some(capture) => match capture.strip_suffix(SEVERAL_SEGMENTS) {
            Some(several) if is_plain(several) => Ok(Piece::OwnerSegments),
            None if is_plain(capture) => Ok(Piece::OwnerSegment),
            _ => Err(refused()),
        },
        None if segment.is_empty() || segment.contains(['{', '}']) => Err(refused()),
        None => Ok(Piece::Literal(segment.to_owned())),
    }
}

fn is_plain(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Remote {
    pub(crate) host: String,
    pub(crate) path: String,
}

impl Remote {
    pub(crate) fn parse(url: &str) -> Option<Self> {
        let url = url.trim();
        let (host, path) = match url.contains("://") {
            true => {
                let parsed = Url::parse(url).ok()?;
                if !SCHEMES.contains(&parsed.scheme()) {
                    return None;
                }
                (parsed.host_str()?.to_owned(), parsed.path().to_owned())
            }
            false => {
                let (before, path) = url.split_once(':')?;
                let host = before.rsplit_once('@').map_or(before, |(_, host)| host);
                if before.contains(['/', '\\']) || host.len() < 2 {
                    return None;
                }
                (host.to_owned(), path.to_owned())
            }
        };
        let host = host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase();
        let path = path.trim_matches('/');
        let path = path.strip_suffix(GIT_SUFFIX).unwrap_or(path).trim_end_matches('/');
        (!host.is_empty() && !path.is_empty()).then(|| Self {
            host,
            path: path.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use sternegit_protocol::RepoRef;

    use super::{PathPattern, Remote};

    fn remote(host: &str, path: &str) -> Option<Remote> {
        Some(Remote {
            host: host.into(),
            path: path.into(),
        })
    }

    fn repo(owner: &str, name: &str) -> Option<RepoRef> {
        Some(RepoRef {
            owner: owner.into(),
            name: name.into(),
        })
    }

    #[test]
    fn every_way_of_writing_a_remote_gives_its_host_and_path() {
        for url in [
            "https://github.com/octo/app.git",
            "https://user:secret@GitHub.com/octo/app",
            "https://github.com/octo/app/",
            "git@github.com:octo/app.git",
            "github.com:octo/app",
            "ssh://git@github.com/octo/app.git",
            "ssh://git@github.com:22/octo/app.git",
            "git://github.com/octo/app.git",
        ] {
            assert_eq!(Remote::parse(url), remote("github.com", "octo/app"), "{url}");
        }
        assert_eq!(
            Remote::parse("ssh://git@ssh.github.com:443/octo/app.git"),
            remote("ssh.github.com", "octo/app")
        );
    }

    #[test]
    fn local_paths_and_other_schemes_are_not_remotes() {
        for url in [
            "/home/me/app.git",
            "../app",
            r"C:\work\app",
            "C:/work/app",
            "file:///home/me/app.git",
            "https://github.com/",
            "",
        ] {
            assert_eq!(Remote::parse(url), None, "{url}");
        }
    }

    #[test]
    fn owner_and_name_come_from_the_pattern() {
        let github = PathPattern::parse("{owner}/{name}").unwrap();
        assert_eq!(github.locate("octo/app"), repo("octo", "app"));
        assert_eq!(github.locate("octo/app/tree"), None);
        assert_eq!(github.locate("octo"), None);
        assert_eq!(github.locate("octo//app"), None);
    }

    #[test]
    fn an_owner_ending_in_dots_takes_every_segment_before_the_name() {
        let gitlab = PathPattern::parse("{owner...}/{name}").unwrap();
        assert_eq!(gitlab.locate("group/sub/deeper/app"), repo("group/sub/deeper", "app"));
        assert_eq!(gitlab.locate("group/app"), repo("group", "app"));
        assert_eq!(gitlab.locate("app"), None);
    }

    #[test]
    fn literal_parts_must_match_and_several_captures_join_into_the_owner() {
        let azure = PathPattern::parse("{org}/{project}/_git/{name}").unwrap();
        assert_eq!(azure.locate("contoso/web/_git/app"), repo("contoso/web", "app"));
        assert_eq!(azure.locate("contoso/web/git/app"), None);
        let ssh = PathPattern::parse("v3/{org}/{project}/{name}").unwrap();
        assert_eq!(ssh.locate("v3/contoso/web/app"), repo("contoso/web", "app"));
    }

    #[test]
    fn a_pattern_needs_one_name_and_an_owner() {
        for pattern in ["{owner}", "{name}", "{owner}/{name}/{name}", "{a...}/{b...}/{name}", "{Owner}/{name}", "x{owner}/{name}", "{owner}//{name}"] {
            assert!(PathPattern::parse(pattern).is_err(), "{pattern}");
        }
    }
}
