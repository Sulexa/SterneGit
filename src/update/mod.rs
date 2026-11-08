mod archive;
mod fetching;
mod installation;
mod launch;
mod preference;
mod release;
mod running;
mod swap;
mod version;

use std::{
    fmt, io,
    path::{Path, PathBuf},
};

pub(crate) use installation::Installation;
pub(crate) use launch::Then;
pub(crate) use preference::{update_automatically, updates_automatically};
pub(crate) use release::{Download, Release};
pub(crate) use running::another_copy_runs;
pub(crate) use version::Version;

use crate::{
    net::{curl::Curl, request::NetError, services::Cancel},
    settings::Folder,
};

const UPDATES_FOLDER: &str = "updates";
const UNPACKED: &str = "portable";
const WRITE_CHECK: &str = ".sternegit-write-check";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum UpdateError {
    BuiltHere,
    Network(NetError),
    UnreadableFeed,
    Corrupt,
    ReadOnlyFolder,
    Disk(String),
}

impl From<io::Error> for UpdateError {
    fn from(error: io::Error) -> Self {
        Self::Disk(error.to_string())
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltHere => f.write_str("Only copies from a SterneGit release update themselves"),
            Self::Network(error) => error.fmt(f),
            Self::UnreadableFeed => f.write_str("The list of releases couldn't be read"),
            Self::Corrupt => f.write_str("The download didn't match the release, so it was thrown away"),
            Self::ReadOnlyFolder => f.write_str("SterneGit's folder can't be written to, so it can't update itself"),
            Self::Disk(message) => f.write_str(message),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Staged {
    Setup(PathBuf),
    Portable(PathBuf),
}

pub(crate) fn updates_folder() -> Option<PathBuf> {
    Folder::Local.path().map(|folder| folder.join(UPDATES_FOLDER))
}

pub(crate) async fn newer_release(
    curl: &Curl,
    installation: &Installation,
    folder: &Path,
    cancel: Cancel,
) -> Result<Option<Release>, UpdateError> {
    let feed = installation.feed().ok_or(UpdateError::BuiltHere)?;
    let release = fetching::read_feed(curl, feed, folder, cancel).await?;
    Ok(release.version.replaces(&Version::running()).then_some(release))
}

pub(crate) async fn stage(
    curl: &Curl,
    release: &Release,
    installation: &Installation,
    folder: &Path,
    cancel: Cancel,
) -> Result<Staged, UpdateError> {
    let folder = folder.join(release.version.to_string());
    match installation {
        Installation::BuiltHere => Err(UpdateError::BuiltHere),
        Installation::Setup { .. } => {
            let setup = folder.join(release.setup.file_name().ok_or(UpdateError::UnreadableFeed)?);
            fetching::download(curl, &release.setup, &setup, cancel).await?;
            Ok(Staged::Setup(setup))
        }
        Installation::Portable { exe, .. } => {
            if !writable(exe.parent().ok_or(UpdateError::ReadOnlyFolder)?) {
                return Err(UpdateError::ReadOnlyFolder);
            }
            let unpacked = folder.join(UNPACKED);
            if unpacked.is_dir() {
                return Ok(Staged::Portable(unpacked));
            }
            let zip = folder.join(release.portable.file_name().ok_or(UpdateError::UnreadableFeed)?);
            fetching::download(curl, &release.portable, &zip, cancel).await?;
            unpack_once(&zip, &unpacked)?;
            let _ = std::fs::remove_file(&zip);
            Ok(Staged::Portable(unpacked))
        }
    }
}

pub(crate) fn apply(staged: &Staged, installation: &Installation, then: Then) -> io::Result<()> {
    match staged {
        Staged::Setup(setup) => launch::run_setup(setup, then),
        Staged::Portable(unpacked) => {
            let exe = installation
                .exe()
                .ok_or_else(|| io::Error::other(UpdateError::BuiltHere.to_string()))?;
            swap::swap_in(unpacked, exe)?;
            if then == Then::Relaunch {
                launch::start(exe, &[])?;
            }
            Ok(())
        }
    }
}

pub(crate) fn tidy(installation: &Installation, folder: &Path) {
    if let Installation::Portable { exe, .. } = installation {
        swap::tidy_after_swap(exe);
    }
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let running = Version::running();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let passed = Version::parse(&name.to_string_lossy()).is_some_and(|version| !version.replaces(&running));
        if passed {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn unpack_once(zip: &Path, unpacked: &Path) -> Result<(), UpdateError> {
    let part = fetching::unfinished(unpacked);
    let _ = std::fs::remove_dir_all(&part);
    if let Err(error) = archive::unpack(zip, &part) {
        let _ = std::fs::remove_dir_all(&part);
        let _ = std::fs::remove_file(zip);
        return Err(error.into());
    }
    if let Err(error) = std::fs::rename(&part, unpacked) {
        let _ = std::fs::remove_dir_all(&part);
        if !unpacked.is_dir() {
            return Err(error.into());
        }
    }
    Ok(())
}

fn writable(folder: &Path) -> bool {
    let probe = folder.join(format!("{WRITE_CHECK}-{}", std::process::id()));
    let created = std::fs::OpenOptions::new().write(true).create_new(true).open(&probe);
    let _ = std::fs::remove_file(&probe);
    created.is_ok()
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use serde_json::json;
    use url::Url;

    use super::{
        Download, Installation, Release, Staged, Then, UNPACKED, Version, apply, archive::tests::zip, newer_release,
        stage, tidy,
    };
    use crate::{
        net::{curl::Curl, services::cancel_pair},
        test_server::{Reply, TestServer},
    };

    const DEFLATED: u16 = 8;

    fn sha256(content: &[u8]) -> String {
        use sha2::{Digest as _, Sha256};
        Sha256::digest(content).iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn put(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn publish(version: &str, setup: &[u8], portable: &[u8]) -> (TestServer, Release) {
        let feed = Arc::new(Mutex::new(String::new()));
        let served_feed = Arc::clone(&feed);
        let (served_setup, served_portable) = (setup.to_vec(), portable.to_vec());
        let server = TestServer::start(move |seen| match seen.target.as_str() {
            "/latest.json" => Reply::status(200).body(served_feed.lock().unwrap().clone()),
            target if target.ends_with(".exe") => Reply::status(200).body(served_setup.clone()),
            target if target.ends_with(".zip") => Reply::status(200).body(served_portable.clone()),
            _ => Reply::status(404),
        });
        let offered = |name: String, content: &[u8]| Download {
            url: Url::parse(&server.url(&format!("/download/v{version}/{name}"))).unwrap(),
            sha256: sha256(content),
            size: content.len() as u64,
        };
        let release = Release {
            version: Version::parse(version).unwrap(),
            notes: Url::parse(&format!("https://github.com/Sulexa/SterneGit/releases/tag/v{version}")).unwrap(),
            setup: offered(format!("SterneGit-Setup-{version}.exe"), setup),
            portable: offered(format!("SterneGit-{version}-portable.zip"), portable),
        };
        let download = |download: &Download| {
            json!({ "url": download.url.as_str(), "sha256": download.sha256, "size": download.size })
        };
        *feed.lock().unwrap() = json!({
            "version": version,
            "notes": release.notes.as_str(),
            "setup": download(&release.setup),
            "portable": download(&release.portable),
        })
        .to_string();
        (server, release)
    }

    fn feed_of(server: &TestServer) -> Url {
        Url::parse(&server.url("/latest.json")).unwrap()
    }

    #[test]
    fn a_portable_copy_downloads_unpacks_and_swaps_in_the_newer_release() {
        let root = scratch("portable");
        let exe = root.join("portable").join("sternegit.exe");
        put(&exe, "MZ running");
        put(&root.join("portable/connectors/github/github.wasm"), "old wasm");
        let packed = zip(&[("sternegit.exe", b"MZ newer"), ("connectors/github/github.wasm", b"new wasm")], DEFLATED);
        let (server, _) = publish("9.0.0", b"MZ setup", &packed);
        let installation = Installation::Portable { exe: exe.clone(), feed: feed_of(&server) };
        let updates = root.join("updates");
        let curl = Curl::ignoring_git_config();
        let (_keep, cancel) = cancel_pair();
        let found = smol::block_on(newer_release(&curl, &installation, &updates, cancel)).unwrap().unwrap();
        let (_keep, cancel) = cancel_pair();
        let staged = smol::block_on(stage(&curl, &found, &installation, &updates, cancel)).unwrap();
        let unpacked = updates.join("9.0.0").join(UNPACKED);
        let zip_kept = updates.join("9.0.0").join("SterneGit-9.0.0-portable.zip").exists();
        apply(&staged, &installation, Then::Stay).unwrap();
        let exe_now = std::fs::read_to_string(&exe).unwrap();
        let wasm_now = std::fs::read_to_string(root.join("portable/connectors/github/github.wasm")).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(staged, Staged::Portable(unpacked));
        assert!(!zip_kept);
        assert_eq!(exe_now, "MZ newer");
        assert_eq!(wasm_now, "new wasm");
    }

    #[test]
    fn an_installed_copy_downloads_the_setup_beside_its_version() {
        let root = scratch("setup");
        let (server, _) = publish("9.0.0", b"MZ setup", b"zip");
        let installation = Installation::Setup { exe: root.join("sternegit.exe"), feed: feed_of(&server) };
        let updates = root.join("updates");
        let curl = Curl::ignoring_git_config();
        let (_keep, cancel) = cancel_pair();
        let found = smol::block_on(newer_release(&curl, &installation, &updates, cancel)).unwrap().unwrap();
        let (_keep, cancel) = cancel_pair();
        let staged = smol::block_on(stage(&curl, &found, &installation, &updates, cancel)).unwrap();
        let setup = updates.join("9.0.0").join("SterneGit-Setup-9.0.0.exe");
        let downloaded = std::fs::read(&setup).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(staged, Staged::Setup(setup));
        assert_eq!(downloaded, b"MZ setup");
    }

    #[test]
    fn a_release_that_is_not_newer_is_not_offered() {
        let root = scratch("not-newer");
        let (server, _) = publish(&Version::running().to_string(), b"MZ", b"zip");
        let installation = Installation::Setup { exe: root.join("sternegit.exe"), feed: feed_of(&server) };
        let (_keep, cancel) = cancel_pair();
        let found = smol::block_on(newer_release(&Curl::ignoring_git_config(), &installation, &root, cancel)).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(found, None);
    }

    #[test]
    fn tidying_drops_releases_already_reached_and_keeps_newer_ones() {
        let root = scratch("tidy");
        let running = Version::running().to_string();
        for name in [running.as_str(), "0.0.1", "999.0.0", "not-a-version"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        tidy(&Installation::BuiltHere, &root);
        let mut left: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(left, ["999.0.0", "not-a-version"]);
    }
}
