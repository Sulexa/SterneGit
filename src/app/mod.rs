pub(crate) mod connectors;
mod saved_tabs;
mod tab_strip;
mod tabs;
mod top_bar;
mod window;

use std::path::PathBuf;

use gpui_kit::{
    Action, App, AppContext as _, KeyBinding, Menu, MenuItem, WindowBounds, WindowOptions,
    component::{GlobalState, TitleBar},
    px, size,
};

use crate::{
    actions::{
        About, CloneRepository, CloseDiff, CloseTab, CopyHash, Fetch, HISTORY_CONTEXT, NewRepository,
        NewerCommit, NextChange, NextFile, NextTab, OlderCommit, OpenRepository, OpenSettings,
        PreviousChange, PreviousFile, PreviousTab,
        LoopChange, MEDIA_CONTEXT, PlayPause, Quit, Refresh, SwitchVersion, WINDOW_CONTEXT, WholeWindow,
    },
    assets::Assets,
};
use window::SterneGit;

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
            app_id: Some("sternegit".into()),
            ..TitleBar::window_options()
        };
        connectors::install_connectors(cx);
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| SterneGit::new(path, window, cx))
        })
        .expect("failed to open the SterneGit window");
        cx.activate(true);
        connectors::start_connectors(cx);
    });
}

fn draws_its_own_menu_bar(cx: &App) -> bool {
    !cx.platform_info().is_mac()
}

fn bind_keys(cx: &mut App) {
    let is_mac = cx.platform_info().is_mac();
    let primary = |mac: &'static str, other: &'static str| if is_mac { mac } else { other };
    cx.bind_keys(menus_show_the_last_key(
        OlderCommit,
        ["j", "down"],
        HISTORY_CONTEXT,
    ));
    cx.bind_keys(menus_show_the_last_key(
        NewerCommit,
        ["k", "up"],
        HISTORY_CONTEXT,
    ));
    cx.bind_keys([
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
            primary("cmd-shift-o", "ctrl-shift-o"),
            CloneRepository,
            Some(WINDOW_CONTEXT),
        ),
        KeyBinding::new(primary("cmd-n", "ctrl-n"), NewRepository, Some(WINDOW_CONTEXT)),
        KeyBinding::new(primary("cmd-,", "ctrl-,"), OpenSettings, Some(WINDOW_CONTEXT)),
        KeyBinding::new(
            primary("cmd-shift-c", "ctrl-shift-c"),
            CopyHash,
            Some(WINDOW_CONTEXT),
        ),
        KeyBinding::new("escape", CloseDiff, Some(WINDOW_CONTEXT)),
        KeyBinding::new("alt-up", PreviousChange, Some(WINDOW_CONTEXT)),
        KeyBinding::new("alt-down", NextChange, Some(WINDOW_CONTEXT)),
        KeyBinding::new("alt-left", PreviousFile, Some(WINDOW_CONTEXT)),
        KeyBinding::new("alt-right", NextFile, Some(WINDOW_CONTEXT)),
        KeyBinding::new("space", PlayPause, Some(MEDIA_CONTEXT)),
        KeyBinding::new("tab", SwitchVersion, Some(MEDIA_CONTEXT)),
        KeyBinding::new("l", LoopChange, Some(MEDIA_CONTEXT)),
        KeyBinding::new(primary("cmd-w", "ctrl-w"), CloseTab, Some(WINDOW_CONTEXT)),
        KeyBinding::new("ctrl-tab", NextTab, Some(WINDOW_CONTEXT)),
        KeyBinding::new("ctrl-shift-tab", PreviousTab, Some(WINDOW_CONTEXT)),
        KeyBinding::new(primary("cmd-q", "ctrl-q"), Quit, Some(WINDOW_CONTEXT)),
    ]);
}

fn menus_show_the_last_key<A: Action + Clone>(
    action: A,
    keys: [&'static str; 2],
    context: &'static str,
) -> [KeyBinding; 2] {
    keys.map(|key| KeyBinding::new(key, action.clone(), Some(context)))
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
            MenuItem::action("Clone Repository…", CloneRepository),
            MenuItem::action("New Repository…", NewRepository),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("Close Tab", CloseTab),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ]),
        Menu::new("Edit").items([MenuItem::action("Copy Commit Hash", CopyHash)]),
        Menu::new("View").items([
            MenuItem::action("Refresh", Refresh),
            MenuItem::action("Fetch All Remotes", Fetch),
            MenuItem::action("Close File Diff", CloseDiff),
            MenuItem::action("Previous Change", PreviousChange),
            MenuItem::action("Next Change", NextChange),
            MenuItem::action("Previous File", PreviousFile),
            MenuItem::action("Next File", NextFile),
            MenuItem::action("Use the Whole Window", WholeWindow),
            MenuItem::separator(),
            MenuItem::action("Newer Commit", NewerCommit),
            MenuItem::action("Older Commit", OlderCommit),
            MenuItem::separator(),
            MenuItem::action("Next Tab", NextTab),
            MenuItem::action("Previous Tab", PreviousTab),
        ]),
        Menu::new("About").items([MenuItem::action("About SterneGit", About)]),
    ]
}
