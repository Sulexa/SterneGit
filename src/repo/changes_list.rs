use std::{path::Path, sync::Arc};

use gpui_kit::{
    AnyElement, AnyView, App, ClipboardItem, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem},
        scroll::Scrollbar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::navigation::OpenFile;
use crate::{
    git::{ChangeKind, FileChange, Scope, Side, WorkingChanges},
    ui::{self, Handler, Look as _, SectionHeader, SideUi as _, file_row, handler, thousands},
};

const SIDES_TOP_DOWN: [Side; 2] = [Side::Staged, Side::Unstaged];

pub(crate) struct DiscardQuestion {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) confirm: &'static str,
}

pub(crate) fn discard_question(change: &FileChange) -> DiscardQuestion {
    let name = change.path.rsplit('/').next().unwrap_or(&change.path);
    if change.kind == ChangeKind::Untracked {
        DiscardQuestion {
            title: format!("Delete {name}?"),
            description: format!(
                "{} isn't tracked by Git, so it can't be brought back once deleted.",
                change.path
            ),
            confirm: "Delete",
        }
    } else {
        DiscardQuestion {
            title: format!("Discard the changes to {name}?"),
            description: format!(
                "The unstaged changes in {} are thrown away. Git keeps no copy of them, so this can't be undone.",
                change.path
            ),
            confirm: "Discard",
        }
    }
}

fn discard_label(change: &FileChange) -> Option<&'static str> {
    match change.kind {
        ChangeKind::Untracked => Some("Delete file…"),
        ChangeKind::Conflicted => None,
        _ => Some("Discard changes…"),
    }
}

#[derive(Clone)]
struct RowActions {
    root: Option<Arc<Path>>,
    writing: bool,
    on_open: Option<Handler<(Side, FileChange)>>,
    on_move: Option<Handler<(Side, Scope)>>,
    on_discard: Option<Handler<FileChange>>,
}

impl RowActions {
    fn menu(&self, menu: PopupMenu, side: Side, change: &FileChange) -> PopupMenu {
        let moved = (side, Scope::Files(vec![change.clone()]));
        let menu = menu.item(self.item(side.labels().file_tooltip, &self.on_move, moved));
        let discard = discard_label(change).filter(|_| side == Side::Unstaged);
        let menu = match (discard, &self.on_discard) {
            (Some(label), Some(_)) => menu.item(self.item(label, &self.on_discard, change.clone())),
            _ => menu,
        };
        let Some(path) = self
            .root
            .as_ref()
            .map(|root| root.join(change.exact_path()))
        else {
            return menu;
        };
        let on_disk = change.kind != ChangeKind::Deleted;
        let open = path.clone();
        let reveal = path.clone();
        menu.separator()
            .when(on_disk, |menu| {
                menu.item(
                    PopupMenuItem::new("Open file")
                        .on_click(move |_, _, cx| cx.open_with_system(&open)),
                )
                .item(
                    PopupMenuItem::new("Show in folder")
                        .on_click(move |_, _, cx| cx.reveal_path(&reveal)),
                )
            })
            .item(
                PopupMenuItem::new("Copy path")
                    .on_click(move |_, window, cx| copy_path(&path, window, cx)),
            )
    }

    fn item<T: 'static>(
        &self,
        label: &'static str,
        handler: &Option<Handler<T>>,
        value: T,
    ) -> PopupMenuItem {
        let handler = handler.clone();
        PopupMenuItem::new(label)
            .disabled(self.writing || handler.is_none())
            .on_click(move |_, window, cx| {
                if let Some(handler) = &handler {
                    handler(&value, window, cx)
                }
            })
    }
}

fn copy_path(path: &Path, window: &mut Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string()));
    window.push_notification("Path copied", cx);
}

#[derive(Clone, Default)]
pub(crate) struct SideScrolls {
    unstaged: UniformListScrollHandle,
    staged: UniformListScrollHandle,
}

impl SideScrolls {
    fn of(&self, side: Side) -> &UniformListScrollHandle {
        match side {
            Side::Unstaged => &self.unstaged,
            Side::Staged => &self.staged,
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct ChangesList {
    changes: Option<Arc<WorkingChanges>>,
    form: AnyView,
    open: Option<OpenFile>,
    scrolls: SideScrolls,
    actions: RowActions,
}

impl ChangesList {
    pub(crate) fn new(
        changes: Option<Arc<WorkingChanges>>,
        form: impl Into<AnyView>,
        scrolls: SideScrolls,
    ) -> Self {
        Self {
            changes,
            form: form.into(),
            open: None,
            scrolls,
            actions: RowActions {
                root: None,
                writing: false,
                on_open: None,
                on_move: None,
                on_discard: None,
            },
        }
    }

    pub(crate) fn root(mut self, root: Option<Arc<Path>>) -> Self {
        self.actions.root = root;
        self
    }

    pub(crate) fn on_discard(
        mut self,
        callback: impl Fn(&FileChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_discard = Some(handler(callback));
        self
    }

    pub(crate) fn open(mut self, file: Option<OpenFile>) -> Self {
        self.open = file;
        self
    }

    pub(crate) fn writing(mut self, writing: bool) -> Self {
        self.actions.writing = writing;
        self
    }

    pub(crate) fn on_open(
        mut self,
        callback: impl Fn(&(Side, FileChange), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_open = Some(handler(callback));
        self
    }

    pub(crate) fn on_move(
        mut self,
        callback: impl Fn(&(Side, Scope), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_move = Some(handler(callback));
        self
    }

    fn side_section(&self, side: Side, changes: &Arc<WorkingChanges>, cx: &App) -> AnyElement {
        let count = changes.files(side).len();
        let content = if count == 0 {
            ui::hint(side.labels().empty, cx)
        } else {
            self.side_list(side, count, changes.clone())
        };
        v_flex()
            .fill_height()
            .when(side == Side::Unstaged, |this| {
                this.border_t_1().border_color(cx.theme().border)
            })
            .child(side_header(
                side,
                count,
                self.actions.writing,
                self.actions.on_move.clone(),
            ))
            .child(div().fill_height().child(content))
            .into_any_element()
    }

    fn side_list(&self, side: Side, count: usize, changes: Arc<WorkingChanges>) -> AnyElement {
        let scroll = self.scrolls.of(side).clone();
        let open = self.open.clone();
        let actions = self.actions.clone();
        div()
            .id(("changes", side as usize))
            .relative()
            .size_full()
            .child(
                uniform_list(
                    ("change-list", side as usize),
                    count,
                    move |range, _, cx| {
                        let files = changes.files(side);
                        range
                            .filter_map(|ix| files.get(ix))
                            .map(|change| {
                                let selected = open.as_ref().is_some_and(|open| {
                                    open.side == side && open.path == change.path
                                });
                                change_row(change, side, selected, &actions, cx)
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll))
            .into_any_element()
    }
}

impl RenderOnce for ChangesList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(changes) = self.changes.clone() else {
            return ui::loading("Reading changes…");
        };
        let lists = if changes.is_empty() {
            vec![
                div()
                    .fill_height()
                    .child(ui::hint("No uncommitted changes.", cx))
                    .into_any_element(),
            ]
        } else {
            SIDES_TOP_DOWN
                .iter()
                .map(|&side| self.side_section(side, &changes, cx))
                .collect()
        };
        v_flex()
            .pane()
            .child(self.form)
            .children(lists)
            .into_any_element()
    }
}

fn side_header(
    side: Side,
    count: usize,
    writing: bool,
    on_move: Option<Handler<(Side, Scope)>>,
) -> AnyElement {
    let labels = side.labels();
    SectionHeader::new(format!("{} ({})", labels.files, thousands(count)))
        .action(
            Button::new(SharedString::from(format!("all-{side:?}")))
                .ghost()
                .xsmall()
                .icon(labels.icon)
                .label(labels.all)
                .disabled(writing || count == 0)
                .when_some(on_move, |button, on_move| {
                    button.on_click(move |_, window, cx| {
                        on_move(&(side, Scope::Everything), window, cx)
                    })
                }),
        )
        .into_any_element()
}

fn change_row(
    change: &FileChange,
    side: Side,
    selected: bool,
    actions: &RowActions,
    cx: &App,
) -> AnyElement {
    let writing = actions.writing;
    let on_open = &actions.on_open;
    let on_move = &actions.on_move;
    let labels = side.labels();
    let id = format!("{side:?}/{}", change.path);
    let moved = (side, Scope::Files(vec![change.clone()]));
    let opened = (side, change.clone());
    let move_button = Button::new(SharedString::from(format!("stage/{id}")))
        .ghost()
        .xsmall()
        .icon(labels.icon)
        .tooltip(labels.file_tooltip)
        .disabled(writing)
        .when_some(on_move.clone(), |button, on_move| {
            button.on_click(move |_, window, cx| {
                cx.stop_propagation();
                on_move(&moved, window, cx)
            })
        });
    file_row(
        SharedString::from(id),
        change.kind,
        &change.path,
        selected,
        move_button,
        cx,
    )
    .when_some(on_open.clone(), |row, on_open| {
        row.on_click(move |_, window, cx| on_open(&opened, window, cx))
    })
    .context_menu({
        let actions = actions.clone();
        let change = change.clone();
        move |menu, _, _| actions.menu(menu, side, &change)
    })
    .into_any_element()
}
