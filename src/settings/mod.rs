mod folders;
mod json_file;

pub(crate) use folders::Folder;
pub(crate) use json_file::{Fields, JsonFile};

const SETTINGS_FILE: &str = "settings.json";

pub(crate) fn settings_file() -> Option<JsonFile> {
    JsonFile::in_folder(Folder::Config, SETTINGS_FILE)
}
