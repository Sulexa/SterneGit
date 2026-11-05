use std::sync::{Arc, RwLock};

use gpui_kit::{App, AppContext as _, Global};

use crate::{
    accounts::{AccountBook, os_secrets},
    connect::{Approvals, Catalog, ConnectorFolders},
    net::{curl::Curl, gateway::Gateway, services::HostServices},
};

#[derive(Default)]
pub(crate) struct Connectors {
    #[expect(dead_code)]
    running: Option<Running>,
}

#[expect(dead_code)]
struct Running {
    catalog: Catalog,
    host: Arc<dyn HostServices>,
    book: Arc<RwLock<AccountBook>>,
}

impl Global for Connectors {}

pub(crate) fn start_connectors(cx: &mut App) {
    cx.set_global(Connectors::default());
    cx.spawn(async move |cx| {
        let running = cx.background_spawn(async { run() }).await;
        let _ = cx.update(|cx| cx.set_global(Connectors { running: Some(running) }));
    })
    .detach();
}

fn run() -> Running {
    let book = Arc::new(RwLock::new(AccountBook::load()));
    let host: Arc<dyn HostServices> = Arc::new(Gateway::new(
        Arc::clone(&book),
        os_secrets(),
        Arc::new(Curl::system()),
    ));
    let mut catalog = Catalog::discover(&ConnectorFolders::of_this_fastgit(), Approvals::load());
    catalog.start(&host);
    Running { catalog, host, book }
}
