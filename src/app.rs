//! The FastGit window: a top bar with the File / Edit / View / About menus and
//! the active repository, a tab per open repository below it, and the active
//! repository's view.

use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, GlobalState, Icon, Sizable as _, StyledExt as _, Theme, ThemeMode,
    TitleBar, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::AppMenuBar,
    spinner::Spinner,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    KeyBinding, Menu, MenuItem, ParentElement as _, PathPromptOptions, Render, SharedString, Styled as _,
    Subscription, Window, div, prelude::FluentBuilder as _, px,
};

use crate::repo_view::RepoView;

mod commands {
    gpui_kit::actions!(
        fastgit,
        [
            OlderCommit,
            NewerCommit,
            Refresh,
            OpenRepository,
            CopyHash,
            CloseTab,
            NextTab,
            PreviousTab,
            About,
            Quit
        ]
    );
}
pub(crate) use commands::*;

pub(crate) fn init(cx: &mut App) {
    let is_mac = cx.platform_info().is_mac();
    let primary = |mac: &'static str, other: &'static str| if is_mac { mac } else { other };
    cx.bind_keys([
        KeyBinding::new("down", OlderCommit, Some("History")),
        KeyBinding::new("j", OlderCommit, Some("History")),
        KeyBinding::new("up", NewerCommit, Some("History")),
        KeyBinding::new("k", NewerCommit, Some("History")),
        KeyBinding::new(primary("cmd-r", "ctrl-r"), Refresh, Some("FastGit")),
        KeyBinding::new(primary("cmd-o", "ctrl-o"), OpenRepository, Some("FastGit")),
        KeyBinding::new(
            primary("cmd-shift-c", "ctrl-shift-c"),
            CopyHash,
            Some("FastGit"),
        ),
        KeyBinding::new(primary("cmd-w", "ctrl-w"), CloseTab, Some("FastGit")),
        KeyBinding::new("ctrl-tab", NextTab, Some("FastGit")),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, Some("FastGit")),
        KeyBinding::new(primary("cmd-q", "ctrl-q"), Quit, Some("FastGit")),
    ]);

    let menus = vec![
        Menu::new("File").items([
            MenuItem::action("Open Repository…", OpenRepository),
            MenuItem::action("Close Tab", CloseTab),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ]),
        Menu::new("Edit").items([MenuItem::action("Copy Commit Hash", CopyHash)]),
        Menu::new("View").items([
            MenuItem::action("Refresh", Refresh),
            MenuItem::separator(),
            MenuItem::action("Newer Commit", NewerCommit),
            MenuItem::action("Older Commit", OlderCommit),
            MenuItem::separator(),
            MenuItem::action("Next Tab", NextTab),
            MenuItem::action("Previous Tab", PreviousTab),
        ]),
        Menu::new("About").items([MenuItem::action("About FastGit", About)]),
    ];
    if is_mac {
        // macOS shows them in its own menu bar, so the window draws none.
        cx.set_menus(menus);
    } else {
        GlobalState::global_mut(cx).set_app_menus(menus.into_iter().map(Menu::owned).collect());
    }
}

struct RepoTab {
    view: Entity<RepoView>,
    _observe: Subscription,
}

pub(crate) struct FastGit {
    tabs: Vec<RepoTab>,
    active: usize,
    /// The in-window menus, on Windows and Linux.
    menu_bar: Option<Entity<AppMenuBar>>,
}

impl FastGit {
    pub(crate) fn new(path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Theme::change(ThemeMode::Dark, Some(window), cx);
        let menu_bar = (!cx.platform_info().is_mac()).then(|| AppMenuBar::new(cx));
        let mut this = Self {
            tabs: Vec::new(),
            active: 0,
            menu_bar,
        };
        match path {
            Some(path) => this.open_paths(vec![path], window, cx),
            None => this.update_title(window, cx),
        }
        this
    }

    fn active_view(&self) -> Option<&Entity<RepoView>> {
        self.tabs.get(self.active).map(|tab| &tab.view)
    }

    /// Opens each path in a tab of its own, or switches to the tab already
    /// showing its repository, and leaves the last one active.
    fn open_paths(&mut self, paths: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let mut last = None;
        for path in paths {
            let resolved = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let existing = self
                .tabs
                .iter()
                .position(|tab| tab.view.read(cx).shows(&resolved));
            last = Some(match existing {
                Some(ix) => ix,
                None => {
                    let view = cx.new(|cx| RepoView::new(path, cx));
                    // The tab label and the top bar show what the view has read.
                    let observe = cx.observe_in(&view, window, |this, _, window, cx| {
                        this.update_title(window, cx);
                        cx.notify();
                    });
                    self.tabs.push(RepoTab {
                        view,
                        _observe: observe,
                    });
                    self.tabs.len() - 1
                }
            });
        }
        if let Some(ix) = last {
            self.activate(ix, window, cx);
        }
    }

    fn activate(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.tabs.len() {
            return;
        }
        self.active = ix;
        let focus = self.tabs[ix].view.read(cx).focus_handle();
        focus.focus(window, cx);
        self.update_title(window, cx);
        cx.notify();
    }

    fn close_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix >= self.tabs.len() {
            return;
        }
        self.tabs.remove(ix);
        if self.tabs.is_empty() {
            self.active = 0;
            self.update_title(window, cx);
            cx.notify();
            return;
        }
        // Closing a tab left of the active one keeps the same repository active.
        let active = if ix < self.active || self.active >= self.tabs.len() {
            self.active.saturating_sub(1)
        } else {
            self.active
        };
        self.activate(active, window, cx);
    }

    fn cycle_tab(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.tabs.len();
        if count > 1 {
            let ix = (self.active as isize + delta).rem_euclid(count as isize) as usize;
            self.activate(ix, window, cx);
        }
    }

    fn update_title(&self, window: &mut Window, cx: &App) {
        match self.active_view() {
            Some(view) => window.set_window_title(&format!("{} — FastGit", view.read(cx).name())),
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

    fn refresh(&self, cx: &mut App) {
        if let Some(view) = self.active_view() {
            view.update(cx, |view, cx| view.refresh(cx));
        }
    }

    fn copy_hash(&self, window: &mut Window, cx: &mut App) {
        if let Some(view) = self.active_view() {
            view.update(cx, |view, cx| view.copy_hash(window, cx));
        }
    }

    fn show_about(window: &mut Window, cx: &mut App) {
        window.open_alert_dialog(cx, |alert, _, _| {
            alert
                .title(format!("FastGit {}", env!("CARGO_PKG_VERSION")))
                .description(
                    "A lightweight Git client for Linux, macOS and Windows,                      built with GPUI and gitoxide.",
                )
        });
    }

    fn render_top_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, border, primary) = (theme.muted_foreground, theme.border, theme.primary);
        let active = self.active_view().map(|view| view.read(cx));
        let name = active.map(|view| view.name());
        let head = active.and_then(|view| view.head());
        TitleBar::new().child(
            h_flex()
                .size_full()
                .px_1()
                .pr_3()
                .gap_2()
                .child(Icon::new(IconName::GitMerge).small().text_color(primary))
                .when_some(self.menu_bar.clone(), |this, menu_bar| {
                    this.child(div().flex_none().h_full().child(menu_bar))
                })
                .child(div().flex_1())
                .when_some(name, |this, name| {
                    this.child(
                        h_flex()
                            .min_w_0()
                            .gap_2()
                            .child(div().text_sm().font_medium().truncate().child(name))
                            .when_some(head, |this, head| {
                                this.child(
                                    h_flex()
                                        .min_w_0()
                                        .gap_1()
                                        .px_1p5()
                                        .py_0p5()
                                        .rounded(px(4.))
                                        .border_1()
                                        .border_color(border)
                                        .text_xs()
                                        .text_color(muted)
                                        .child(Icon::new(IconName::GitBranch).xsmall())
                                        .child(div().truncate().child(head)),
                                )
                            }),
                    )
                }),
        )
    }

    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        let tabs = self.tabs.iter().enumerate().map(|(ix, tab)| {
            let view = tab.view.read(cx);
            let owner = owner.clone();
            Tab::new()
                .min_w(px(120.))
                .max_w(px(240.))
                .map(|this| {
                    if view.is_loading() {
                        this.prefix(div().pl_3().child(Spinner::new().xsmall()))
                    } else if view.has_failed() {
                        this.prefix(
                            div().pl_3().child(
                                Icon::new(IconName::TriangleAlert)
                                    .xsmall()
                                    .text_color(cx.theme().red),
                            ),
                        )
                    } else {
                        this
                    }
                })
                .child(div().truncate().child(view.name()))
                .suffix(
                    Button::new(("close-tab", ix))
                        .ghost()
                        .icon(IconName::Close)
                        .xsmall()
                        .ml(-px(8.))
                        .mr_2()
                        .tab_stop(false)
                        .tooltip_with_action("Close tab", &CloseTab, Some("FastGit"))
                        .on_click(move |_, window, cx| {
                            // The click closes the tab without also selecting it.
                            cx.stop_propagation();
                            let _ = owner.update(cx, |this, cx| this.close_tab(ix, window, cx));
                        }),
                )
        });
        let select = owner.clone();
        TabBar::new("repo-tabs")
            .children(tabs.collect::<Vec<_>>())
            .selected_index(self.active)
            .on_click(move |ix, window, cx| {
                let _ = select.update(cx, |this, cx| this.activate(*ix, window, cx));
            })
            .suffix(
                h_flex().h_full().px_1().child(
                    Button::new("new-tab")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Plus)
                        .tooltip_with_action(
                            "Open a repository in a new tab",
                            &OpenRepository,
                            Some("FastGit"),
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_repository(window, cx)),
                        ),
                ),
            )
    }

    fn render_welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(Icon::new(IconName::GitMerge).large().text_color(muted))
            .child(div().text_lg().font_medium().child("Open a Git repository"))
            .child(
                div()
                    .max_w(px(520.))
                    .text_sm()
                    .text_center()
                    .text_color(muted)
                    .child(SharedString::from(
                        "FastGit reads history straight from the repository on disk. \
                         Each repository opens in its own tab.",
                    )),
            )
            .child(
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
        let body = match self.active_view() {
            Some(view) => view.clone().into_any_element(),
            None => self.render_welcome(cx),
        };
        v_flex()
            .id("fastgit")
            .key_context("FastGit")
            .size_full()
            .min_w_0()
            .min_h_0()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_action(
                cx.listener(|this, _: &OpenRepository, window, cx| {
                    this.open_repository(window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &CloseTab, window, cx| {
                    this.close_tab(this.active, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &NextTab, window, cx| this.cycle_tab(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousTab, window, cx| this.cycle_tab(-1, window, cx)),
            )
            // The active view handles these first when it has focus.
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_action(cx.listener(|this, _: &CopyHash, window, cx| this.copy_hash(window, cx)))
            .on_action(|_: &About, window, cx| Self::show_about(window, cx))
            .on_action(|_: &Quit, _, cx| cx.quit())
            .child(self.render_top_bar(cx))
            .when(!self.tabs.is_empty(), |this| {
                this.child(self.render_tabs(cx))
            })
            .child(div().flex_1().min_h_0().child(body))
    }
}
