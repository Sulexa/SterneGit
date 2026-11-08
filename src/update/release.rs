use serde_json::Value;
use url::Url;

use super::Version;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) notes: Url,
    pub(crate) setup: Download,
    pub(crate) portable: Download,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Download {
    pub(crate) url: Url,
    pub(crate) sha256: String,
    pub(crate) size: u64,
}

const SHA256_HEX_DIGITS: usize = 64;

impl Release {
    pub(crate) fn read(json: &[u8]) -> Option<Self> {
        let fields: Value = serde_json::from_slice(json).ok()?;
        Some(Self {
            version: Version::parse(fields.get("version")?.as_str()?)?,
            notes: web_address(fields.get("notes")?)?,
            setup: Download::read(fields.get("setup")?)?,
            portable: Download::read(fields.get("portable")?)?,
        })
    }
}

impl Download {
    fn read(fields: &Value) -> Option<Self> {
        let sha256 = fields.get("sha256")?.as_str()?.to_ascii_lowercase();
        let hex = sha256.len() == SHA256_HEX_DIGITS && sha256.bytes().all(|byte| byte.is_ascii_hexdigit());
        Some(Self {
            url: web_address(fields.get("url")?)?,
            sha256: hex.then_some(sha256)?,
            size: fields.get("size")?.as_u64()?,
        })
    }

    pub(crate) fn file_name(&self) -> Option<&str> {
        let name = self.url.path_segments()?.next_back()?;
        let plain = !name.is_empty()
            && name != ".."
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'));
        plain.then_some(name)
    }
}

fn web_address(value: &Value) -> Option<Url> {
    Url::parse(value.as_str()?)
        .ok()
        .filter(|url| matches!(url.scheme(), "https" | "http"))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::Release;

    const HASH: &str = "4F0E7B1C9A2D3E5F60718293A4B5C6D7E8F90A1B2C3D4E5F60718293A4B5C6D7";

    fn published() -> Value {
        json!({
            "version": "0.2.0",
            "notes": "https://github.com/Sulexa/SterneGit/releases/tag/v0.2.0",
            "setup": {
                "url": "https://github.com/Sulexa/SterneGit/releases/download/v0.2.0/SterneGit-Setup-0.2.0.exe",
                "sha256": HASH,
                "size": 15_000_000
            },
            "portable": {
                "url": "https://github.com/Sulexa/SterneGit/releases/download/v0.2.0/SterneGit-0.2.0-portable.zip",
                "sha256": HASH,
                "size": 17_000_000
            }
        })
    }

    fn read(value: &Value) -> Option<Release> {
        Release::read(value.to_string().as_bytes())
    }

    #[test]
    fn the_file_ci_publishes_reads_as_a_release() {
        let release = read(&published()).unwrap();
        assert_eq!(release.version.to_string(), "0.2.0");
        assert_eq!(release.setup.sha256, HASH.to_ascii_lowercase());
        assert_eq!(release.setup.size, 15_000_000);
        assert_eq!(release.setup.file_name(), Some("SterneGit-Setup-0.2.0.exe"));
        assert_eq!(release.portable.file_name(), Some("SterneGit-0.2.0-portable.zip"));
        assert_eq!(release.notes.as_str(), "https://github.com/Sulexa/SterneGit/releases/tag/v0.2.0");
    }

    #[test]
    fn a_release_missing_anything_or_with_a_bad_hash_is_not_a_release() {
        let mut no_portable = published();
        no_portable.as_object_mut().unwrap().remove("portable");
        let mut short_hash = published();
        short_hash["setup"]["sha256"] = json!("abc");
        let mut not_hex = published();
        not_hex["setup"]["sha256"] = json!("z".repeat(64));
        let mut odd_version = published();
        odd_version["version"] = json!("v0.2.0");
        let mut file_url = published();
        file_url["setup"]["url"] = json!("file:///C:/Windows/notepad.exe");
        for broken in [no_portable, short_hash, not_hex, odd_version, file_url] {
            assert_eq!(read(&broken), None, "{broken}");
        }
        assert_eq!(Release::read(b"<html>Not Found</html>"), None);
    }

    #[test]
    fn a_download_whose_name_could_leave_its_folder_has_no_file_name() {
        let mut release = read(&published()).unwrap();
        release.setup.url = "https://example.com/a/%2E%2E".parse().unwrap();
        assert_eq!(release.setup.file_name(), None);
        release.setup.url = "https://example.com/a/".parse().unwrap();
        assert_eq!(release.setup.file_name(), None);
        release.setup.url = "https://example.com/a/b%5Cc.exe".parse().unwrap();
        assert_eq!(release.setup.file_name(), None);
    }
}
