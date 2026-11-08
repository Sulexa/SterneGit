use std::path::{Path, PathBuf};

use url::Url;

const FEED: Option<&str> = option_env!("STERNEGIT_UPDATE_FEED");
const UNINSTALLER: &str = "unins000.exe";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Installation {
    BuiltHere,
    Setup { exe: PathBuf, feed: Url },
    Portable { exe: PathBuf, feed: Url },
}

impl Installation {
    pub(crate) fn of_this_sternegit() -> Self {
        let feed = FEED.and_then(|feed| Url::parse(feed).ok());
        if !cfg!(windows) {
            return Self::BuiltHere;
        }
        Self::found(feed, std::env::current_exe().ok())
    }

    fn found(feed: Option<Url>, exe: Option<PathBuf>) -> Self {
        let (Some(feed), Some(exe)) = (feed, exe) else {
            return Self::BuiltHere;
        };
        if beside(&exe, UNINSTALLER).is_some_and(|uninstaller| uninstaller.is_file()) {
            Self::Setup { exe, feed }
        } else {
            Self::Portable { exe, feed }
        }
    }

    pub(crate) fn feed(&self) -> Option<&Url> {
        match self {
            Self::BuiltHere => None,
            Self::Setup { feed, .. } | Self::Portable { feed, .. } => Some(feed),
        }
    }

    pub(crate) fn exe(&self) -> Option<&Path> {
        match self {
            Self::BuiltHere => None,
            Self::Setup { exe, .. } | Self::Portable { exe, .. } => Some(exe),
        }
    }
}

fn beside(exe: &Path, name: &str) -> Option<PathBuf> {
    Some(exe.parent()?.join(name))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use url::Url;

    use super::{Installation, UNINSTALLER};

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-installation-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn feed() -> Url {
        Url::parse("https://github.com/Sulexa/SterneGit/releases/latest/download/latest.json").unwrap()
    }

    #[test]
    fn an_exe_built_without_a_feed_never_updates_wherever_it_is() {
        let folder = scratch("built");
        std::fs::write(folder.join(UNINSTALLER), "").unwrap();
        let found = Installation::found(None, Some(folder.join("sternegit.exe")));
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(found, Installation::BuiltHere);
        assert_eq!(found.feed(), None);
    }

    #[test]
    fn an_exe_beside_the_uninstaller_was_installed_by_the_setup() {
        let folder = scratch("setup");
        std::fs::write(folder.join(UNINSTALLER), "").unwrap();
        let exe = folder.join("sternegit.exe");
        let found = Installation::found(Some(feed()), Some(exe.clone()));
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(found, Installation::Setup { exe, feed: feed() });
    }

    #[test]
    fn any_other_exe_with_a_feed_is_portable() {
        let folder = scratch("portable");
        let exe = folder.join("sternegit.exe");
        let found = Installation::found(Some(feed()), Some(exe.clone()));
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(found.exe(), Some(exe.as_path()));
        assert_eq!(found, Installation::Portable { exe, feed: feed() });
    }
}
