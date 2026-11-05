#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MediaKind {
    Picture,
    Sound,
    Video,
}

const PICTURES: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tga", "qoi"];
const SOUNDS: &[&str] = &[
    "wav", "wave", "ogg", "oga", "mp3", "flac", "aif", "aiff", "m4a", "aac", "opus",
];
const VIDEOS: &[&str] = &["mp4", "m4v", "mov", "webm", "mkv", "avi", "wmv", "mpg", "mpeg"];

pub(crate) fn extension(path: &str) -> Option<String> {
    let name = path.rsplit('/').next()?;
    let (_, extension) = name.rsplit_once('.')?;
    (!extension.is_empty()).then(|| extension.to_ascii_lowercase())
}

pub(crate) fn media_kind(path: &str) -> Option<MediaKind> {
    let extension = extension(path)?;
    let listed = |names: &[&str]| names.contains(&extension.as_str());
    if listed(PICTURES) {
        Some(MediaKind::Picture)
    } else if listed(SOUNDS) {
        Some(MediaKind::Sound)
    } else if listed(VIDEOS) {
        Some(MediaKind::Video)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{MediaKind, extension, media_kind};

    #[test]
    fn the_extension_says_what_kind_of_file_it_is() {
        assert_eq!(media_kind("Assets/Art/slime.PNG"), Some(MediaKind::Picture));
        assert_eq!(media_kind("music/boss-theme.ogg"), Some(MediaKind::Sound));
        assert_eq!(media_kind("intro.mp4"), Some(MediaKind::Video));
        assert_eq!(media_kind("src/main.rs"), None);
    }

    #[test]
    fn a_dot_in_a_folder_name_is_not_an_extension() {
        assert_eq!(extension("assets.v2/README"), None);
        assert_eq!(extension("archive.tar.gz").as_deref(), Some("gz"));
        assert_eq!(extension("trailing."), None);
    }
}
