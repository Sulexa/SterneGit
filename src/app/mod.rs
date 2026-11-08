pub(crate) mod connectors;
mod logo;
mod opening;
mod saved_tabs;
mod tab_strip;
mod tabs;
pub(crate) mod top_bar;
pub(crate) mod updates;
mod window;
pub(crate) mod zoom;

use std::{path::PathBuf, rc::Rc};

use gpui_kit::{
    Action, App, AppContext as _, Bounds, KeyBinding, KeyBindingContextPredicate, Menu, MenuItem,
    WindowBounds, WindowOptions,
    component::{GlobalState, TitleBar},
    px, size,
};

use crate::{
    actions::{
        About, CheckForUpdates, CloneRepository, CloseDiff, CloseTab, CopyHash, Fetch, HISTORY_CONTEXT, NewRepository,
        NewerCommit, NextChange, NextFile, NextTab, OlderCommit, OpenRepository, OpenSettings,
        PreviousChange, PreviousFile, PreviousTab, ResetZoom,
        LoopChange, MEDIA_CONTEXT, PlayPause, Quit, Refresh, SwitchVersion, WINDOW_CONTEXT, WholeWindow,
        ZoomIn, ZoomOut,
    },
    assets::Assets,
};
pub(crate) use opening::Opening;
use window::SterneGit;

const WINDOW_WIDTH: f32 = 1440.;
const WINDOW_HEIGHT: f32 = 900.;
const MIN_WINDOW_WIDTH: f32 = 960.;
const MIN_WINDOW_HEIGHT: f32 = 600.;

pub(crate) fn run(path: Option<PathBuf>, opening: Opening) {
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        bind_keys(cx);
        install_menus(cx);
        let display_id = opening.display(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                display_id,
                size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
                cx,
            ))),
            display_id,
            focus: opening.takes_focus(),
            window_min_size: Some(size(px(MIN_WINDOW_WIDTH), px(MIN_WINDOW_HEIGHT))),
            app_id: Some("sternegit".into()),
            ..TitleBar::window_options()
        };
        connectors::install_connectors(cx);
        zoom::install_zoom(cx);
        updates::install_updates(cx);
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| SterneGit::new(path, window, cx))
        })
        .expect("failed to open the SterneGit window");
        if opening.takes_focus() {
            cx.activate(true);
        }
        connectors::start_connectors(cx);
    });
}

fn draws_its_own_menu_bar(cx: &App) -> bool {
    !cx.platform_info().is_mac()
}

fn bind_keys(cx: &mut App) {
    let is_mac = cx.platform_info().is_mac();
    let primary = |mac: &'static str, other: &'static str| if is_mac { mac } else { other };
    let zoom_keys_where_they_sit_on_a_us_keyboard = zoom_keys_by_position(is_mac, cx);
    cx.bind_keys(zoom_keys_where_they_sit_on_a_us_keyboard);
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
    cx.bind_keys(menus_show_the_last_key(
        ZoomIn,
        [primary("cmd-+", "ctrl-+"), primary("cmd-=", "ctrl-=")],
        WINDOW_CONTEXT,
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
        KeyBinding::new(primary("cmd--", "ctrl--"), ZoomOut, Some(WINDOW_CONTEXT)),
        KeyBinding::new(primary("cmd-0", "ctrl-0"), ResetZoom, Some(WINDOW_CONTEXT)),
        KeyBinding::new(primary("cmd-q", "ctrl-q"), Quit, Some(WINDOW_CONTEXT)),
    ]);
}

fn zoom_keys_by_position(is_mac: bool, cx: &App) -> Vec<KeyBinding> {
    let primary = if is_mac { "cmd" } else { "ctrl" };
    let window = KeyBindingContextPredicate::parse(WINDOW_CONTEXT).ok().map(Rc::new);
    let at = |key: &str, action: Box<dyn Action>| {
        KeyBinding::load(&format!("{primary}-{key}"), action, window.clone(), true, None, cx).ok()
    };
    [
        at("=", ZoomIn.boxed_clone()),
        at("-", ZoomOut.boxed_clone()),
        at("0", ResetZoom.boxed_clone()),
    ]
    .into_iter()
    .flatten()
    .collect()
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
            MenuItem::separator(),
            MenuItem::action("Zoom In", ZoomIn),
            MenuItem::action("Zoom Out", ZoomOut),
            MenuItem::action("Reset Zoom", ResetZoom),
        ]),
        Menu::new("About").items([
            MenuItem::action("About SterneGit", About),
            MenuItem::action("Check for Updates…", CheckForUpdates),
        ]),
    ]
}
