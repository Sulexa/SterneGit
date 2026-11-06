use std::path::PathBuf;

use gpui_kit::{
    App, ClipboardItem, Window,
    component::{
        WindowExt as _,
        menu::{PopupMenu, PopupMenuItem},
    },
    prelude::FluentBuilder as _,
};

use crate::ui::{Handler, thousands};

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
    .item(PopupMenuItem::new("Copy path").on_click(move |_, window, cx| {
        copy_paths(std::slice::from_ref(&path), window, cx)
    }))
}

pub(crate) fn many_on_disk_items(menu: PopupMenu, paths: Vec<PathBuf>, on_disk: bool) -> PopupMenu {
    let count = thousands(paths.len());
    let open = paths.clone();
    menu.when(on_disk, |menu| {
        menu.item(
            PopupMenuItem::new(format!("Open {count} files")).on_click(move |_, _, cx| {
                for path in &open {
                    cx.open_with_system(path)
                }
            }),
        )
    })
    .item(
        PopupMenuItem::new(format!("Copy {count} paths"))
            .on_click(move |_, window, cx| copy_paths(&paths, window, cx)),
    )
}

fn copy_paths(paths: &[PathBuf], window: &mut Window, cx: &mut App) {
    let lines = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    cx.write_to_clipboard(ClipboardItem::new_string(lines.join("\n")));
    let copied = match paths {
        [_] => "Path copied".to_owned(),
        _ => format!("{} paths copied", thousands(paths.len())),
    };
    window.push_notification(copied, cx);
}
