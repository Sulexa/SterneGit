use std::sync::Arc;

use sternegit_protocol::{Cursor, Page, PullState, PullSummary, RepoRef, StateFilter};
use gpui_kit::{App, Context, Entity, SharedString, Subscription, Task};

use super::super::repository::{Repository, RepositoryEvent};
use crate::{
    accounts::AccountId,
    app::connectors::{Connectors, note_failure},
    connect::{CallError, Forge, Remote},
    git::NamedRemote,
    hosting::Trouble,
    ui::{run_async_in_background, run_in_background},
};

const LISTS_PULLS: &str = "pulls";
const CREATES_PULLS: &str = "pulls.create";
const MERGES_PULLS: &str = "pulls.merge";
const PREFERRED_REMOTE: &str = "origin";

#[derive(Clone)]
pub(crate) struct Hosting {
    pub(crate) forge: Forge,
    pub(crate) account: AccountId,
    pub(crate) remote: String,
    pub(crate) repo: RepoRef,
    pub(crate) service: SharedString,
    pub(crate) creates: bool,
    pub(crate) merges: bool,
}

impl Hosting {
    fn same_place(&self, other: &Hosting) -> bool {
        self.account == other.account && self.remote == other.remote && self.repo == other.repo
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    Unclaimed,
    NoAccount { service: SharedString, server: String },
    Choosing,
    SignedOut { account: AccountId, login: String },
    Hosted,
}

pub(crate) enum Listing {
    Idle,
    Reading(#[expect(dead_code)] Task<()>),
    Read {
        pulls: Arc<[PullSummary]>,
        next: Option<Cursor>,
    },
    Failed(Trouble),
}

pub(crate) struct Pulls {
    remotes: Vec<NamedRemote>,
    reach: Reach,
    hosting: Option<Hosting>,
    filter: StateFilter,
    listing: Listing,
    open: Arc<[PullSummary]>,
    rereading: Option<Task<()>>,
    more: Option<Task<()>>,
    default_branch: Option<String>,
    reading_default_branch: Option<Task<()>>,
    repository: Entity<Repository>,
    _subscriptions: [Subscription; 2],
}

impl Pulls {
    pub(crate) fn new(repository: Entity<Repository>, cx: &mut Context<Self>) -> Self {
        let subscriptions = [
            cx.observe_global::<Connectors>(|this, cx| this.settle(cx)),
            cx.subscribe(&repository, |this, _, event, cx| {
                if matches!(event, RepositoryEvent::Fetched) {
                    this.reread(cx)
                }
            }),
        ];
        Self {
            remotes: Vec::new(),
            reach: Reach::Unclaimed,
            hosting: None,
            filter: StateFilter::Open,
            listing: Listing::Idle,
            open: Arc::from([]),
            rereading: None,
            more: None,
            default_branch: None,
            reading_default_branch: None,
            repository,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn reach(&self) -> &Reach {
        &self.reach
    }

    pub(crate) fn hosting(&self) -> Option<&Hosting> {
        self.hosting.as_ref()
    }

    pub(crate) fn filter(&self) -> StateFilter {
        self.filter
    }

    pub(crate) fn listing(&self) -> &Listing {
        &self.listing
    }

    pub(crate) fn is_loading_more(&self) -> bool {
        self.more.is_some()
    }

    pub(crate) fn default_branch(&self) -> Option<&str> {
        self.default_branch.as_deref()
    }

    pub(crate) fn listed(&self) -> &[PullSummary] {
        match &self.listing {
            Listing::Read { pulls, .. } => pulls,
            Listing::Idle | Listing::Reading(_) | Listing::Failed(_) => &[],
        }
    }

    pub(crate) fn open(&self) -> &[PullSummary] {
        &self.open
    }

    pub(crate) fn find(&self, number: u64) -> Option<&PullSummary> {
        self.listed()
            .iter()
            .chain(self.open.iter())
            .find(|pull| pull.number == number)
    }

    pub(crate) fn track(&mut self, remotes: Vec<NamedRemote>, cx: &mut Context<Self>) {
        if self.remotes != remotes {
            self.remotes = remotes;
            self.settle(cx);
        }
    }

    pub(crate) fn choose_filter(&mut self, filter: StateFilter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            self.list(cx);
        }
    }

    pub(crate) fn retry(&mut self, cx: &mut Context<Self>) {
        self.list(cx);
    }

    pub(crate) fn reread(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.listing, Listing::Read { .. }) {
            if matches!(self.listing, Listing::Failed(_)) {
                self.list(cx);
            }
            return;
        }
        let Some(hosting) = self.hosting.clone() else {
            return;
        };
        let read = hosting.forge.pulls(self.filter, None);
        self.rereading = Some(run_async_in_background(cx, read, move |this, result, cx| {
            this.rereading = None;
            match result {
                Ok(page) => this.show(page),
                Err(error) => this.fail(&hosting.account, &error, cx),
            }
            cx.notify();
        }));
    }

    pub(crate) fn load_more(&mut self, cx: &mut Context<Self>) {
        let (Some(hosting), Listing::Read { next: Some(next), .. }) = (self.hosting.clone(), &self.listing) else {
            return;
        };
        if self.more.is_some() {
            return;
        }
        let read = hosting.forge.pulls(self.filter, Some(next.clone()));
        self.more = Some(run_async_in_background(cx, read, move |this, result, cx| {
            this.more = None;
            match result {
                Ok(page) => this.append(page),
                Err(error) => this.fail(&hosting.account, &error, cx),
            }
            cx.notify();
        }));
        cx.notify();
    }

    fn settle(&mut self, cx: &mut Context<Self>) {
        let (reach, hosting) = reach_of(&self.remotes, cx);
        let moved = match (&self.hosting, &hosting) {
            (Some(shown), Some(found)) => !shown.same_place(found),
            (None, None) => false,
            _ => true,
        };
        let remote = hosting.as_ref().map(|hosting| hosting.remote.clone());
        self.reach = reach;
        self.hosting = hosting;
        if moved {
            self.listing = Listing::Idle;
            self.open = Arc::from([]);
            self.rereading = None;
            self.more = None;
            self.default_branch = None;
            self.list(cx);
            self.read_default_branch(remote, cx);
        }
        cx.notify();
    }

    fn list(&mut self, cx: &mut Context<Self>) {
        self.rereading = None;
        self.more = None;
        let Some(hosting) = self.hosting.clone() else {
            self.listing = Listing::Idle;
            cx.notify();
            return;
        };
        let read = hosting.forge.pulls(self.filter, None);
        self.listing = Listing::Reading(run_async_in_background(cx, read, move |this, result, cx| {
            match result {
                Ok(page) => this.show(page),
                Err(error) => this.fail(&hosting.account, &error, cx),
            }
            cx.notify();
        }));
        cx.notify();
    }

    fn read_default_branch(&mut self, remote: Option<String>, cx: &mut Context<Self>) {
        let (Some(remote), Some(repo)) = (remote, self.repository.read(cx).repo().cloned()) else {
            return;
        };
        self.reading_default_branch = Some(run_in_background(
            cx,
            move || repo.default_branch_of(&remote),
            |this, branch, cx| {
                this.reading_default_branch = None;
                this.default_branch = branch;
                cx.notify();
            },
        ));
    }

    fn show(&mut self, page: Page<PullSummary>) {
        let pulls: Arc<[PullSummary]> = page.items.into();
        if self.filter != StateFilter::Closed {
            self.open = pulls.iter().filter(|pull| pull.state == PullState::Open).cloned().collect();
        }
        self.listing = Listing::Read {
            pulls,
            next: page.next,
        };
    }

    fn append(&mut self, page: Page<PullSummary>) {
        if let Listing::Read { pulls, next } = &mut self.listing {
            let mut all = pulls.to_vec();
            let added: Vec<PullSummary> = page
                .items
                .into_iter()
                .filter(|added| all.iter().all(|known| known.number != added.number))
                .collect();
            all.extend(added);
            *pulls = all.into();
            *next = page.next;
        }
    }

    fn fail(&mut self, account: &AccountId, error: &CallError, cx: &mut Context<Self>) {
        note_failure(cx, account, error);
        self.listing = Listing::Failed(Trouble::of(error, account, cx));
    }
}

fn reach_of(remotes: &[NamedRemote], cx: &App) -> (Reach, Option<Hosting>) {
    let connectors = Connectors::of(cx);
    let mut ordered: Vec<&NamedRemote> = remotes.iter().filter(|remote| remote.name == PREFERRED_REMOTE).collect();
    ordered.extend(remotes.iter().filter(|remote| remote.name != PREFERRED_REMOTE));
    for remote in ordered {
        for claim in connectors.claims(&remote.url) {
            let Some(manifest) = connectors.connector(&claim.connector).and_then(|connector| connector.manifest.clone()) else {
                continue;
            };
            let offers = |feature: &str| manifest.features.iter().any(|offered| offered == feature);
            if !offers(LISTS_PULLS) {
                continue;
            }
            let service: SharedString = manifest
                .services
                .iter()
                .find(|service| service.id == claim.service)
                .map_or_else(|| manifest.name.clone(), |service| service.label.clone())
                .into();
            let account = match claim.accounts.as_slice() {
                [] => {
                    let server = Remote::parse(&remote.url).map_or_else(String::new, |parsed| parsed.host);
                    return (Reach::NoAccount { service, server }, None);
                }
                [account] => account.clone(),
                _ => return (Reach::Choosing, None),
            };
            if connectors.is_signed_out(&account) {
                let login = connectors.account(&account).map_or_else(String::new, |found| found.login);
                return (Reach::SignedOut { account, login }, None);
            }
            let Some(forge) = connectors.forge(&account, Some(claim.repo.clone())) else {
                continue;
            };
            let hosting = Hosting {
                forge,
                account,
                remote: remote.name.clone(),
                repo: claim.repo,
                service,
                creates: offers(CREATES_PULLS),
                merges: offers(MERGES_PULLS),
            };
            return (Reach::Hosted, Some(hosting));
        }
    }
    (Reach::Unclaimed, None)
}
