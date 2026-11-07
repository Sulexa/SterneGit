use std::{
    collections::BTreeSet,
    io,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard},
};

use sternegit_protocol::RepoRef;
use gpui_kit::{App, AppContext as _, BorrowAppContext as _, Global};

use crate::{
    accounts::{Account, AccountBook, AccountId, Secrets, Token, os_secrets},
    connect::{
        Approvals, CallError, Catalog, Claim, Connector, ConnectorFolders, Forge, Manifest,
        Service, Started,
    },
    net::{
        curl::{Curl, Transport},
        gateway::Gateway,
        request::NetError,
        services::{ConnectorId, HostServices},
    },
};

#[derive(Default)]
pub(crate) struct Connectors {
    running: Option<Running>,
}

struct Running {
    catalog: Catalog,
    gateway: Arc<Gateway>,
    book: Arc<RwLock<AccountBook>>,
    secrets: Arc<dyn Secrets>,
    transport: Arc<dyn Transport>,
    signed_out: BTreeSet<AccountId>,
    connected: Vec<AccountId>,
}

#[derive(Clone)]
pub(crate) struct ServiceChoice {
    pub(crate) manifest: Arc<Manifest>,
    pub(crate) index: usize,
}

impl ServiceChoice {
    pub(crate) fn service(&self) -> &Service {
        &self.manifest.services[self.index]
    }

    pub(crate) fn connector(&self) -> &ConnectorId {
        &self.manifest.id
    }

    pub(crate) fn is(&self, other: &ServiceChoice) -> bool {
        self.manifest.id == other.manifest.id && self.index == other.index
    }
}

impl Global for Connectors {}

impl Connectors {
    pub(crate) fn of(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.running.is_some()
    }

    pub(crate) fn accounts(&self) -> Vec<Account> {
        self.book().map(|book| book.accounts.clone()).unwrap_or_default()
    }

    pub(crate) fn account(&self, id: &AccountId) -> Option<Account> {
        self.book()?.account(id).cloned()
    }

    pub(crate) fn book_copy(&self) -> AccountBook {
        self.book().map(|book| book.clone()).unwrap_or_default()
    }

    pub(crate) fn is_signed_out(&self, id: &AccountId) -> bool {
        self.running
            .as_ref()
            .is_some_and(|running| running.signed_out.contains(id))
    }

    pub(crate) fn connections(&self) -> usize {
        self.running.as_ref().map_or(0, |running| running.connected.len())
    }

    pub(crate) fn connected_after(&self, seen: usize) -> &[AccountId] {
        self.running
            .as_ref()
            .and_then(|running| running.connected.get(seen..))
            .unwrap_or_default()
    }

    pub(crate) fn connectors(&self) -> &[Connector] {
        self.running
            .as_ref()
            .map_or(&[], |running| running.catalog.connectors())
    }

    pub(crate) fn approved_version(&self, id: &ConnectorId) -> Option<String> {
        self.running.as_ref()?.catalog.approved_version(id)
    }

    pub(crate) fn connector(&self, id: &ConnectorId) -> Option<&Connector> {
        self.connectors().iter().find(|connector| &connector.id == id)
    }

    pub(crate) fn waiting(&self) -> impl Iterator<Item = &Connector> {
        self.connectors()
            .iter()
            .filter(|connector| connector.standing.awaits_approval())
    }

    pub(crate) fn services(&self) -> Vec<ServiceChoice> {
        self.connectors()
            .iter()
            .filter(|connector| connector.started.is_some())
            .filter_map(|connector| connector.manifest.clone())
            .flat_map(|manifest| {
                (0..manifest.services.len()).map(move |index| ServiceChoice {
                    manifest: Arc::clone(&manifest),
                    index,
                })
            })
            .collect()
    }

    pub(crate) fn service_of(&self, account: &Account) -> Option<ServiceChoice> {
        self.services().into_iter().find(|choice| {
            choice.connector().0 == account.connector && choice.service().id == account.service
        })
    }

    pub(crate) fn started(&self, connector: &ConnectorId) -> Option<Arc<Started>> {
        self.connector(connector)?.started.clone()
    }

    pub(crate) fn forge(&self, account: &AccountId, repo: Option<RepoRef>) -> Option<Forge> {
        let running = self.running.as_ref()?;
        let found = self.account(account)?;
        let started = self.started(&ConnectorId(found.connector))?;
        let host: Arc<dyn HostServices> = running.gateway.clone();
        Some(Forge::new(started, host, account.clone(), repo))
    }

    pub(crate) fn gateway(&self) -> Option<Arc<Gateway>> {
        self.running.as_ref().map(|running| Arc::clone(&running.gateway))
    }

    pub(crate) fn transport(&self) -> Option<Arc<dyn Transport>> {
        self.running.as_ref().map(|running| Arc::clone(&running.transport))
    }

    pub(crate) fn claims(&self, remote_url: &str) -> Vec<Claim> {
        match (&self.running, self.book()) {
            (Some(running), Some(book)) => running.catalog.claims(remote_url, &book),
            _ => Vec::new(),
        }
    }

    pub(crate) fn log_lines(&self, connector: &ConnectorId) -> Vec<String> {
        self.gateway()
            .map(|gateway| gateway.log_lines(connector))
            .unwrap_or_default()
    }

    fn book(&self) -> Option<RwLockReadGuard<'_, AccountBook>> {
        let running = self.running.as_ref()?;
        Some(running.book.read().unwrap_or_else(PoisonError::into_inner))
    }

    fn change_book<R>(&mut self, change: impl FnOnce(&mut AccountBook) -> R) -> io::Result<R> {
        let running = self.running.as_mut().ok_or_else(not_ready)?;
        let mut book = running.book.write().unwrap_or_else(PoisonError::into_inner);
        let result = change(&mut book);
        book.save()?;
        Ok(result)
    }
}

pub(crate) fn keep_account(cx: &mut App, account: Account, token: &Token) -> io::Result<()> {
    cx.update_global::<Connectors, _>(|connectors, _| {
        let running = connectors.running.as_mut().ok_or_else(not_ready)?;
        running.secrets.write(&account.id, &account.login, token)?;
        running.signed_out.remove(&account.id);
        running.connected.push(account.id.clone());
        connectors.change_book(|book| book.sign_in(account))
    })
}

pub(crate) fn forget_account(cx: &mut App, id: &AccountId) -> io::Result<()> {
    cx.update_global::<Connectors, _>(|connectors, _| {
        let running = connectors.running.as_mut().ok_or_else(not_ready)?;
        running.secrets.erase(id)?;
        running.signed_out.remove(id);
        connectors.change_book(|book| book.remove(id))
    })
}

pub(crate) fn change_account(
    cx: &mut App,
    id: &AccountId,
    change: impl FnOnce(&mut Account),
) -> io::Result<()> {
    cx.update_global::<Connectors, _>(|connectors, _| {
        connectors.change_book(|book| {
            if let Some(account) = book.accounts.iter_mut().find(|account| &account.id == id) {
                change(account);
            }
        })
    })
}

pub(crate) fn choose_owner(cx: &mut App, host: &str, owner: &str, account: AccountId) -> io::Result<()> {
    cx.update_global::<Connectors, _>(|connectors, _| {
        let running = connectors.running.as_mut().ok_or_else(not_ready)?;
        running.connected.push(account.clone());
        connectors.change_book(|book| book.choose(host, owner, account))
    })
}

pub(crate) fn forget_owner(cx: &mut App, host: &str, owner: &str) -> io::Result<()> {
    cx.update_global::<Connectors, _>(|connectors, _| {
        connectors.change_book(|book| {
            book.owners
                .retain(|choice| !(choice.host.eq_ignore_ascii_case(host) && choice.owner == owner))
        })
    })
}

pub(crate) fn approve(cx: &mut App, id: &ConnectorId) -> io::Result<()> {
    cx.update_global::<Connectors, _>(|connectors, _| {
        let running = connectors.running.as_mut().ok_or_else(not_ready)?;
        running.catalog.approve(id)?;
        let host: Arc<dyn HostServices> = running.gateway.clone();
        running.catalog.start(&host);
        Ok(())
    })
}

pub(crate) fn mark_seen(cx: &mut App, id: &ConnectorId) {
    cx.update_global::<Connectors, _>(|connectors, _| {
        if let Some(running) = connectors.running.as_mut() {
            let _ = running.catalog.seen(id);
        }
    })
}

pub(crate) fn note_failure(cx: &mut App, account: &AccountId, error: &CallError) {
    let signed_out = matches!(
        error,
        CallError::Failed(sternegit_protocol::Failure::SignedOut) | CallError::Net(NetError::NotSignedIn)
    );
    if !signed_out || Connectors::of(cx).is_signed_out(account) {
        return;
    }
    cx.update_global::<Connectors, _>(|connectors, _| {
        if let Some(running) = connectors.running.as_mut() {
            running.signed_out.insert(account.clone());
        }
    })
}

pub(crate) fn install_connectors(cx: &mut App) {
    cx.set_global(Connectors::default());
}

pub(crate) fn start_connectors(cx: &mut App) {
    cx.spawn(async move |cx| {
        let running = cx.background_spawn(async { run() }).await;
        let _ = cx.update(|cx| {
            cx.set_global(Connectors {
                running: Some(running),
            })
        });
    })
    .detach();
}

fn run() -> Running {
    let book = Arc::new(RwLock::new(AccountBook::load()));
    let secrets = os_secrets();
    let transport: Arc<dyn Transport> = Arc::new(Curl::system());
    let gateway = Arc::new(Gateway::new(
        Arc::clone(&book),
        Arc::clone(&secrets),
        Arc::clone(&transport),
    ));
    let host: Arc<dyn HostServices> = gateway.clone();
    let mut catalog = Catalog::discover(&ConnectorFolders::of_this_sternegit(), Approvals::load());
    catalog.start(&host);
    Running {
        catalog,
        gateway,
        book,
        secrets,
        transport,
        signed_out: BTreeSet::new(),
        connected: Vec::new(),
    }
}

fn not_ready() -> io::Error {
    io::Error::other("SterneGit is still starting its connectors")
}
