use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{
    Started, Stop,
    approvals::{Approval, Approvals, Hashes},
    bundled::{PINS, Pin, sha256},
    grant::Grant,
    manifest::{MANIFEST_FILE, Manifest},
};
use crate::{
    net::services::{ConnectorId, HostServices},
    settings::Folder,
};

const CONNECTORS_FOLDER: &str = "connectors";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    Bundled,
    Installed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Standing {
    Ready,
    Updated,
    Waiting(Grant),
    Modified(Grant),
    Unusable(String),
    Failed(Stop),
}

impl Standing {
    fn may_start(&self) -> bool {
        matches!(self, Standing::Ready | Standing::Updated)
    }

    pub(crate) fn awaits_approval(&self) -> bool {
        matches!(self, Standing::Waiting(_) | Standing::Modified(_))
    }
}

pub(crate) struct Connector {
    pub(crate) id: ConnectorId,
    pub(crate) folder: PathBuf,
    pub(crate) origin: Origin,
    pub(crate) manifest: Option<Arc<Manifest>>,
    pub(crate) standing: Standing,
    pub(crate) replaces_bundled: bool,
    pub(crate) started: Option<Arc<Started>>,
    files: Hashes,
    module: Option<Vec<u8>>,
}

struct Package {
    manifest: Manifest,
    files: Hashes,
    module: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ConnectorFolders {
    pub(crate) bundled: Option<PathBuf>,
    pub(crate) source: Option<PathBuf>,
    pub(crate) installed: Option<PathBuf>,
}

impl ConnectorFolders {
    pub(crate) fn of_this_sternegit() -> Self {
        Self {
            bundled: std::env::current_exe()
                .ok()
                .and_then(|exe| bundled_beside(&exe))
                .filter(|folder| folder.is_dir()),
            source: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join(CONNECTORS_FOLDER)),
            installed: Folder::Data.path().map(|folder| folder.join(CONNECTORS_FOLDER)),
        }
    }

    fn pinned(&self, pin: &Pin) -> Option<PathBuf> {
        match (&self.bundled, &self.source) {
            (Some(bundled), _) => Some(bundled.join(pin.id)),
            (None, Some(source)) => Some(pin.source_folder(source)),
            (None, None) => None,
        }
    }
}

fn bundled_beside(exe: &Path) -> Option<PathBuf> {
    let folder = exe.parent()?;
    if cfg!(target_os = "macos") && folder.ends_with("Contents/MacOS") {
        return Some(folder.parent()?.join("Resources").join(CONNECTORS_FOLDER));
    }
    Some(folder.join(CONNECTORS_FOLDER))
}

pub(crate) struct Catalog {
    connectors: Vec<Connector>,
    approvals: Approvals,
}

impl Catalog {
    pub(crate) fn discover(folders: &ConnectorFolders, approvals: Approvals) -> Self {
        let mut catalog = Self {
            connectors: Vec::new(),
            approvals,
        };
        for pin in &PINS {
            if let Some(folder) = folders.pinned(pin) {
                catalog.add_pinned(pin, folder);
            }
        }
        if let Some(bundled) = &folders.bundled {
            for folder in subfolders(bundled) {
                if !PINS.iter().any(|pin| folder.ends_with(pin.id)) {
                    catalog.add_installed(folder);
                }
            }
        }
        if let Some(installed) = &folders.installed {
            for folder in subfolders(installed) {
                catalog.add_installed(folder);
            }
        }
        catalog.settle_replacements();
        catalog
    }

    pub(crate) fn connectors(&self) -> &[Connector] {
        &self.connectors
    }

    pub(crate) fn approved_version(&self, id: &ConnectorId) -> Option<String> {
        self.approvals.get(id).map(|approval| approval.version.clone())
    }

    pub(crate) fn start(&mut self, host: &Arc<dyn HostServices>) {
        for connector in &mut self.connectors {
            if connector.started.is_some() || !connector.standing.may_start() {
                continue;
            }
            let Some(module) = connector.module.take() else {
                continue;
            };
            match Started::start(connector.id.clone(), &module, Arc::clone(host)) {
                Ok((started, _)) => connector.started = Some(Arc::new(started)),
                Err(stop) => connector.standing = Standing::Failed(stop),
            }
        }
    }

    pub(crate) fn approve(&mut self, id: &ConnectorId) -> io::Result<Option<Grant>> {
        let Some(connector) = self
            .connectors
            .iter_mut()
            .find(|connector| &connector.id == id && connector.standing.awaits_approval())
        else {
            return Ok(None);
        };
        let Some(manifest) = connector.manifest.clone() else {
            return Ok(None);
        };
        let grant = Grant::asked_by(&manifest);
        let mut approval = Approval::new(
            id.clone(),
            manifest.version.clone(),
            connector.files.clone(),
            grant.clone(),
        );
        approval.replaces_bundled = connector.replaces_bundled;
        let replaces_bundled = connector.replaces_bundled;
        self.approvals.record(approval)?;
        connector.standing = Standing::Ready;
        if replaces_bundled {
            self.connectors
                .retain(|connector| !(&connector.id == id && connector.origin == Origin::Bundled));
        }
        Ok(Some(grant))
    }

    pub(crate) fn seen(&mut self, id: &ConnectorId) -> io::Result<()> {
        let Some(connector) = self
            .connectors
            .iter_mut()
            .find(|connector| &connector.id == id && connector.standing == Standing::Updated)
        else {
            return Ok(());
        };
        connector.standing = Standing::Ready;
        if let Some(mut approval) = self.approvals.get(id).cloned() {
            approval.updated = false;
            self.approvals.record(approval)?;
        }
        Ok(())
    }

    fn add_pinned(&mut self, pin: &Pin, folder: PathBuf) {
        let id = ConnectorId(pin.id.to_owned());
        let connector = match read_package(&folder, &id) {
            Err(reason) => unusable(id, folder, Origin::Bundled, reason),
            Ok(package) if package.files == pin.files() => {
                built(id, folder, Origin::Bundled, package, Standing::Ready)
            }
            Ok(package) => {
                let standing = self.judge_modified(&id, &package);
                built(id, folder, Origin::Bundled, package, standing)
            }
        };
        self.connectors.push(connector);
    }

    fn add_installed(&mut self, folder: PathBuf) {
        let id = ConnectorId(folder.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned()));
        let installed_twice = self
            .connectors
            .iter()
            .any(|known| known.id == id && known.origin == Origin::Installed);
        let connector = match read_package(&folder, &id) {
            Err(reason) => unusable(id, folder, Origin::Installed, reason),
            Ok(_) if installed_twice => {
                let reason = format!("Another installed connector already has the id {}", id.0);
                unusable(id, folder, Origin::Installed, reason)
            }
            Ok(package) => {
                let standing = self.judge_installed(&id, &package);
                built(id, folder, Origin::Installed, package, standing)
            }
        };
        self.connectors.push(connector);
    }

    fn judge_modified(&self, id: &ConnectorId, package: &Package) -> Standing {
        let asked = Grant::asked_by(&package.manifest);
        match self.approvals.get(id) {
            Some(approval) if approval.files == package.files => match asked.beyond(&approval.grant) {
                Some(more) => Standing::Modified(more),
                None => Standing::Ready,
            },
            _ => Standing::Modified(asked),
        }
    }

    fn judge_installed(&mut self, id: &ConnectorId, package: &Package) -> Standing {
        let asked = Grant::asked_by(&package.manifest);
        let Some(approval) = self.approvals.get(id) else {
            return Standing::Waiting(asked);
        };
        if let Some(more) = asked.beyond(&approval.grant) {
            return Standing::Waiting(more);
        }
        let unchanged = approval.files == package.files && approval.version == package.manifest.version;
        if unchanged {
            return match approval.updated {
                true => Standing::Updated,
                false => Standing::Ready,
            };
        }
        let mut updated = approval.clone();
        updated.version = package.manifest.version.clone();
        updated.files = package.files.clone();
        updated.updated = true;
        let _ = self.approvals.record(updated);
        Standing::Updated
    }

    fn settle_replacements(&mut self) {
        let bundled: Vec<ConnectorId> = self
            .connectors
            .iter()
            .filter(|connector| connector.origin == Origin::Bundled)
            .map(|connector| connector.id.clone())
            .collect();
        let mut replaced = Vec::new();
        for connector in &mut self.connectors {
            if connector.origin != Origin::Installed || !bundled.contains(&connector.id) {
                continue;
            }
            connector.replaces_bundled = true;
            let approved_to_replace = self
                .approvals
                .get(&connector.id)
                .is_some_and(|approval| approval.replaces_bundled);
            match (approved_to_replace, connector.standing.may_start(), &connector.manifest) {
                (true, true, _) => replaced.push(connector.id.clone()),
                (false, true, Some(manifest)) => {
                    connector.standing = Standing::Waiting(Grant::asked_by(manifest));
                }
                _ => {}
            }
        }
        self.connectors.retain(|connector| {
            !(connector.origin == Origin::Bundled && replaced.contains(&connector.id))
        });
    }
}

fn read_package(folder: &Path, id: &ConnectorId) -> Result<Package, String> {
    let manifest_path = folder.join(MANIFEST_FILE);
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("Can't read {}: {error}", manifest_path.display()))?;
    let manifest = Manifest::parse(&manifest_bytes)?;
    if &manifest.id != id {
        return Err(format!(
            "The folder {} holds the connector {}; a connector's folder must be named after its id",
            folder.display(),
            manifest.id.0
        ));
    }
    let module_path = folder.join(&manifest.module);
    let module = fs::read(&module_path)
        .map_err(|error| format!("Can't read {}: {error}", module_path.display()))?;
    let files = Hashes::from([
        (MANIFEST_FILE.to_owned(), sha256(&manifest_bytes)),
        (manifest.module.clone(), sha256(&module)),
    ]);
    Ok(Package {
        manifest,
        files,
        module,
    })
}

fn built(id: ConnectorId, folder: PathBuf, origin: Origin, package: Package, standing: Standing) -> Connector {
    Connector {
        id,
        folder,
        origin,
        manifest: Some(Arc::new(package.manifest)),
        standing,
        replaces_bundled: false,
        started: None,
        files: package.files,
        module: Some(package.module),
    }
}

fn unusable(id: ConnectorId, folder: PathBuf, origin: Origin, reason: String) -> Connector {
    Connector {
        id,
        folder,
        origin,
        manifest: None,
        standing: Standing::Unusable(reason),
        replaces_bundled: false,
        started: None,
        files: Hashes::new(),
        module: None,
    }
}

fn subfolders(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use serde_json::{Value, json};

    use super::{Catalog, ConnectorFolders, Origin, Standing};
    use crate::{
        connect::{
            Stop,
            approvals::Approvals,
            fakes::{FakeHost, greeter},
            grant::Grant,
            manifest::{Manifest, samples},
        },
        net::services::{ConnectorId, HostServices},
    };

    pub(crate) fn sources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("connectors")
    }

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sternegit-catalog-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn host() -> Arc<dyn HostServices> {
        FakeHost::quiet()
    }

    fn id(text: &str) -> ConnectorId {
        ConnectorId(text.into())
    }

    fn copy_github_into(root: &Path) -> PathBuf {
        let folder = root.join("github");
        fs::create_dir_all(&folder).unwrap();
        for file in ["connector.json", "github.wasm"] {
            fs::copy(sources().join("github/dist").join(file), folder.join(file)).unwrap();
        }
        folder
    }

    fn install_fake(root: &Path, name: &str, manifest: &Value, wasm: &[u8]) {
        let folder = root.join(name);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("connector.json"), manifest.to_string()).unwrap();
        fs::write(folder.join("fake.wasm"), wasm).unwrap();
    }

    fn asked(manifest: &Value) -> Grant {
        Grant::asked_by(&Manifest::parse(manifest.to_string().as_bytes()).unwrap())
    }

    fn only_installed(root: &Path) -> ConnectorFolders {
        ConnectorFolders {
            installed: Some(root.join("installed")),
            ..ConnectorFolders::default()
        }
    }

    fn standing(catalog: &Catalog, wanted: &str, origin: Origin) -> Standing {
        catalog
            .connectors()
            .iter()
            .find(|connector| connector.id == id(wanted) && connector.origin == origin)
            .map(|connector| connector.standing.clone())
            .unwrap_or_else(|| panic!("no {origin:?} connector {wanted}"))
    }

    pub(crate) fn started_github() -> Catalog {
        let folders = ConnectorFolders {
            source: Some(sources()),
            ..ConnectorFolders::default()
        };
        let mut catalog = Catalog::discover(&folders, Approvals::default());
        catalog.start(&host());
        catalog
    }

    #[test]
    fn the_bundled_github_connector_is_found_in_the_sources_matches_its_pins_and_starts() {
        let catalog = started_github();
        let [github] = catalog.connectors() else {
            panic!("expected only GitHub");
        };
        assert_eq!(
            (github.id.0.as_str(), github.origin, &github.standing),
            ("github", Origin::Bundled, &Standing::Ready)
        );
        assert_eq!(github.folder, sources().join("github").join("dist"));
        assert!(github.started.is_some());
        assert!(github.module.is_none(), "the module bytes are dropped once compiled");
    }

    #[test]
    fn a_connectors_folder_beside_the_exe_wins_over_the_sources() {
        let root = scratch("beside");
        let folder = copy_github_into(&root.join("connectors"));
        let folders = ConnectorFolders {
            bundled: Some(root.join("connectors")),
            source: Some(sources()),
            installed: None,
        };
        let catalog = Catalog::discover(&folders, Approvals::default());
        let _ = fs::remove_dir_all(&root);
        assert_eq!(catalog.connectors()[0].folder, folder);
        assert_eq!(catalog.connectors()[0].standing, Standing::Ready);
    }

    #[test]
    fn a_bundled_connector_whose_files_differ_from_the_pins_waits_as_modified() {
        let root = scratch("modified");
        let folder = copy_github_into(&root.join("connectors"));
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(folder.join("connector.json")).unwrap()).unwrap();
        manifest["features"] = json!(["pulls"]);
        fs::write(folder.join("connector.json"), manifest.to_string()).unwrap();
        let folders = ConnectorFolders {
            bundled: Some(root.join("connectors")),
            ..ConnectorFolders::default()
        };
        let mut catalog = Catalog::discover(&folders, Approvals::default());
        catalog.start(&host());
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            standing(&catalog, "github", Origin::Bundled),
            Standing::Modified(asked(&manifest))
        );
        assert!(catalog.connectors()[0].started.is_none());
    }

    #[test]
    fn a_bundled_connector_missing_beside_the_exe_is_unusable() {
        let root = scratch("missing");
        fs::create_dir_all(root.join("connectors")).unwrap();
        let folders = ConnectorFolders {
            bundled: Some(root.join("connectors")),
            source: Some(sources()),
            installed: None,
        };
        let catalog = Catalog::discover(&folders, Approvals::default());
        let _ = fs::remove_dir_all(&root);
        assert!(matches!(
            standing(&catalog, "github", Origin::Bundled),
            Standing::Unusable(reason) if reason.starts_with("Can't read")
        ));
    }

    #[test]
    fn a_new_installed_connector_waits_for_approval_and_starts_once_approved() {
        let root = scratch("approve");
        let manifest = samples::minimal("fake");
        install_fake(&root.join("installed"), "fake", &manifest, &greeter());
        let approvals_file = Some(root.join("connectors.json"));
        let mut catalog =
            Catalog::discover(&only_installed(&root), Approvals::at(approvals_file.clone()));
        catalog.start(&host());
        assert_eq!(
            standing(&catalog, "fake", Origin::Installed),
            Standing::Waiting(asked(&manifest))
        );
        assert!(catalog.connectors()[0].started.is_none());
        assert_eq!(catalog.approve(&id("fake")).unwrap(), Some(asked(&manifest)));
        catalog.start(&host());
        assert_eq!(standing(&catalog, "fake", Origin::Installed), Standing::Ready);
        assert!(catalog.connectors()[0].started.is_some());
        let again = Catalog::discover(&only_installed(&root), Approvals::at(approvals_file));
        let _ = fs::remove_dir_all(&root);
        assert_eq!(standing(&again, "fake", Origin::Installed), Standing::Ready);
    }

    #[test]
    fn an_update_within_its_grant_starts_marked_updated_until_seen() {
        let root = scratch("update");
        let mut manifest = samples::minimal("fake");
        install_fake(&root.join("installed"), "fake", &manifest, &greeter());
        let approvals_file = Some(root.join("connectors.json"));
        let mut first =
            Catalog::discover(&only_installed(&root), Approvals::at(approvals_file.clone()));
        first.approve(&id("fake")).unwrap();
        manifest["version"] = json!("0.0.2");
        install_fake(&root.join("installed"), "fake", &manifest, &greeter());
        let mut updated =
            Catalog::discover(&only_installed(&root), Approvals::at(approvals_file.clone()));
        updated.start(&host());
        assert_eq!(standing(&updated, "fake", Origin::Installed), Standing::Updated);
        assert!(updated.connectors()[0].started.is_some());
        let recorded = Approvals::at(approvals_file.clone())
            .get(&id("fake"))
            .cloned()
            .unwrap();
        assert_eq!((recorded.version.as_str(), recorded.updated), ("0.0.2", true));
        updated.seen(&id("fake")).unwrap();
        assert_eq!(standing(&updated, "fake", Origin::Installed), Standing::Ready);
        let seen = Catalog::discover(&only_installed(&root), Approvals::at(approvals_file));
        let _ = fs::remove_dir_all(&root);
        assert_eq!(standing(&seen, "fake", Origin::Installed), Standing::Ready);
    }

    #[test]
    fn an_update_asking_for_more_waits_and_lists_only_what_is_new() {
        let root = scratch("more");
        let mut manifest = samples::minimal("fake");
        install_fake(&root.join("installed"), "fake", &manifest, &greeter());
        let approvals_file = Some(root.join("connectors.json"));
        Catalog::discover(&only_installed(&root), Approvals::at(approvals_file.clone()))
            .approve(&id("fake"))
            .unwrap();
        manifest["features"] = json!(["pulls"]);
        manifest["services"][0]["remotes"][0]["hosts"] = json!(["example.com", "ssh.example.com"]);
        install_fake(&root.join("installed"), "fake", &manifest, &greeter());
        let mut catalog = Catalog::discover(&only_installed(&root), Approvals::at(approvals_file));
        catalog.start(&host());
        let _ = fs::remove_dir_all(&root);
        let mut more = Grant::default();
        more.features.insert("pulls".into());
        more.remote_hosts.insert("ssh.example.com".into());
        assert_eq!(standing(&catalog, "fake", Origin::Installed), Standing::Waiting(more));
        assert!(catalog.connectors()[0].started.is_none());
    }

    #[test]
    fn a_folder_named_differently_from_its_connector_is_unusable() {
        let root = scratch("misnamed");
        install_fake(&root.join("installed"), "other", &samples::minimal("fake"), &greeter());
        let catalog = Catalog::discover(&only_installed(&root), Approvals::default());
        let _ = fs::remove_dir_all(&root);
        assert!(matches!(
            standing(&catalog, "other", Origin::Installed),
            Standing::Unusable(reason) if reason.contains("named after its id")
        ));
    }

    #[test]
    fn an_approved_module_that_cannot_start_fails_with_its_stop() {
        let root = scratch("fails");
        install_fake(&root.join("installed"), "fake", &samples::minimal("fake"), b"not webassembly");
        let mut catalog = Catalog::discover(
            &only_installed(&root),
            Approvals::at(Some(root.join("connectors.json"))),
        );
        catalog.approve(&id("fake")).unwrap();
        catalog.start(&host());
        let _ = fs::remove_dir_all(&root);
        assert!(matches!(
            standing(&catalog, "fake", Origin::Installed),
            Standing::Failed(Stop::Refused(_))
        ));
    }

    #[test]
    fn an_installed_connector_with_a_bundled_id_replaces_it_only_once_approved() {
        let root = scratch("replace");
        copy_github_into(&root.join("installed"));
        let folders = ConnectorFolders {
            source: Some(sources()),
            installed: Some(root.join("installed")),
            bundled: None,
        };
        let approvals_file = Some(root.join("connectors.json"));
        let mut catalog = Catalog::discover(&folders, Approvals::at(approvals_file.clone()));
        assert_eq!(standing(&catalog, "github", Origin::Bundled), Standing::Ready);
        assert!(matches!(standing(&catalog, "github", Origin::Installed), Standing::Waiting(_)));
        assert!(catalog.connectors().iter().any(|connector| connector.replaces_bundled));
        catalog.approve(&id("github")).unwrap();
        assert_eq!(catalog.connectors().len(), 1);
        assert_eq!(catalog.connectors()[0].origin, Origin::Installed);
        let again = Catalog::discover(&folders, Approvals::at(approvals_file.clone()));
        assert_eq!(again.connectors().len(), 1);
        assert_eq!(standing(&again, "github", Origin::Installed), Standing::Ready);
        let mut approvals = Approvals::at(approvals_file.clone());
        let mut without_replacing = approvals.get(&id("github")).cloned().unwrap();
        without_replacing.replaces_bundled = false;
        approvals.record(without_replacing).unwrap();
        let unconfirmed = Catalog::discover(&folders, Approvals::at(approvals_file));
        let _ = fs::remove_dir_all(&root);
        assert_eq!(standing(&unconfirmed, "github", Origin::Bundled), Standing::Ready);
        assert!(matches!(standing(&unconfirmed, "github", Origin::Installed), Standing::Waiting(_)));
    }

    #[test]
    fn approving_what_needs_no_approval_does_nothing() {
        let mut catalog = started_github();
        assert_eq!(catalog.approve(&id("github")).unwrap(), None);
        assert_eq!(catalog.approve(&id("unknown")).unwrap(), None);
    }
}
