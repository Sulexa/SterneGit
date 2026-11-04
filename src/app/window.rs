use std::{path::PathBuf, time::Duration};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, PathPromptOptions, Render, SharedString, Styled as _, Subscription, Task,
    Window,
    component::{
        ActiveTheme as _, Theme, ThemeMode, WindowExt as _,
        button::{Button, ButtonVariants as _},
        menu::AppMenuBar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use super::{
    draws_its_own_menu_bar,
    tab_strip::{TabLabel, TabState, TabStrip},
    tabs::Tabs,
    top_bar::TopBar,
};
use crate::{
    actions::{
        About, CloseTab, CopyHash, Fetch, NextTab, OpenRepository, PreviousTab, Quit, Refresh,
        WINDOW_CONTEXT,
    },
    repo::{RepoView, Repository},
    ui::{Look as _, Notice},
};

const CHECK_FOR_OUTSIDE_CHANGES_EVERY: Duration = Duration::from_secs(3);
const FETCH_IN_THE_BACKGROUND_EVERY: Duration = Duration::from_secs(5 * 60);
const FETCH_ON_RETURN_UNLESS_FETCHED_WITHIN: Duration = Duration::from_secs(60);

struct RepoTab {
    view: Entity<RepoView>,
    repository: Entity<Repository>,
    _title_follows_repository: Subscription,
}

pub(crate) struct FastGit {
    tabs: Tabs<RepoTab>,
    menu_bar: Option<Entity<AppMenuBar>>,
    _poll: Task<()>,
    _activation: Subscription,
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
        let mut this = Self {
            tabs: Tabs::default(),
            menu_bar,
            _poll: poll,
            _activation: activation,
        };
        match path {
            Some(path) => this.open_paths(vec![path], window, cx),
            None => this.update_title(window, cx),
        }
        this
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        let active = self.tabs.active_index();
        for (ix, tab) in self.tabs.iter().enumerate() {
            tab.repository.update(cx, |repository, cx| {
                if ix == active {
                    repository.check_for_changes(cx);
                }
                repository.fetch_if_older_than(FETCH_IN_THE_BACKGROUND_EVERY, cx);
            });
        }
    }

    fn catch_up(&self, cx: &mut App) {
        if let Some(tab) = self.tabs.active() {
            tab.repository.update(cx, |repository, cx| {
                repository.check_for_changes(cx);
                repository.fetch_if_older_than(FETCH_ON_RETURN_UNLESS_FETCHED_WITHIN, cx);
            });
        }
    }

    fn open_paths(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let mut last = None;
        for path in paths {
            let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let existing = self
                .tabs
                .position(|tab| tab.repository.read(cx).shows(&resolved));
            last = Some(match existing {
                Some(ix) => ix,
                None => {
                    let tab = self.new_tab(path, window, cx);
                    self.tabs.push(tab)
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
        if let Some(tab) = self.tabs.active() {
            let focus = tab.view.read(cx).focus_handle();
            focus.focus(window, cx);
        }
        self.catch_up(cx);
        self.update_title(window, cx);
        cx.notify();
    }

    fn update_title(&self, window: &mut Window, cx: &App) {
        match self.tabs.active() {
            Some(tab) => {
                window.set_window_title(&format!("{} — FastGit", tab.repository.read(cx).name()))
            }
            None => window.set_window_title("FastGit"),
        }
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

    fn update_active_repository(
        &self,
        cx: &mut App,
        update: impl FnOnce(&mut Repository, &mut Context<Repository>),
    ) {
        if let Some(tab) = self.tabs.active() {
            tab.repository.update(cx, update);
        }
    }

    fn copy_hash(&self, window: &mut Window, cx: &mut App) {
        if let Some(tab) = self.tabs.active() {
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
        let active = self.tabs.active().map(|tab| tab.repository.read(cx));
        TopBar::new(self.menu_bar.clone()).repository(
            active.map(|repository| repository.name()),
            active.and_then(|repository| repository.head_label()),
        )
    }

    fn tab_strip(&self, cx: &mut Context<Self>) -> TabStrip {
        let labels = self
            .tabs
            .iter()
            .map(|tab| {
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
            "Open a Git repository",
            SharedString::from(
                "FastGit reads history straight from the repository on disk. \
                 Each repository opens in its own tab.",
            ),
            Button::new("open-repository")
                .primary()
                .label("Open repository…")
                .on_click(cx.listener(|this, _, window, cx| this.open_repository(window, cx))),
        )
        .into_any_element()
    }
}

impl Render for FastGit {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.tabs.active() {
            Some(tab) => tab.view.clone().into_any_element(),
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
