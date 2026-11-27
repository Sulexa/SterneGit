use std::sync::Arc;

use gpui_kit::{App, AppContext as _, Global};

use crate::{
    plugin::Folders,
    read::{FileKind, READERS_FOLDER, Reader, ReaderApprovals, ReaderCatalog},
};

#[derive(Default)]
pub(crate) struct Readers {
    catalog: Option<ReaderCatalog>,
}

impl Global for Readers {}

#[derive(Clone)]
pub(crate) struct Chosen {
    pub(crate) reader: Arc<Reader>,
    pub(crate) kind: FileKind,
}

impl Readers {
    pub(crate) fn of(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn for_path(&self, path: &str) -> Option<Chosen> {
        self.catalog.as_ref()?.plugins().iter().find_map(|plugin| {
            let reader = plugin.started.clone()?;
            let manifest = plugin.manifest.clone()?;
            let kind = manifest.kind_for(path, None)?.clone();
            Some(Chosen { reader, kind })
        })
    }
}

pub(crate) fn install_readers(cx: &mut App) {
    cx.set_global(Readers::default());
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
        let _ = cx.update(|cx| cx.set_global(Readers { catalog: Some(catalog) }));
    })
    .detach();
}
