use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{Approval, Approvals, Hashes, Kind, Pin, Stop, sha256};
use crate::settings::Folder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin {
    Bundled,
    Installed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Standing<G> {
    Ready,
    Updated,
    Waiting(G),
    Modified(G),
    Unusable(String),
    Failed(Stop),
}

impl<G> Standing<G> {
    pub(crate) fn may_start(&self) -> bool {
        matches!(self, Standing::Ready | Standing::Updated)
    }

    pub(crate) fn awaits_approval(&self) -> bool {
        matches!(self, Standing::Waiting(_) | Standing::Modified(_))
    }
}

pub(crate) struct Plugin<K: Kind> {
    pub(crate) id: K::Id,
    pub(crate) folder: PathBuf,
    pub(crate) origin: Origin,
    pub(crate) manifest: Option<Arc<K::Manifest>>,
    pub(crate) standing: Standing<K::Grant>,
    pub(crate) replaces_bundled: bool,
    pub(crate) started: Option<Arc<K::Started>>,
    files: Hashes,
    module: Option<Vec<u8>>,
}

impl<K: Kind> Plugin<K> {
    #[cfg(test)]
    pub(crate) fn holds_module_bytes(&self) -> bool {
        self.module.is_some()
    }
}

struct Package<M> {
    manifest: M,
    files: Hashes,
    module: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Folders {
    pub(crate) bundled: Option<PathBuf>,
    pub(crate) source: Option<PathBuf>,
    pub(crate) installed: Option<PathBuf>,
}

impl Folders {
    pub(crate) fn of_this_sternegit(name: &str) -> Self {
        Self {
            bundled: std::env::current_exe()
                .ok()
                .and_then(|exe| bundled_beside(&exe, name))
                .filter(|folder| folder.is_dir()),
            source: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join(name)),
            installed: Folder::Data.path().map(|folder| folder.join(name)),
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

fn bundled_beside(exe: &Path, name: &str) -> Option<PathBuf> {
    let folder = exe.parent()?;
    if cfg!(target_os = "macos") && folder.ends_with("Contents/MacOS") {
        return Some(folder.parent()?.join("Resources").join(name));
    }
    Some(folder.join(name))
}

pub(crate) struct Catalog<K: Kind> {
    plugins: Vec<Plugin<K>>,
    approvals: Approvals<K>,
}

impl<K: Kind> Catalog<K> {
    pub(crate) fn discover(folders: &Folders, approvals: Approvals<K>) -> Self {
        let mut catalog = Self {
            plugins: Vec::new(),
            approvals,
        };
        for pin in K::pins() {
            if let Some(folder) = folders.pinned(pin) {
                catalog.add_pinned(pin, folder);
            }
        }
        if let Some(bundled) = &folders.bundled {
            for folder in subfolders(bundled) {
                if !K::pins().iter().any(|pin| folder.ends_with(pin.id)) {
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

    pub(crate) fn plugins(&self) -> &[Plugin<K>] {
        &self.plugins
    }

    pub(crate) fn approved_grant(&self, id: &K::Id) -> Option<K::Grant> {
        self.approvals.get(id).map(|approval| approval.grant.clone())
    }

    pub(crate) fn approved_version(&self, id: &K::Id) -> Option<String> {
        self.approvals.get(id).map(|approval| approval.version.clone())
    }

    pub(crate) fn start(&mut self, host: &Arc<K::Host>) {
        for plugin in &mut self.plugins {
            if plugin.started.is_some() || !plugin.standing.may_start() {
                continue;
            }
            let Some(module) = plugin.module.take() else {
                continue;
            };
            match K::start(&plugin.id, &module, host) {
                Ok(started) => plugin.started = Some(Arc::new(started)),
                Err(stop) => plugin.standing = Standing::Failed(stop),
            }
        }
    }

    pub(crate) fn approve(&mut self, id: &K::Id) -> io::Result<Option<K::Grant>> {
        let Some(plugin) = self
            .plugins
            .iter_mut()
            .find(|plugin| &plugin.id == id && plugin.standing.awaits_approval())
        else {
            return Ok(None);
        };
        let Some(manifest) = plugin.manifest.clone() else {
            return Ok(None);
        };
        let grant = K::asked(&manifest);
        let mut approval = Approval::new(
            id.clone(),
            K::version_of(&manifest).to_owned(),
            plugin.files.clone(),
            grant.clone(),
        );
        approval.replaces_bundled = plugin.replaces_bundled;
        let replaces_bundled = plugin.replaces_bundled;
        self.approvals.record(approval)?;
        plugin.standing = Standing::Ready;
        if replaces_bundled {
            self.plugins
                .retain(|plugin| !(&plugin.id == id && plugin.origin == Origin::Bundled));
        }
        Ok(Some(grant))
    }

    pub(crate) fn seen(&mut self, id: &K::Id) -> io::Result<()> {
        let Some(plugin) = self
            .plugins
            .iter_mut()
            .find(|plugin| &plugin.id == id && plugin.standing == Standing::Updated)
        else {
            return Ok(());
        };
        plugin.standing = Standing::Ready;
        if let Some(mut approval) = self.approvals.get(id).cloned() {
            approval.updated = false;
            self.approvals.record(approval)?;
        }
        Ok(())
    }

    fn add_pinned(&mut self, pin: &Pin, folder: PathBuf) {
        let id = K::id_named(pin.id.to_owned());
        let plugin = match read_package::<K>(&folder, &id) {
            Err(reason) => unusable(id, folder, Origin::Bundled, reason),
            Ok(package) if package.files == pin.files() => built(id, folder, Origin::Bundled, package, Standing::Ready),
            Ok(package) => {
                let standing = self.judge_modified(&id, &package);
                built(id, folder, Origin::Bundled, package, standing)
            }
        };
        self.plugins.push(plugin);
    }

    fn add_installed(&mut self, folder: PathBuf) {
        let id = K::id_named(folder.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned()));
        let installed_twice = self
            .plugins
            .iter()
            .any(|known| known.id == id && known.origin == Origin::Installed);
        let plugin = match read_package::<K>(&folder, &id) {
            Err(reason) => unusable(id, folder, Origin::Installed, reason),
            Ok(_) if installed_twice => {
                let reason = format!("Another installed {} already has the id {}", K::NOUN, K::name_of(&id));
                unusable(id, folder, Origin::Installed, reason)
            }
            Ok(package) => {
                let standing = self.judge_installed(&id, &package);
                built(id, folder, Origin::Installed, package, standing)
            }
        };
        self.plugins.push(plugin);
    }

    fn judge_modified(&self, id: &K::Id, package: &Package<K::Manifest>) -> Standing<K::Grant> {
        let asked = K::asked(&package.manifest);
        match self.approvals.get(id) {
            Some(approval) if approval.files == package.files => match K::beyond(&asked, &approval.grant) {
                Some(more) => Standing::Modified(more),
                None => Standing::Ready,
            },
            _ => Standing::Modified(asked),
        }
    }

    fn judge_installed(&mut self, id: &K::Id, package: &Package<K::Manifest>) -> Standing<K::Grant> {
        let asked = K::asked(&package.manifest);
        let Some(approval) = self.approvals.get(id) else {
            return Standing::Waiting(asked);
        };
        if let Some(more) = K::beyond(&asked, &approval.grant) {
            return Standing::Waiting(more);
        }
        let version = K::version_of(&package.manifest);
        let unchanged = approval.files == package.files && approval.version == version;
        if unchanged {
            return match approval.updated {
                true => Standing::Updated,
                false => Standing::Ready,
            };
        }
        let mut updated = approval.clone();
        updated.version = version.to_owned();
        updated.files = package.files.clone();
        updated.updated = true;
        let _ = self.approvals.record(updated);
        Standing::Updated
    }

    fn settle_replacements(&mut self) {
        let bundled: Vec<K::Id> = self
            .plugins
            .iter()
            .filter(|plugin| plugin.origin == Origin::Bundled)
            .map(|plugin| plugin.id.clone())
            .collect();
        let mut replaced = Vec::new();
        for plugin in &mut self.plugins {
            if plugin.origin != Origin::Installed || !bundled.contains(&plugin.id) {
                continue;
            }
            plugin.replaces_bundled = true;
            let approved_to_replace = self
                .approvals
                .get(&plugin.id)
                .is_some_and(|approval| approval.replaces_bundled);
            match (approved_to_replace, plugin.standing.may_start(), &plugin.manifest) {
                (true, true, _) => replaced.push(plugin.id.clone()),
                (false, true, Some(manifest)) => {
                    plugin.standing = Standing::Waiting(K::asked(manifest));
                }
                _ => {}
            }
        }
        self.plugins
            .retain(|plugin| !(plugin.origin == Origin::Bundled && replaced.contains(&plugin.id)));
    }
}

fn read_package<K: Kind>(folder: &Path, id: &K::Id) -> Result<Package<K::Manifest>, String> {
    let manifest_path = folder.join(K::MANIFEST_FILE);
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|error| format!("Can't read {}: {error}", manifest_path.display()))?;
    let manifest = K::parse(&manifest_bytes)?;
    if K::id_of(&manifest) != id {
        return Err(format!(
            "The folder {} holds the {noun} {}; a {noun}'s folder must be named after its id",
            folder.display(),
            K::name_of(K::id_of(&manifest)),
            noun = K::NOUN,
        ));
    }
    let module_name = K::module_of(&manifest).to_owned();
    let module_path = folder.join(&module_name);
    let module = fs::read(&module_path).map_err(|error| format!("Can't read {}: {error}", module_path.display()))?;
    let files = Hashes::from([
        (K::MANIFEST_FILE.to_owned(), sha256(&manifest_bytes)),
        (module_name, sha256(&module)),
    ]);
    Ok(Package {
        manifest,
        files,
        module,
    })
}

fn built<K: Kind>(
    id: K::Id,
    folder: PathBuf,
    origin: Origin,
    package: Package<K::Manifest>,
    standing: Standing<K::Grant>,
) -> Plugin<K> {
    Plugin {
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

fn unusable<K: Kind>(id: K::Id, folder: PathBuf, origin: Origin, reason: String) -> Plugin<K> {
    Plugin {
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
