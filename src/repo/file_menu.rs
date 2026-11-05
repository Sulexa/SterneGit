use std::path::{Path, PathBuf};

use gpui_kit::{
    App, ClipboardItem, Window,
    component::{
        WindowExt as _,
        menu::{PopupMenu, PopupMenuItem},
    },
    prelude::FluentBuilder as _,
};

use crate::ui::Handler;

pub(crate) fn history_item<T: 'static>(
    menu: PopupMenu,
    on_history: &Handler<T>,
    value: T,
) -> PopupMenu {
    let on_history = on_history.clone();
    menu.item(
        PopupMenuItem::new("File history")
            .on_click(move |_, window, cx| on_history(&value, window, cx)),
    )
}

pub(crate) fn on_disk_items(menu: PopupMenu, path: PathBuf, on_disk: bool) -> PopupMenu {
    let open = path.clone();
    let reveal = path.clone();
    menu.when(on_disk, |menu| {
        menu.item(
            PopupMenuItem::new("Open file").on_click(move |_, _, cx| cx.open_with_system(&open)),
        )
        .item(
            PopupMenuItem::new("Show in folder").on_click(move |_, _, cx| cx.reveal_path(&reveal)),
        )
    })
    .item(
        PopupMenuItem::new("Copy path").on_click(move |_, window, cx| copy_path(&path, window, cx)),
    )
}

fn copy_path(path: &Path, window: &mut Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string()));
    window.push_notification("Path copied", cx);
}
