use std::sync::Arc;

use serde_json::Value;

use super::{
    Started, Stop,
    bundled::PINS,
    grant::Grant,
    manifest::{MANIFEST_FILE, Manifest},
};
use crate::{
    net::services::{ConnectorId, HostServices},
    plugin::{self, Kind, Pin},
};

pub(crate) const CONNECTORS_FOLDER: &str = "connectors";

pub(crate) struct Connectors;

pub(crate) type Catalog = plugin::Catalog<Connectors>;
pub(crate) type Connector = plugin::Plugin<Connectors>;
pub(crate) type Standing = plugin::Standing<Grant>;
pub(crate) type ConnectorFolders = plugin::Folders;
pub(crate) use plugin::Origin;

impl Kind for Connectors {
    type Id = ConnectorId;
    type Manifest = Manifest;
    type Grant = Grant;
    type Started = Started;
    type Host = dyn HostServices;

    const MANIFEST_FILE: &'static str = MANIFEST_FILE;
    const APPROVALS_FILE: &'static str = "connectors.json";
    const APPROVALS_LIST: &'static str = "connectors";
    const NOUN: &'static str = "connector";

    fn pins() -> &'static [Pin] {
        &PINS
    }

    fn id_named(name: String) -> ConnectorId {
        ConnectorId(name)
    }

    fn name_of(id: &ConnectorId) -> &str {
        &id.0
    }

    fn parse(bytes: &[u8]) -> Result<Manifest, String> {
        Manifest::parse(bytes)
    }

    fn id_of(manifest: &Manifest) -> &ConnectorId {
        &manifest.id
    }

    fn version_of(manifest: &Manifest) -> &str {
        &manifest.version
    }

    fn module_of(manifest: &Manifest) -> &str {
        &manifest.module
    }

    fn asked(manifest: &Manifest) -> Grant {
        Grant::asked_by(manifest)
    }

    fn beyond(asked: &Grant, granted: &Grant) -> Option<Grant> {
        asked.beyond(granted)
    }

    fn grant_from_json(value: &Value) -> Grant {
        Grant::from_json(value)
    }

    fn grant_to_json(grant: &Grant) -> Value {
        grant.to_json()
    }

    fn start(id: &ConnectorId, wasm: &[u8], host: &Arc<dyn HostServices>) -> Result<Started, Stop> {
        Started::start(id.clone(), wasm, Arc::clone(host)).map(|(started, _)| started)
    }
}

impl Catalog {
    pub(crate) fn connectors(&self) -> &[Connector] {
        self.plugins()
    }
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

    pub(crate) fn started_bundled() -> Catalog {
        let folders = ConnectorFolders {
            source: Some(sources()),
            ..ConnectorFolders::default()
        };
        let mut catalog = Catalog::discover(&folders, Approvals::default());
        catalog.start(&host());
        catalog
    }

    #[test]
    fn the_bundled_connectors_are_found_in_the_sources_match_their_pins_and_start() {
        let catalog = started_bundled();
        let ids: Vec<&str> = catalog.connectors().iter().map(|connector| connector.id.0.as_str()).collect();
        assert_eq!(ids, ["github", "azure-devops"]);
        for connector in catalog.connectors() {
            assert_eq!((connector.origin, &connector.standing), (Origin::Bundled, &Standing::Ready), "{}", connector.id.0);
            assert_eq!(connector.folder, sources().join(&connector.id.0).join("dist"));
            assert!(connector.started.is_some(), "{}", connector.id.0);
            assert!(!connector.holds_module_bytes(), "the module bytes are dropped once compiled");
        }
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
        let github = |catalog: &Catalog| -> Vec<Origin> {
            catalog
                .connectors()
                .iter()
                .filter(|connector| connector.id == id("github"))
                .map(|connector| connector.origin)
                .collect()
        };
        assert_eq!(github(&catalog), [Origin::Installed]);
        let again = Catalog::discover(&folders, Approvals::at(approvals_file.clone()));
        assert_eq!(github(&again), [Origin::Installed]);
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
        let mut catalog = started_bundled();
        assert_eq!(catalog.approve(&id("github")).unwrap(), None);
        assert_eq!(catalog.approve(&id("unknown")).unwrap(), None);
    }
}
