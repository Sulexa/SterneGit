use std::{ffi::OsString, path::PathBuf};

#[cfg(any(windows, target_os = "macos"))]
const PRODUCT: &str = "FastGit";
#[cfg(all(unix, not(target_os = "macos")))]
const PRODUCT: &str = "fastgit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Folder {
    Config,
    State,
    Data,
    Local,
    #[cfg(not(windows))]
    Session,
}

impl Folder {
    pub(crate) fn path(self) -> Option<PathBuf> {
        self.path_from(&|name| std::env::var_os(name))
    }

    #[cfg(windows)]
    fn path_from(self, var: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        match self {
            Folder::Config | Folder::State | Folder::Data => {
                Some(absolute(var("APPDATA"))?.join(PRODUCT))
            }
            Folder::Local => Some(absolute(var("LOCALAPPDATA"))?.join(PRODUCT)),
        }
    }

    #[cfg(target_os = "macos")]
    fn path_from(self, var: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        let base = match self {
            Folder::Session => absolute(var("TMPDIR"))?,
            Folder::Config | Folder::State | Folder::Data | Folder::Local => {
                absolute(var("HOME"))?.join("Library/Application Support")
            }
        };
        Some(base.join(PRODUCT))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn path_from(self, var: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
        let in_home = |relative: &str| Some(absolute(var("HOME"))?.join(relative));
        let base = match self {
            Folder::Config => absolute(var("XDG_CONFIG_HOME")).or_else(|| in_home(".config"))?,
            Folder::State => absolute(var("XDG_STATE_HOME")).or_else(|| in_home(".local/state"))?,
            Folder::Data | Folder::Local => {
                absolute(var("XDG_DATA_HOME")).or_else(|| in_home(".local/share"))?
            }
            Folder::Session => absolute(var("XDG_RUNTIME_DIR"))?,
        };
        Some(base.join(PRODUCT))
    }
}

fn absolute(value: Option<OsString>) -> Option<PathBuf> {
    value.map(PathBuf::from).filter(|path| path.is_absolute())
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::Folder;

    fn with(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[cfg(windows)]
    #[test]
    fn settings_saved_tabs_and_installed_connectors_live_in_roaming_app_data() {
        let vars = with(&[("APPDATA", r"C:\Users\me\AppData\Roaming")]);
        let roaming = PathBuf::from(r"C:\Users\me\AppData\Roaming\FastGit");
        assert_eq!(Folder::Config.path_from(&vars), Some(roaming.clone()));
        assert_eq!(Folder::State.path_from(&vars), Some(roaming.clone()));
        assert_eq!(Folder::Data.path_from(&vars), Some(roaming));
    }

    #[cfg(windows)]
    #[test]
    fn ssh_keys_stay_on_this_machine_in_local_app_data() {
        let vars = with(&[("LOCALAPPDATA", r"C:\Users\me\AppData\Local")]);
        assert_eq!(
            Folder::Local.path_from(&vars),
            Some(PathBuf::from(r"C:\Users\me\AppData\Local\FastGit"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_relative_app_data_is_ignored() {
        assert_eq!(Folder::Config.path_from(&with(&[("APPDATA", "Roaming")])), None);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_folders_follow_the_xdg_variables_and_fall_back_to_home() {
        let vars = with(&[("HOME", "/home/me"), ("XDG_RUNTIME_DIR", "/run/user/1000")]);
        assert_eq!(
            Folder::Config.path_from(&vars),
            Some(PathBuf::from("/home/me/.config/fastgit"))
        );
        assert_eq!(
            Folder::State.path_from(&vars),
            Some(PathBuf::from("/home/me/.local/state/fastgit"))
        );
        assert_eq!(
            Folder::Data.path_from(&vars),
            Some(PathBuf::from("/home/me/.local/share/fastgit"))
        );
        assert_eq!(
            Folder::Session.path_from(&vars),
            Some(PathBuf::from("/run/user/1000/fastgit"))
        );
    }
}
