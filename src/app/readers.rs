use std::{collections::BTreeMap, io, path::PathBuf, sync::Arc};

use gpui_kit::{App, AppContext as _, BorrowAppContext as _, Global};
use serde_json::Value;

use crate::{
    git::SideFiles,
    plugin::{Folders, Plugin},
    read::{FileKind, Grant, READERS_FOLDER, Reader, ReaderApprovals, ReaderCatalog, ReaderId, Readers as ReaderKind},
    settings::{Folder, JsonFile},
};

const CHOICES_FILE: &str = "reader-choices.json";
const EXTENSIONS: &str = "extensions";

pub(crate) type ReaderPlugin = Plugin<ReaderKind>;

#[derive(Default)]
pub(crate) struct Readers {
    catalog: Option<ReaderCatalog>,
    choices: BTreeMap<String, String>,
}

impl Global for Readers {}

#[derive(Clone)]
pub(crate) struct Chosen {
    pub(crate) id: ReaderId,
    pub(crate) name: String,
    pub(crate) merges: bool,
    pub(crate) reader: Arc<Reader>,
    pub(crate) kind: FileKind,
}

#[derive(Clone)]
pub(crate) enum Routed {
    Plain,
    By(Chosen),
    Ask { extension: String, candidates: Vec<Chosen> },
}

fn extension_of(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rfind('.').map_or_else(String::new, |at| name[at..].to_ascii_lowercase())
}

fn choices_file() -> Option<PathBuf> {
    Folder::Config.path().map(|folder| folder.join(CHOICES_FILE))
}

fn load_choices() -> BTreeMap<String, String> {
    choices_file()
        .map(|path| JsonFile::at(path).read())
        .and_then(|fields| fields.get(EXTENSIONS).and_then(Value::as_object).cloned())
        .map(|map| {
            map.into_iter()
                .filter_map(|(extension, id)| Some((extension, id.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

impl Readers {
    pub(crate) fn of(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.catalog.is_some()
    }

    pub(crate) fn readers(&self) -> &[ReaderPlugin] {
        self.catalog.as_ref().map_or(&[], ReaderCatalog::plugins)
    }

    pub(crate) fn reader(&self, id: &ReaderId) -> Option<&ReaderPlugin> {
        self.readers().iter().find(|reader| &reader.id == id)
    }

    pub(crate) fn approved_grant(&self, id: &ReaderId) -> Option<Grant> {
        self.catalog.as_ref()?.approved_grant(id)
    }

    pub(crate) fn approved_version(&self, id: &ReaderId) -> Option<String> {
        self.catalog.as_ref()?.approved_version(id)
    }

    pub(crate) fn waiting(&self) -> usize {
        self.readers().iter().filter(|reader| reader.standing.awaits_approval()).count()
    }

    pub(crate) fn shared_extensions(&self) -> Vec<(String, Vec<(ReaderId, String)>)> {
        let mut claims: BTreeMap<String, Vec<(ReaderId, String)>> = BTreeMap::new();
        for reader in self.readers().iter().filter(|reader| reader.started.is_some()) {
            let Some(manifest) = &reader.manifest else {
                continue;
            };
            for kind in &manifest.kinds {
                for extension in &kind.extensions {
                    let entry = claims.entry(extension.to_ascii_lowercase()).or_default();
                    if !entry.iter().any(|(id, _)| id == &reader.id) {
                        entry.push((reader.id.clone(), manifest.name.clone()));
                    }
                }
            }
        }
        claims.into_iter().filter(|(_, readers)| readers.len() > 1).collect()
    }

    pub(crate) fn side_files(&self) -> SideFiles {
        SideFiles::new(
            self.readers()
                .iter()
                .filter(|reader| reader.started.is_some())
                .filter_map(|reader| reader.manifest.as_ref())
                .flat_map(|manifest| manifest.side_files.iter().map(|side| side.pattern.clone())),
        )
    }

    pub(crate) fn side_files_in(cx: &App) -> SideFiles {
        cx.try_global::<Self>().map(Self::side_files).unwrap_or_default()
    }

    pub(crate) fn chosen_for(&self, extension: &str) -> Option<&str> {
        self.choices.get(extension).map(String::as_str)
    }

    pub(crate) fn route(&self, path: &str) -> Routed {
        let candidates: Vec<Chosen> = self
            .readers()
            .iter()
            .filter_map(|plugin| {
                let reader = plugin.started.clone()?;
                let manifest = plugin.manifest.clone()?;
                let kind = manifest.kind_for(path, None)?.clone();
                Some(Chosen {
                    id: plugin.id.clone(),
                    name: manifest.name.clone(),
                    merges: manifest.grant.jobs.contains("merge"),
                    reader,
                    kind,
                })
            })
            .collect();
        let extension = extension_of(path);
        match candidates.len() {
            0 => Routed::Plain,
            1 => Routed::By(candidates.into_iter().next().unwrap_or_else(|| unreachable!())),
            _ => match self.chosen_for(&extension) {
                Some(chosen) if chosen.is_empty() => Routed::Plain,
                Some(chosen) => candidates
                    .iter()
                    .find(|candidate| candidate.id.0 == chosen)
                    .cloned()
                    .map_or(Routed::Ask { extension, candidates: candidates.clone() }, Routed::By),
                None => Routed::Ask { extension, candidates },
            },
        }
    }

    pub(crate) fn for_path(&self, path: &str) -> Option<Chosen> {
        match self.route(path) {
            Routed::By(chosen) => Some(chosen),
            Routed::Plain | Routed::Ask { .. } => None,
        }
    }
}

pub(crate) fn install_readers(cx: &mut App) {
    cx.set_global(Readers {
        catalog: None,
        choices: load_choices(),
    });
}

pub(crate) fn start_readers(cx: &mut App) {
    cx.spawn(async move |cx| {
        let catalog = cx
            .background_spawn(async {
                let mut catalog =
                    ReaderCatalog::discover(&Folders::of_this_sternegit(READERS_FOLDER), ReaderApprovals::load());
                catalog.start(&Arc::new(()));
                catalog
            })
            .await;
        let _ = cx.update(|cx| {
            cx.update_global::<Readers, _>(|readers, _| readers.catalog = Some(catalog));
        });
    })
    .detach();
}

fn not_ready() -> io::Error {
    io::Error::other("SterneGit is still starting its readers")
}

pub(crate) fn approve(cx: &mut App, id: &ReaderId) -> io::Result<()> {
    cx.update_global::<Readers, _>(|readers, _| {
        let catalog = readers.catalog.as_mut().ok_or_else(not_ready)?;
        catalog.approve(id)?;
        catalog.start(&Arc::new(()));
        Ok(())
    })
}

pub(crate) fn mark_seen(cx: &mut App, id: &ReaderId) {
    cx.update_global::<Readers, _>(|readers, _| {
        if let Some(catalog) = readers.catalog.as_mut() {
            let _ = catalog.seen(id);
        }
    })
}

pub(crate) fn choose(cx: &mut App, extension: &str, reader: Option<&ReaderId>) -> io::Result<()> {
    cx.update_global::<Readers, _>(|readers, _| {
        readers
            .choices
            .insert(extension.to_owned(), reader.map_or_else(String::new, |id| id.0.clone()));
        let Some(path) = choices_file() else {
            return Ok(());
        };
        let listed: serde_json::Map<String, Value> = readers
            .choices
            .iter()
            .map(|(extension, id)| (extension.clone(), Value::from(id.as_str())))
            .collect();
        JsonFile::at(path).update(|fields| {
            fields.insert(EXTENSIONS.into(), Value::Object(listed));
        })
    })
}

#[cfg(test)]
mod tests {
    use super::extension_of;

    #[test]
    fn the_extension_is_the_lowercase_end_of_the_file_name() {
        assert_eq!(extension_of("Assets/Data/Stats.ASSET"), ".asset");
        assert_eq!(extension_of("Assets.v2/README"), "");
        assert_eq!(extension_of("Hero.png.meta"), ".meta");
    }
}
