use std::{collections::BTreeSet, path::PathBuf, time::Duration};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, PathPromptOptions, Render, SharedString, Styled as _,
    Subscription, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Sizable as _, Theme, ThemeMode, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::AppMenuBar,
        notification::Notification,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use super::{
    connectors::Connectors,
    draws_its_own_menu_bar,
    saved_tabs::SavedTabs,
    tab_strip::{TabLabel, TabState, TabStrip},
    tabs::Tabs,
    top_bar::TopBar,
};
use crate::{
    actions::{
        About, CloneRepository, CloseTab, CopyHash, Fetch, NewRepository, NextTab, OpenRepository,
        OpenSettings, PreviousTab, Quit, Refresh, WINDOW_CONTEXT,
    },
    git,
    hosting::{
        CloneEvent, CloneJob, CloneView, Page, SettingsTab, open_clone_dialog,
        open_new_repository,
    },
    net::services::ConnectorId,
    repo::{RepoView, Repository},
    ui::{Look as _, Notice},
};

const CHECK_FOR_OUTSIDE_CHANGES_EVERY: Duration = Duration::from_secs(3);
const FETCH_IN_THE_BACKGROUND_EVERY: Duration = Duration::from_secs(5 * 60);
const FETCH_ON_RETURN_UNLESS_FETCHED_WITHIN: Duration = Duration::from_secs(60);
const SETTINGS: &str = "Settings";
const REVIEW: &str = "Review…";
const WELCOME_TITLE: &str = "Open a Git repository";
const WELCOME_ABOUT: &str =
    "FastGit reads history straight from the repository on disk. Each repository opens in its own tab.";

struct RepoTab {
    view: Entity<RepoView>,
    repository: Entity<Repository>,
    _title_follows_repository: Subscription,
}

enum Tab {
    Repo(RepoTab),
    Settings(Entity<SettingsTab>),
    Cloning {
        view: Entity<CloneView>,
        _events: Subscription,
    },
}

impl Tab {
    fn repository(&self) -> Option<&Entity<Repository>> {
        match self {
            Tab::Repo(tab) => Some(&tab.repository),
            Tab::Settings(_) | Tab::Cloning { .. } => None,
        }
    }
}

pub(crate) struct FastGit {
    tabs: Tabs<Tab>,
    menu_bar: Option<Entity<AppMenuBar>>,
    announced: BTreeSet<ConnectorId>,
    _poll: Task<()>,
    _activation: Subscription,
    _connectors: Subscription,
}

impl FastGit {
    pub(crate) fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Theme::change(ThemeMode::Dark, Some(window), cx);
        let menu_bar = draws_its_own_menu_bar(cx).then(|| AppMenuBar::new(cx));
        let poll = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(CHECK_FOR_OUTSIDE_CHANGES_EVERY)
                    .await;
                if this.update(cx, |this, cx| this.poll(cx)).is_err() {
                    break;
                }
            }
        });
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.catch_up(cx);
            }
        });
        let connectors = cx.observe_global_in::<Connectors>(window, |this, window, cx| {
            this.announce_waiting_connectors(window, cx);
            cx.notify();
        });
        let mut this = Self {
            tabs: Tabs::default(),
            menu_bar,
            announced: BTreeSet::new(),
            _poll: poll,
            _activation: activation,
            _connectors: connectors,
        };
        this.restore(SavedTabs::load().still_on_disk(), window, cx);
        match path {
            Some(path) => this.open_paths(vec![path], window, cx),
            None => this.show_active_tab(window, cx),
        }
        this
    }

    fn restore(&mut self, saved: SavedTabs, window: &mut Window, cx: &mut Context<Self>) {
        for path in saved.paths {
            let tab = self.new_tab(path, window, cx);
            self.tabs.push(Tab::Repo(tab));
        }
        self.tabs.activate(saved.active);
    }

    fn save_tabs(&self, cx: &App) {
        let active = self.tabs.active_index();
        let mut saved = SavedTabs::default();
        for (ix, tab) in self.tabs.iter().enumerate() {
            if ix == active {
                saved.active = saved.paths.len();
            }
            if let Some(repository) = tab.repository() {
                saved.paths.push(repository.read(cx).path().to_owned());
            }
        }
        saved.active = saved.active.min(saved.paths.len().saturating_sub(1));
        saved.save();
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        let active = self.tabs.active_index();
        for (ix, tab) in self.tabs.iter().enumerate() {
            let Some(repository) = tab.repository() else {
                continue;
            };
            repository.update(cx, |repository, cx| {
                if ix == active {
                    repository.check_for_changes(cx);
                }
                repository.fetch_if_older_than(FETCH_IN_THE_BACKGROUND_EVERY, cx);
            });
        }
    }

    fn catch_up(&self, cx: &mut App) {
        if let Some(repository) = self.tabs.active().and_then(Tab::repository) {
            repository.update(cx, |repository, cx| {
                repository.read_if_never_read(cx);
                repository.check_for_changes(cx);
                repository.fetch_if_older_than(FETCH_ON_RETURN_UNLESS_FETCHED_WITHIN, cx);
            });
        }
    }

    pub(crate) fn open_paths(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let mut last = None;
        for path in paths {
            let path = git::top_folder(&path).unwrap_or(path);
            let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let existing = self.tabs.position(|tab| {
                tab.repository()
                    .is_some_and(|repository| repository.read(cx).shows(&resolved))
            });
            last = Some(match existing {
                Some(ix) => ix,
                None => {
                    let tab = self.new_tab(path, window, cx);
                    self.tabs.push(Tab::Repo(tab))
                }
            });
        }
        if let Some(ix) = last {
            self.activate(ix, window, cx);
        }
    }

    fn new_tab(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> RepoTab {
        let view = cx.new(|cx| RepoView::new(path, window, cx));
        let repository = view.read(cx).repository().clone();
        let title_follows_repository = cx.observe_in(&repository, window, |this, _, window, cx| {
            this.update_title(window, cx);
            cx.notify();
        });
        RepoTab {
            view,
            repository,
            _title_follows_repository: title_follows_repository,
        }
    }

    fn activate(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.activate(ix) {
            self.show_active_tab(window, cx);
        }
    }

    fn close_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.close(ix) {
            self.show_active_tab(window, cx);
        }
    }

    fn cycle_tab(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.cycle(delta) {
            self.show_active_tab(window, cx);
        }
    }

    fn show_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.tabs.active() {
            Some(Tab::Repo(tab)) => {
                let focus = tab.view.read(cx).focus_handle();
                focus.focus(window, cx);
            }
            Some(Tab::Settings(settings)) => {
                let focus = settings.focus_handle(cx);
                focus.focus(window, cx);
            }
            Some(Tab::Cloning { .. }) | None => {}
        }
        self.catch_up(cx);
        self.update_title(window, cx);
        self.save_tabs(cx);
        cx.notify();
    }

    fn update_title(&self, window: &mut Window, cx: &App) {
        let title = match self.tabs.active() {
            Some(Tab::Repo(tab)) => format!("{} — FastGit", tab.repository.read(cx).name()),
            Some(Tab::Settings(_)) => format!("{SETTINGS} — FastGit"),
            Some(Tab::Cloning { view, .. }) => format!("{} — FastGit", view.read(cx).name()),
            None => "FastGit".to_owned(),
        };
        window.set_window_title(&title);
    }

    fn open_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Open repository".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| this.open_paths(paths, window, cx));
        })
        .detach();
    }

    pub(crate) fn open_settings(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        let existing = self.tabs.position(|tab| matches!(tab, Tab::Settings(_)));
        let ix = match existing {
            Some(ix) => {
                if let Some(Tab::Settings(settings)) = self.tabs.iter().nth(ix) {
                    settings.update(cx, |settings, cx| settings.show(page, cx));
                }
                ix
            }
            None => {
                let settings = cx.new(|cx| SettingsTab::new(page, cx));
                self.tabs.push(Tab::Settings(settings))
            }
        };
        self.activate(ix, window, cx);
    }

    fn start_clone(&mut self, job: CloneJob, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|cx| CloneView::new(job, cx));
        let subscription = cx.subscribe_in(&view, window, |this, view, event, window, cx| {
            let Some(ix) = this.tabs.position(|tab| matches!(tab, Tab::Cloning { view: known, .. } if known == view)) else {
                return;
            };
            match event {
                CloneEvent::Cloned(folder) => {
                    let tab = this.new_tab(folder.clone(), window, cx);
                    this.tabs.replace(ix, Tab::Repo(tab));
                    this.show_active_tab(window, cx);
                }
                CloneEvent::Abandoned => this.close_tab(ix, window, cx),
            }
        });
        let ix = self.tabs.push(Tab::Cloning { view, _events: subscription });
        self.activate(ix, window, cx);
    }

    fn clone_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = self.parent_of_repositories(cx);
        let this = cx.entity().downgrade();
        open_clone_dialog(parent, window, cx, move |job, window, cx| {
            let _ = this.update(cx, |this, cx| this.start_clone(job, window, cx));
        });
    }

    fn new_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let parent = self.parent_of_repositories(cx);
        let this = cx.entity().downgrade();
        open_new_repository(parent, window, cx, move |folder, window, cx| {
            let _ = this.update(cx, |this, cx| this.open_paths(vec![folder], window, cx));
        });
    }

    fn parent_of_repositories(&self, cx: &App) -> PathBuf {
        self.tabs
            .active()
            .and_then(Tab::repository)
            .or_else(|| self.tabs.iter().find_map(Tab::repository))
            .and_then(|repository| repository.read(cx).path().parent().map(PathBuf::from))
            .or_else(home_folder)
            .unwrap_or_default()
    }

    fn announce_waiting_connectors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let connectors = Connectors::of(cx);
        if !connectors.is_ready() {
            return;
        }
        let waiting: Vec<(ConnectorId, String, String)> = connectors
            .waiting()
            .filter(|connector| !self.announced.contains(&connector.id))
            .map(|connector| {
                let (name, version) = connector
                    .manifest
                    .as_ref()
                    .map_or_else(|| (connector.id.0.clone(), String::new()), |manifest| (manifest.name.clone(), manifest.version.clone()));
                (connector.id.clone(), name, version)
            })
            .collect();
        for (id, name, version) in waiting {
            self.announced.insert(id.clone());
            let this = cx.entity().downgrade();
            let notification = Notification::info(
                "It was copied into FastGit's connectors folder. Nothing of it runs until you approve it.",
            )
            .title(format!("{name} {version} is waiting for your approval"))
            .action(move |_, _, cx| {
                let this = this.clone();
                let id = id.clone();
                let toast = cx.entity().downgrade();
                Button::new("review-waiting-connector")
                    .primary()
                    .small()
                    .label(REVIEW)
                    .on_click(move |_, window, cx| {
                        let _ = toast.update(cx, |toast, cx| toast.dismiss(window, cx));
                        let _ = this.update(cx, |this, cx| {
                            this.open_settings(Page::Connector(id.clone()), window, cx)
                        });
                    })
            });
            window.push_notification(notification, cx);
        }
    }

    fn update_active_repository(
        &self,
        cx: &mut App,
        update: impl FnOnce(&mut Repository, &mut Context<Repository>),
    ) {
        if let Some(repository) = self.tabs.active().and_then(Tab::repository) {
            repository.update(cx, update);
        }
    }

    fn copy_hash(&self, window: &mut Window, cx: &mut App) {
        if let Some(Tab::Repo(tab)) = self.tabs.active() {
            tab.view.update(cx, |view, cx| view.copy_hash(window, cx));
        }
    }

    fn show_about(window: &mut Window, cx: &mut App) {
        window.open_alert_dialog(cx, |alert, _, _| {
            alert
                .title(format!("FastGit {}", env!("CARGO_PKG_VERSION")))
                .description(
                    "A lightweight Git client for Linux, macOS and Windows, \
                     built with GPUI and gitoxide.",
                )
        });
    }

    fn top_bar(&self, cx: &App) -> TopBar {
        let active = self.tabs.active().and_then(Tab::repository).map(|tab| tab.read(cx));
        let name = match self.tabs.active() {
            Some(Tab::Cloning { view, .. }) => Some(view.read(cx).name()),
            _ => active.map(|repository| repository.name()),
        };
        TopBar::new(self.menu_bar.clone()).repository(name, active.and_then(|repository| repository.head_label()))
    }

    fn tab_strip(&self, cx: &mut Context<Self>) -> TabStrip {
        let labels = self
            .tabs
            .iter()
            .map(|tab| match tab {
                Tab::Repo(tab) => {
                    let repository = tab.repository.read(cx);
                    let state = if repository.is_reading_history() {
                        TabState::Reading
                    } else if repository.failure().is_some() {
                        TabState::Failed
                    } else {
                        TabState::Ready
                    };
                    TabLabel {
                        name: repository.name(),
                        state,
                        icon: None,
                    }
                }
                Tab::Settings(_) => TabLabel {
                    name: SETTINGS.into(),
                    state: TabState::Ready,
                    icon: Some(IconName::Settings),
                },
                Tab::Cloning { view, .. } => {
                    let view = view.read(cx);
                    TabLabel {
                        name: view.name(),
                        state: if view.is_running() { TabState::Reading } else { TabState::Failed },
                        icon: None,
                    }
                }
            })
            .collect();
        TabStrip::new(labels, self.tabs.active_index())
            .on_select(cx.listener(|this, ix: &usize, window, cx| this.activate(*ix, window, cx)))
            .on_close(cx.listener(|this, ix: &usize, window, cx| this.close_tab(*ix, window, cx)))
            .on_open(cx.listener(|this, _: &(), window, cx| this.open_repository(window, cx)))
    }

    fn welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        Notice::new(
            WELCOME_TITLE,
            SharedString::from(WELCOME_ABOUT),
            h_flex()
                .gap_2()
                .child(
                    Button::new("open-repository")
                        .primary()
                        .label("Open repository…")
                        .on_click(cx.listener(|this, _, window, cx| this.open_repository(window, cx))),
                )
                .child(
                    Button::new("clone-repository")
                        .label("Clone repository…")
                        .on_click(cx.listener(|this, _, window, cx| this.clone_repository(window, cx))),
                ),
        )
        .into_any_element()
    }
}

fn home_folder() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

impl Render for FastGit {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.tabs.active() {
            Some(Tab::Repo(tab)) => tab.view.clone().into_any_element(),
            Some(Tab::Settings(settings)) => settings.clone().into_any_element(),
            Some(Tab::Cloning { view, .. }) => view.clone().into_any_element(),
            None => self.welcome(cx),
        };
        v_flex()
            .id("fastgit")
            .key_context(WINDOW_CONTEXT)
            .pane()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(
                cx.listener(|this, _: &OpenRepository, window, cx| {
                    this.open_repository(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &CloneRepository, window, cx| this.clone_repository(window, cx)))
            .on_action(cx.listener(|this, _: &NewRepository, window, cx| this.new_repository(window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.open_settings(Page::Accounts, window, cx)
            }))
            .on_action(cx.listener(|this, _: &CloseTab, window, cx| {
                this.close_tab(this.tabs.active_index(), window, cx)
            }))
            .on_action(cx.listener(|this, _: &NextTab, window, cx| this.cycle_tab(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousTab, window, cx| this.cycle_tab(-1, window, cx)),
            )
            .on_action(cx.listener(|this, _: &Refresh, _, cx| {
                this.update_active_repository(cx, |repository, cx| repository.refresh(cx))
            }))
            .on_action(cx.listener(|this, _: &Fetch, _, cx| {
                this.update_active_repository(cx, |repository, cx| repository.fetch(cx))
            }))
            .on_action(cx.listener(|this, _: &CopyHash, window, cx| this.copy_hash(window, cx)))
            .on_action(|_: &About, window, cx| Self::show_about(window, cx))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(self.top_bar(cx))
            .when(!self.tabs.is_empty(), |this| this.child(self.tab_strip(cx)))
            .child(div().fill_height().child(body))
    }
}
