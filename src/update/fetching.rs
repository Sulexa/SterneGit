use std::{
    fs::File,
    io::{self, Read as _},
    path::{Path, PathBuf},
};

use sha2::{Digest as _, Sha256};
use url::Url;

use super::{Download, Release, UpdateError};
use crate::net::{
    curl::{Curl, Fetch},
    services::Cancel,
};

const USER_AGENT: (&str, &str) = ("User-Agent", concat!("SterneGit/", env!("CARGO_PKG_VERSION")));
const LARGEST_FEED_BYTES: u64 = 64 * 1024;
const FEED_FILE: &str = "latest.json";
const HASHED_AT_ONCE: usize = 64 * 1024;

pub(crate) async fn read_feed(curl: &Curl, feed: &Url, folder: &Path, cancel: Cancel) -> Result<Release, UpdateError> {
    std::fs::create_dir_all(folder)?;
    let into = unfinished(&folder.join(FEED_FILE));
    let fetched = curl.fetch(fetch(feed.clone(), into.clone(), LARGEST_FEED_BYTES), cancel).await;
    let read = fetched.map_err(UpdateError::Network).and_then(|()| Ok(std::fs::read(&into)?));
    let _ = std::fs::remove_file(&into);
    Release::read(&read?).ok_or(UpdateError::UnreadableFeed)
}

pub(crate) async fn download(curl: &Curl, download: &Download, into: &Path, cancel: Cancel) -> Result<(), UpdateError> {
    if matches(into, download)? {
        return Ok(());
    }
    if let Some(folder) = into.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let part = unfinished(into);
    let fetched = curl
        .fetch(fetch(download.url.clone(), part.clone(), download.size), cancel)
        .await
        .map_err(UpdateError::Network);
    let verified = fetched.and_then(|()| match matches(&part, download)? {
        true => Ok(()),
        false => Err(UpdateError::Corrupt),
    });
    if let Err(error) = verified {
        let _ = std::fs::remove_file(&part);
        return Err(error);
    }
    if let Err(error) = std::fs::rename(&part, into) {
        let _ = std::fs::remove_file(&part);
        if !matches(into, download)? {
            return Err(error.into());
        }
    }
    Ok(())
}

fn fetch(url: Url, into: PathBuf, largest_bytes: u64) -> Fetch {
    Fetch {
        url,
        into,
        largest_bytes,
        headers: vec![(USER_AGENT.0.to_owned(), USER_AGENT.1.to_owned())],
    }
}

pub(crate) fn unfinished(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.part", std::process::id()));
    path.with_file_name(name)
}

fn matches(path: &Path, download: &Download) -> io::Result<bool> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() != download.size {
        return Ok(false);
    }
    Ok(sha256_of(&mut file)? == download.sha256)
}

fn sha256_of(file: &mut File) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; HASHED_AT_ONCE];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;
    use url::Url;

    use super::{download, read_feed};
    use crate::{
        net::{curl::Curl, services::cancel_pair},
        test_server::{Reply, TestServer},
        update::{Download, UpdateError},
    };

    fn sha256(content: &[u8]) -> String {
        use sha2::{Digest as _, Sha256};
        Sha256::digest(content).iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-fetching-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn offered(server: &TestServer, path: &str, content: &[u8]) -> Download {
        Download {
            url: Url::parse(&server.url(path)).unwrap(),
            sha256: sha256(content),
            size: content.len() as u64,
        }
    }

    #[test]
    fn the_feed_is_followed_through_redirects_and_read_as_a_release() {
        let server = TestServer::start(|seen| match seen.target.as_str() {
            "/latest/download/latest.json" => Reply::status(302).header("Location", "/download/v0.2.0/latest.json"),
            _ => Reply::status(200).body(
                json!({
                    "version": "0.2.0",
                    "notes": "https://github.com/Sulexa/SterneGit/releases/tag/v0.2.0",
                    "setup": { "url": "https://example.com/Setup.exe", "sha256": "a".repeat(64), "size": 1 },
                    "portable": { "url": "https://example.com/p.zip", "sha256": "b".repeat(64), "size": 2 }
                })
                .to_string(),
            ),
        });
        let folder = scratch("feed");
        let (_keep, cancel) = cancel_pair();
        let feed = Url::parse(&server.url("/latest/download/latest.json")).unwrap();
        let release = smol::block_on(read_feed(&Curl::ignoring_git_config(), &feed, &folder, cancel)).unwrap();
        let left = std::fs::read_dir(&folder).unwrap().count();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(release.version.to_string(), "0.2.0");
        assert_eq!(left, 0);
        assert!(server.seen()[0].header("user-agent").unwrap().starts_with("SterneGit/"));
    }

    #[test]
    fn a_missing_feed_is_a_network_failure_and_a_page_is_unreadable() {
        let server = TestServer::start(|seen| match seen.target.as_str() {
            "/page" => Reply::status(200).body("<html></html>"),
            _ => Reply::status(404),
        });
        let folder = scratch("no-feed");
        let read = |path: &str| {
            let (_keep, cancel) = cancel_pair();
            let feed = Url::parse(&server.url(path)).unwrap();
            smol::block_on(read_feed(&Curl::ignoring_git_config(), &feed, &folder, cancel))
        };
        let missing = read("/missing");
        let page = read("/page");
        let _ = std::fs::remove_dir_all(&folder);
        assert!(matches!(missing, Err(UpdateError::Network(_))));
        assert_eq!(page, Err(UpdateError::UnreadableFeed));
    }

    #[test]
    fn a_download_matching_its_hash_lands_in_place() {
        let content = b"MZ the new setup".repeat(1000);
        let served = content.clone();
        let server = TestServer::start(move |_| Reply::status(200).body(served.clone()));
        let folder = scratch("download");
        let into = folder.join("0.2.0").join("SterneGit-Setup-0.2.0.exe");
        let (_keep, cancel) = cancel_pair();
        let offer = offered(&server, "/setup.exe", &content);
        smol::block_on(download(&Curl::ignoring_git_config(), &offer, &into, cancel)).unwrap();
        let landed = std::fs::read(&into).unwrap();
        let (_keep, cancel) = cancel_pair();
        smol::block_on(download(&Curl::ignoring_git_config(), &offer, &into, cancel)).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(landed, content);
        assert_eq!(server.seen().len(), 1);
    }

    #[test]
    fn a_download_that_differs_from_its_hash_is_thrown_away() {
        let server = TestServer::start(|_| Reply::status(200).body("MZ tampered"));
        let folder = scratch("tampered");
        let into = folder.join("setup.exe");
        let (_keep, cancel) = cancel_pair();
        let mut offer = offered(&server, "/setup.exe", b"MZ original");
        offer.size = "MZ tampered".len() as u64;
        let result = smol::block_on(download(&Curl::ignoring_git_config(), &offer, &into, cancel));
        let left = std::fs::read_dir(&folder).unwrap().count();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(result, Err(UpdateError::Corrupt));
        assert_eq!(left, 0);
    }

    #[test]
    fn a_download_larger_than_announced_is_stopped() {
        let server = TestServer::start(|_| Reply::status(200).body("x".repeat(5000)));
        let folder = scratch("too-large");
        let (_keep, cancel) = cancel_pair();
        let offer = offered(&server, "/setup.exe", b"small");
        let result = smol::block_on(download(&Curl::ignoring_git_config(), &offer, &folder.join("setup.exe"), cancel));
        let _ = std::fs::remove_dir_all(&folder);
        assert!(matches!(result, Err(UpdateError::Network(_))));
    }
}
