use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use sternegit_protocol::{Cursor, Owner, Repository, RepositoryQuery, Visibility};
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, PathPromptOptions, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::PopupMenuItem,
        spinner::Spinner,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    clone_view::CloneJob,
    failure::{Fix, Trouble},
    parts::{Note, Picker, account_avatar, chip, field, server_name, toggle_pair},
    sign_in::{SignInStart, open_sign_in},
    ssh_key::local_key,
};
use crate::{
    accounts::{Account, AccountId, access_as, access_for_url},
    app::connectors::{Connectors, change_account, choose_owner, note_failure},
    git::{
        CloneRequest, is_empty_or_missing,
        ssh_key::{known_hosts_file, knows_host, ssh_command},
    },
    ui::{Look as _, SectionHeader, is_double_click, list_row, run_async_in_background},
};

const WIDTH: f32 = 880.;
const BODY_HEIGHT: f32 = 560.;
const SOURCES_WIDTH: f32 = 210.;
const TITLE: &str = "Clone a repository";
const URL: &str = "URL";
const ACCOUNTS: &str = "Accounts";
const ADD_ACCOUNT: &str = "Add account…";
const REPOSITORY_URL: &str = "Repository URL";
const URL_HINT: &str = "Anything git clone accepts: https://, ssh:// or user@host:path. \
     When an account fits the URL, SterneGit lends Git its token.";
const CLONE_INTO: &str = "Clone into";
const BROWSE: &str = "Browse…";
const PROTOCOL: &str = "Protocol";
const ALL_OWNERS: &str = "All owners";
const LOAD_MORE: &str = "Load more";
const CANCEL: &str = "Cancel";
const CLONE: &str = "Clone";
const SIGN_IN_AGAIN: &str = "Sign in again";
const RETRY: &str = "Retry";
const NO_REPOSITORIES: &str = "No repositories match.";
const NOT_EMPTY: &str = "This folder isn't empty: choose another one.";
const SSH_NEEDS_KEY: &str = "Create an SSH key for this account first, in Settings → Accounts";
const SEARCH_AFTER: Duration = Duration::from_millis(350);

type OnClone = Rc<dyn Fn(CloneJob, &mut Window, &mut App)>;

#[derive(Clone, PartialEq, Eq)]
enum Source {
    Url,
    Account(AccountId),
}

enum Listing {
    Loading(#[expect(dead_code)] Task<()>),
    Shown {
        items: Vec<Repository>,
        next: Option<Cursor>,
        more: Option<Task<()>>,
    },
    Failed(Trouble),
}

struct CloneForm {
    source: Source,
    url: Entity<InputState>,
    folder: Entity<InputState>,
    search: Entity<InputState>,
    owner: Option<String>,
    owners: Vec<Owner>,
    listing: Option<Listing>,
    selected: Option<usize>,
    over_ssh: bool,
    named: bool,
    suggested_for: Option<String>,
    waiting_to_search: Option<Task<()>>,
    on_clone: OnClone,
    _subscriptions: Vec<Subscription>,
}

impl CloneForm {
    fn new(parent: PathBuf, on_clone: OnClone, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("https://github.com/owner/name.git"));
        let folder = cx.new(|cx| InputState::new(window, cx).default_value(parent.display().to_string()));
        let search = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.subscribe_in(&url, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let typed = this.url.read(cx).value().to_string();
                    this.suggest_folder(&name_in_url(&typed), window, cx);
                    cx.notify();
                }
            }),
            cx.subscribe(&folder, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
            cx.subscribe(&search, |this, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.search_soon(cx);
                }
            }),
            cx.observe_global::<Connectors>(|_, cx| cx.notify()),
        ];
        Self {
            source: Source::Url,
            url,
            folder,
            search,
            owner: None,
            owners: Vec::new(),
            listing: None,
            selected: None,
            over_ssh: false,
            named: false,
            suggested_for: None,
            waiting_to_search: None,
            on_clone,
            _subscriptions: subscriptions,
        }
    }

    fn account(&self, cx: &App) -> Option<Account> {
        match &self.source {
            Source::Account(id) => Connectors::of(cx).account(id),
            Source::Url => None,
        }
    }

    fn suggest_folder(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let current = PathBuf::from(self.folder.read(cx).value().trim());
        let parent = match self.named {
            true => current.parent().map_or_else(|| current.clone(), Path::to_path_buf),
            false => current,
        };
        self.named = !name.is_empty();
        let suggested = match self.named {
            true => parent.join(name),
            false => parent,
        };
        self.folder
            .update(cx, |state, cx| state.set_value(suggested.display().to_string(), window, cx));
    }

    fn pick(&mut self, source: Source, window: &mut Window, cx: &mut Context<Self>) {
        if self.source == source {
            return;
        }
        self.source = source.clone();
        self.selected = None;
        self.owner = None;
        self.owners.clear();
        self.listing = None;
        match source {
            Source::Url => {
                let typed = self.url.read(cx).value().to_string();
                self.suggest_folder(&name_in_url(&typed), window, cx);
            }
            Source::Account(id) => {
                let account = self.account(cx);
                self.over_ssh = account.as_ref().is_some_and(|account| account.clone_over_ssh && account.ssh_key.is_some());
                let placeholder = account.map_or_else(String::new, |account| format!("Search {}'s repositories", account.login));
                self.search.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                    state.set_placeholder(placeholder, window, cx);
                });
                self.read_owners(&id, cx);
                self.list(cx);
            }
        }
        cx.notify();
    }

    fn read_owners(&mut self, id: &AccountId, cx: &mut Context<Self>) {
        let Some(forge) = Connectors::of(cx).forge(id, None) else {
            return;
        };
        run_async_in_background(cx, forge.owners(), |this, owners, cx| {
            if let Ok(owners) = owners {
                this.owners = owners;
                cx.notify();
            }
        })
        .detach();
    }

    fn search_soon(&mut self, cx: &mut Context<Self>) {
        self.waiting_to_search = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SEARCH_AFTER).await;
            let _ = this.update(cx, |this, cx| {
                this.waiting_to_search = None;
                this.list(cx);
            });
        }));
    }

    fn query(&self, cursor: Option<Cursor>, cx: &App) -> RepositoryQuery {
        let search = self.search.read(cx).value().trim().to_owned();
        RepositoryQuery {
            search: (!search.is_empty()).then_some(search),
            owner: self.owner.clone(),
            cursor,
        }
    }

    fn list(&mut self, cx: &mut Context<Self>) {
        let Source::Account(id) = self.source.clone() else {
            return;
        };
        let Some(forge) = Connectors::of(cx).forge(&id, None) else {
            return;
        };
        let asked = forge.repositories(self.query(None, cx));
        self.selected = None;
        let task = run_async_in_background(cx, asked, move |this, page, cx| {
            if this.source != Source::Account(id.clone()) {
                return;
            }
            this.listing = Some(match page {
                Ok(page) => Listing::Shown {
                    items: page.items,
                    next: page.next,
                    more: None,
                },
                Err(error) => {
                    note_failure(cx, &id, &error);
                    Listing::Failed(Trouble::of(&error, &id, cx))
                }
            });
            this.select_first(cx);
            cx.notify();
        });
        self.listing = Some(Listing::Loading(task));
        cx.notify();
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        let Source::Account(id) = self.source.clone() else {
            return;
        };
        let Some(Listing::Shown { next: Some(next), more: None, .. }) = &self.listing else {
            return;
        };
        let Some(forge) = Connectors::of(cx).forge(&id, None) else {
            return;
        };
        let asked = forge.repositories(self.query(Some(next.clone()), cx));
        let task = run_async_in_background(cx, asked, move |this, page, cx| {
            if let Some(Listing::Shown { items, next, more }) = &mut this.listing {
                *more = None;
                match page {
                    Ok(page) => {
                        items.extend(page.items);
                        *next = page.next;
                    }
                    Err(error) => {
                        note_failure(cx, &id, &error);
                        this.listing = Some(Listing::Failed(Trouble::of(&error, &id, cx)));
                    }
                }
            }
            cx.notify();
        });
        if let Some(Listing::Shown { more, .. }) = &mut self.listing {
            *more = Some(task);
        }
        cx.notify();
    }

    fn select_first(&mut self, cx: &mut Context<Self>) {
        let has_items = matches!(&self.listing, Some(Listing::Shown { items, .. }) if !items.is_empty());
        if has_items && self.selected.is_none() {
            self.selected = Some(0);
        }
        cx.notify();
    }

    fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected = Some(ix);
        cx.notify();
    }

    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Clone into".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(chosen) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                let name = this.chosen_name(cx);
                this.named = !name.is_empty();
                let folder = match this.named {
                    true => chosen.join(name),
                    false => chosen,
                };
                this.folder.update(cx, |state, cx| state.set_value(folder.display().to_string(), window, cx));
                cx.notify();
            });
        })
        .detach();
    }

    fn chosen_name(&self, cx: &App) -> String {
        match (&self.source, &self.listing, self.selected) {
            (Source::Url, ..) => name_in_url(&self.url.read(cx).value()),
            (Source::Account(_), Some(Listing::Shown { items, .. }), Some(ix)) => {
                items.get(ix).map_or_else(String::new, |item| item.repo.name.clone())
            }
            _ => String::new(),
        }
    }

    fn folder_problem(&self, cx: &App) -> Option<&'static str> {
        if !self.named {
            return None;
        }
        let folder = self.folder.read(cx).value().to_string();
        (!folder.trim().is_empty() && !is_empty_or_missing(Path::new(folder.trim()))).then_some(NOT_EMPTY)
    }

    fn ready(&self, cx: &App) -> bool {
        let folder = self.folder.read(cx).value();
        if folder.trim().is_empty() || self.folder_problem(cx).is_some() {
            return false;
        }
        match &self.source {
            Source::Url => !self.url.read(cx).value().trim().is_empty(),
            Source::Account(_) => self.chosen_repository().is_some(),
        }
    }

    fn chosen_repository(&self) -> Option<&Repository> {
        match (&self.listing, self.selected) {
            (Some(Listing::Shown { items, .. }), Some(ix)) => items.get(ix),
            _ => None,
        }
    }

    fn clone(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.ready(cx) {
            return;
        }
        let folder = PathBuf::from(self.folder.read(cx).value().trim());
        let job = match self.source.clone() {
            Source::Url => {
                let url = self.url.read(cx).value().trim().to_owned();
                CloneJob {
                    shown: url.clone(),
                    request: CloneRequest {
                        access: access_for_url(&url),
                        url,
                        folder,
                        ssh_command: None,
                    },
                    uses_key_of: None,
                }
            }
            Source::Account(id) => {
                let (Some(account), Some(chosen)) = (self.account(cx), self.chosen_repository().cloned()) else {
                    return;
                };
                let key = local_key(&account).filter(|_| self.over_ssh);
                let ssh_url = chosen.clone.ssh.clone().filter(|_| key.is_some());
                let shown = format!("{}/{}", chosen.repo.owner, chosen.repo.name);
                if Connectors::of(cx).accounts().iter().filter(|other| other.server == account.server).count() > 1 {
                    if let Some(host) = account.server.host_str() {
                        let _ = choose_owner(cx, host, &chosen.repo.owner, account.id.clone());
                    }
                }
                let over_ssh = ssh_url.is_some();
                let _ = change_account(cx, &id, |account| account.clone_over_ssh = over_ssh);
                let (url, ssh_command, uses_key_of) = match (ssh_url, key, known_hosts_file()) {
                    (Some(url), Some(key), Some(known_hosts)) => {
                        let strict = ssh_host(&url).is_some_and(|host| knows_host(&known_hosts, &host));
                        (url, Some(ssh_command(&key.private, &known_hosts, strict)), Some(id.clone()))
                    }
                    _ => (chosen.clone.https.clone(), None, None),
                };
                CloneJob {
                    shown,
                    request: CloneRequest {
                        url,
                        folder,
                        access: access_as(&account, &chosen.repo.owner),
                        ssh_command,
                    },
                    uses_key_of,
                }
            }
        };
        window.close_dialog(cx);
        (self.on_clone)(job, window, cx);
    }

    fn sources(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let connectors = Connectors::of(cx);
        let accounts = connectors.accounts();
        let theme = cx.theme();
        let (muted, yellow, border) = (theme.muted_foreground, theme.yellow, theme.border);
        let mut list = v_flex()
            .flex_none()
            .w(px(SOURCES_WIDTH))
            .gap_0p5()
            .pr_3()
            .border_r_1()
            .border_color(border)
            .child(
                list_row("clone-from-url", self.source == Source::Url)
                    .px_2()
                    .rounded(px(6.))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Icon::new(IconName::Link).small().text_color(muted))
                            .child(div().sm_text().child(URL)),
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.pick(Source::Url, window, cx))),
            )
            .child(SectionHeader::new(ACCOUNTS));
        for (ix, account) in accounts.iter().enumerate() {
            let signed_out = connectors.is_signed_out(&account.id);
            let id = account.id.clone();
            let side = match signed_out {
                true => Icon::new(IconName::TriangleAlert).xsmall().text_color(yellow).into_any_element(),
                false => div()
                    .xs_text()
                    .text_color(muted)
                    .child(match account.service.contains('.') {
                        true => super::accounts_page::service_label(account, cx),
                        false => server_name(account),
                    })
                    .into_any_element(),
            };
            list = list.child(
                list_row(("clone-from-account", ix), self.source == Source::Account(id.clone()))
                    .px_2()
                    .rounded(px(6.))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_2()
                            .child(account_avatar(account, 18.))
                            .child(div().flex_1().sm_text().ellipsis().child(account.login.clone()))
                            .child(side),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| this.pick(Source::Account(id.clone()), window, cx))),
            );
        }
        list.child(
            list_row("clone-add-account", false)
                .px_2()
                .rounded(px(6.))
                .child(
                    h_flex()
                        .gap_2()
                        .child(Icon::new(IconName::Plus).small().text_color(muted))
                        .child(div().sm_text().child(ADD_ACCOUNT)),
                )
                .on_click(|_, window, cx| open_sign_in(SignInStart::Fresh, window, cx)),
        )
    }

    fn folder_field(&self, cx: &mut Context<Self>) -> AnyElement {
        field(
            CLONE_INTO,
            h_flex()
                .gap_2()
                .child(div().flex_1().child(Input::new(&self.folder)))
                .child(
                    Button::new("clone-browse")
                        .label(BROWSE)
                        .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
                ),
            self.folder_problem(cx).map(|problem| Note::Problem(problem.into())),
            cx,
        )
        .into_any_element()
    }

    fn url_view(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .gap_4()
            .child(field(REPOSITORY_URL, Input::new(&self.url), Some(Note::Plain(URL_HINT.into())), cx))
            .child(self.folder_field(cx))
            .into_any_element()
    }

    fn account_view(&self, account: &Account, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        if let Some(Listing::Failed(trouble)) = &self.listing {
            let id = account.id.clone();
            let action = match trouble.fix {
                Fix::SignInAgain | Fix::SignInWithBrowser => Some(
                    Button::new("clone-sign-in-again")
                        .small()
                        .label(SIGN_IN_AGAIN)
                        .on_click(move |_, window, cx| open_sign_in(SignInStart::Again(id.clone()), window, cx))
                        .into_any_element(),
                ),
                Fix::Retry => Some(
                    Button::new("clone-retry")
                        .small()
                        .label(RETRY)
                        .on_click(cx.listener(|this, _, _, cx| this.list(cx)))
                        .into_any_element(),
                ),
                Fix::CopyLog | Fix::Nothing => None,
            };
            return trouble.render(action, cx).into_any_element();
        }
        let owners = self.owners.clone();
        let this = cx.entity().downgrade();
        let filter = Picker::new("clone-owner", self.owner.clone().unwrap_or_else(|| ALL_OWNERS.to_owned())).menu(
            move |menu, _, _| {
                let all = this.clone();
                let menu = menu.item(PopupMenuItem::new(ALL_OWNERS).on_click(move |_, _, cx| {
                    let _ = all.update(cx, |this, cx| {
                        this.owner = None;
                        this.list(cx);
                    });
                }));
                owners.iter().fold(menu, |menu, owner| {
                    let this = this.clone();
                    let login = owner.login.clone();
                    menu.item(PopupMenuItem::new(owner.login.clone()).on_click(move |_, _, cx| {
                        let _ = this.update(cx, |this, cx| {
                            this.owner = Some(login.clone());
                            this.list(cx);
                        });
                    }))
                })
            },
        );
        let filters = h_flex()
            .gap_2()
            .child(div().flex_none().w(px(180.)).child(filter))
            .child(
                div().flex_1().child(
                    Input::new(&self.search)
                        .prefix(Icon::new(IconName::Search).small().text_color(muted))
                        .cleanable(true),
                ),
            );
        let list = match &self.listing {
            None | Some(Listing::Loading(_)) => v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(Spinner::new())
                .into_any_element(),
            Some(Listing::Shown { items, next, more }) if items.is_empty() && next.is_none() && more.is_none() => {
                v_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .sm_text()
                    .text_color(muted)
                    .child(NO_REPOSITORIES)
                    .into_any_element()
            }
            Some(Listing::Shown { items, next, more }) => {
                let rows = items
                    .iter()
                    .enumerate()
                    .map(|(ix, item)| self.repository_row(ix, item, cx))
                    .collect::<Vec<_>>();
                let loading_more = more.is_some();
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .id("clone-repositories")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .children(rows),
                    )
                    .when(next.is_some() || loading_more, |this| {
                        this.child(
                            h_flex().flex_none().justify_center().p_1p5().border_t_1().border_color(border).child(
                                Button::new("clone-load-more")
                                    .ghost()
                                    .small()
                                    .label(LOAD_MORE)
                                    .loading(loading_more)
                                    .on_click(cx.listener(|this, _, _, cx| this.load_more(cx))),
                            ),
                        )
                    })
                    .into_any_element()
            }
            Some(Listing::Failed(_)) => div().into_any_element(),
        };
        let has_key = account.ssh_key.is_some();
        let chosen = usize::from(self.over_ssh && has_key);
        let protocol = toggle_pair(
            "clone-protocol",
            ["HTTPS", "SSH"],
            chosen,
            [true, has_key],
            [None, (!has_key).then_some(SSH_NEEDS_KEY)],
            {
                let this = cx.entity().downgrade();
                move |ix, _, cx| {
                    let _ = this.update(cx, |this, cx| {
                        this.over_ssh = ix == 1;
                        cx.notify();
                    });
                }
            },
            cx,
        );
        v_flex()
            .size_full()
            .gap_3()
            .child(filters)
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(border)
                    .rounded(px(8.))
                    .overflow_hidden()
                    .child(list),
            )
            .child(
                h_flex()
                    .items_end()
                    .gap_3()
                    .child(div().flex_1().child(self.folder_field(cx)))
                    .child(field(PROTOCOL, protocol, None, cx)),
            )
            .into_any_element()
    }

    fn repository_row(&self, ix: usize, item: &Repository, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, yellow, hover, selected_bg, border) =
            (theme.muted_foreground, theme.yellow, theme.list_hover, theme.list_active, theme.border);
        let selected = self.selected == Some(ix);
        let icon = if item.archived {
            IconName::Archive
        } else if item.fork {
            IconName::GitFork
        } else {
            match item.visibility {
                Visibility::Private => IconName::Lock,
                Visibility::Internal => IconName::Building2,
                Visibility::Public | Visibility::Unknown => IconName::Globe,
            }
        };
        let mut chips = Vec::new();
        match item.visibility {
            Visibility::Private => chips.push(chip("Private", None, cx)),
            Visibility::Internal => chips.push(chip("Internal", None, cx)),
            Visibility::Public | Visibility::Unknown => {}
        }
        if item.fork {
            chips.push(chip("Fork", None, cx));
        }
        if item.archived {
            chips.push(chip("Archived", Some(yellow), cx));
        }
        h_flex()
            .id(("clone-repository", ix))
            .gap_3()
            .px_3()
            .py_2()
            .when(ix > 0, |this| this.border_t_1().border_color(border))
            .when(selected, |this| this.bg(selected_bg))
            .hover(move |style| style.bg(hover))
            .child(Icon::new(icon).small().text_color(muted))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .sm_text()
                            .child(div().text_color(muted).child(format!("{}/", item.repo.owner)))
                            .child(div().ml(px(-6.)).font_semibold().child(item.repo.name.clone()))
                            .children(chips),
                    )
                    .children(item.description.clone().map(|description| {
                        div().xs_text().text_color(muted).ellipsis().child(description)
                    })),
            )
            .children(item.updated_at.as_deref().map(|updated| {
                div().flex_none().xs_text().text_color(muted).child(SharedString::from(updated.get(..10).unwrap_or(updated).to_owned()))
            }))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                this.select(ix, cx);
                if is_double_click(event) {
                    this.clone(window, cx);
                }
            }))
            .into_any_element()
    }
}

impl Render for CloneForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Source::Account(_) = self.source {
            let name = self.chosen_name(cx);
            if !name.is_empty() && self.suggested_for.as_deref() != Some(name.as_str()) {
                self.suggested_for = Some(name.clone());
                self.suggest_folder(&name, window, cx);
            }
        }
        let main = match self.account(cx) {
            Some(account) => self.account_view(&account, cx),
            None => self.url_view(cx),
        };
        let ready = self.ready(cx);
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .items_stretch()
                    .gap_4()
                    .h(px(BODY_HEIGHT))
                    .mt_3()
                    .child(self.sources(cx))
                    .child(v_flex().flex_1().min_w_0().child(main)),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("clone-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("clone-confirm")
                            .primary()
                            .icon(IconName::Download)
                            .label(CLONE)
                            .disabled(!ready)
                            .on_click(cx.listener(|this, _, window, cx| this.clone(window, cx))),
                    ),
            )
    }
}

pub(crate) fn name_in_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches(['/', '\\']);
    let last = trimmed.rsplit(['/', '\\', ':']).next().unwrap_or_default();
    last.strip_suffix(".git").unwrap_or(last).to_owned()
}

fn ssh_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("ssh://").unwrap_or(url);
    let rest = rest.split_once('@').map_or(rest, |(_, host)| host);
    let host = rest.split([':', '/']).next()?;
    (!host.is_empty()).then(|| host.to_owned())
}

pub(crate) fn open_clone_dialog(
    parent: PathBuf,
    window: &mut Window,
    cx: &mut App,
    on_clone: impl Fn(CloneJob, &mut Window, &mut App) + 'static,
) {
    let on_clone: OnClone = Rc::new(on_clone);
    let form = cx.new(|cx| CloneForm::new(parent, on_clone, window, cx));
    let url = form.read(cx).url.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        let cloned = form.clone();
        dialog
            .w(px(WIDTH))
            .title(TITLE)
            .child(form.clone())
            .on_ok(move |_, window, cx| {
                cloned.update(cx, |form, cx| form.clone(window, cx));
                false
            })
    });
    window.defer(cx, move |window, cx| url.update(cx, |state, cx| state.focus(window, cx)));
}

#[cfg(test)]
mod tests {
    use super::{name_in_url, ssh_host};

    #[test]
    fn the_folder_is_named_after_the_last_part_of_the_url() {
        assert_eq!(name_in_url("https://github.com/sternegit-org/connectors.git"), "connectors");
        assert_eq!(name_in_url("git@github.com:sternegit-org/sternegit.git"), "sternegit");
        assert_eq!(name_in_url("ssh://git@host:2222/team/app/"), "app");
        assert_eq!(name_in_url("git@host:app"), "app");
        assert_eq!(name_in_url(""), "");
    }

    #[test]
    fn the_ssh_host_is_read_from_both_url_forms() {
        assert_eq!(ssh_host("git@github.com:o/n.git").as_deref(), Some("github.com"));
        assert_eq!(ssh_host("ssh://git@ssh.github.com:443/o/n.git").as_deref(), Some("ssh.github.com"));
    }
}
