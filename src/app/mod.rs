mod tab_strip;
mod tabs;
mod top_bar;
mod window;

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, KeyBinding, Menu, MenuItem, WindowBounds, WindowOptions,
    component::{GlobalState, TitleBar},
    px, size,
};

use crate::{
    actions::{
        About, CloseDiff, CloseTab, CopyHash, Fetch, HISTORY_CONTEXT, NewerCommit, NextTab,
        OlderCommit, OpenRepository, PreviousTab, Quit, Refresh, WINDOW_CONTEXT,
    },
    assets::Assets,
};
use window::FastGit;

const WINDOW_WIDTH: f32 = 1440.;
const WINDOW_HEIGHT: f32 = 900.;
const MIN_WINDOW_WIDTH: f32 = 960.;
const MIN_WINDOW_HEIGHT: f32 = 600.;

pub(crate) fn run(path: Option<PathBuf>) {
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        bind_keys(cx);
        install_menus(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(
                size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
                cx,
            )),
            window_min_size: Some(size(px(MIN_WINDOW_WIDTH), px(MIN_WINDOW_HEIGHT))),
            app_id: Some("fastgit".into()),
            ..TitleBar::window_options()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| FastGit::new(path, window, cx))
        })
        .expect("failed to open the FastGit window");
        cx.activate(true);
    });
}

fn draws_its_own_menu_bar(cx: &App) -> bool {
    !cx.platform_info().is_mac()
}

fn bind_keys(cx: &mut App) {
    let is_mac = cx.platform_info().is_mac();
    let primary = |mac: &'static str, other: &'static str| if is_mac { mac } else { other };
    cx.bind_keys([
        KeyBinding::new("down", OlderCommit, Some(HISTORY_CONTEXT)),
        KeyBinding::new("j", OlderCommit, Some(HISTORY_CONTEXT)),
        KeyBinding::new("up", NewerCommit, Some(HISTORY_CONTEXT)),
        KeyBinding::new("k", NewerCommit, Some(HISTORY_CONTEXT)),
        KeyBinding::new(primary("cmd-r", "ctrl-r"), Refresh, Some(WINDOW_CONTEXT)),
        KeyBinding::new(
            primary("cmd-shift-r", "ctrl-shift-r"),
            Fetch,
            Some(WINDOW_CONTEXT),
        ),
        KeyBinding::new(
            primary("cmd-o", "ctrl-o"),
            OpenRepository,
            Some(WINDOW_CONTEXT),
        ),
        KeyBinding::new(
            primary("cmd-shift-c", "ctrl-shift-c"),
            CopyHash,
            Some(WINDOW_CONTEXT),
        ),
        KeyBinding::new("escape", CloseDiff, Some(WINDOW_CONTEXT)),
        KeyBinding::new(primary("cmd-w", "ctrl-w"), CloseTab, Some(WINDOW_CONTEXT)),
        KeyBinding::new("ctrl-tab", NextTab, Some(WINDOW_CONTEXT)),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, Some(WINDOW_CONTEXT)),
        KeyBinding::new(primary("cmd-q", "ctrl-q"), Quit, Some(WINDOW_CONTEXT)),
    ]);
}

fn install_menus(cx: &mut App) {
    let menus = menus();
    if draws_its_own_menu_bar(cx) {
        GlobalState::global_mut(cx).set_app_menus(menus.into_iter().map(Menu::owned).collect());
    } else {
        cx.set_menus(menus);
    }
}

fn menus() -> Vec<Menu> {
    vec![
        Menu::new("File").items([
            MenuItem::action("Open Repository…", OpenRepository),
            MenuItem::action("Close Tab", CloseTab),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ]),
        Menu::new("Edit").items([MenuItem::action("Copy Commit Hash", CopyHash)]),
        Menu::new("View").items([
            MenuItem::action("Refresh", Refresh),
            MenuItem::action("Fetch All Remotes", Fetch),
            MenuItem::action("Close File Diff", CloseDiff),
            MenuItem::separator(),
            MenuItem::action("Newer Commit", NewerCommit),
            MenuItem::action("Older Commit", OlderCommit),
            MenuItem::separator(),
            MenuItem::action("Next Tab", NextTab),
            MenuItem::action("Previous Tab", PreviousTab),
        ]),
        Menu::new("About").items([MenuItem::action("About FastGit", About)]),
    ]
}
