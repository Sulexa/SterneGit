use std::{path::Path, rc::Rc, sync::Arc};

use gpui_kit::{
    AnyElement, AnyView, App, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem},
        scroll::Scrollbar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::{
    file_menu::{history_item, on_disk_items},
    file_tree::{FileTreeRow, file_rows, is_inside},
    navigation::OpenFile,
};
use crate::{
    git::{ChangeKind, FileChange, Scope, Side, WorkingChanges},
    ui::{
        self, BySide, FilePlace, FileView, FileViewToggle, Folded, Handler, Look as _,
        SectionHeader, SideUi as _, counted, file_row, folder_row, handler, thousands,
    },
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

fn has_history(change: &FileChange) -> bool {
    !matches!(change.kind, ChangeKind::Added | ChangeKind::Untracked)
}

#[derive(Clone)]
struct RowActions {
    root: Option<Arc<Path>>,
    writing: bool,
    on_open: Option<Handler<(Side, FileChange)>>,
    on_move: Option<Handler<(Side, Scope)>>,
    on_discard: Option<Handler<FileChange>>,
    on_history: Option<Handler<FileChange>>,
    on_fold: Option<Handler<(Side, SharedString)>>,
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
        let menu = match &self.on_history {
            Some(on_history) if has_history(change) => {
                history_item(menu.separator(), on_history, change.clone())
            }
            _ => menu,
        };
        match &self.root {
            Some(root) => on_disk_items(
                menu.separator(),
                root.join(change.exact_path()),
                change.kind != ChangeKind::Deleted,
            ),
            None => menu,
        }
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

pub(crate) type SideScrolls = BySide<UniformListScrollHandle>;

pub(crate) type SideFolds = BySide<Folded<SharedString>>;

#[derive(IntoElement)]
pub(crate) struct ChangesList {
    changes: Option<Arc<WorkingChanges>>,
    rows: BySide<Rc<[FileTreeRow]>>,
    form: AnyView,
    open: Option<OpenFile>,
    scrolls: SideScrolls,
    actions: RowActions,
}

impl ChangesList {
    pub(crate) fn new(
        changes: Option<Arc<WorkingChanges>>,
        form: impl Into<AnyView>,
        view: FileView,
        folded: &SideFolds,
        scrolls: SideScrolls,
    ) -> Self {
        let rows = BySide::new(|side| {
            let paths = changes
                .as_deref()
                .map_or(&[][..], |changes| changes.files(side))
                .iter()
                .map(|change| change.path.as_ref());
            file_rows(view, paths, folded.of(side)).into()
        });
        Self {
            changes,
            rows,
            form: form.into(),
            open: None,
            scrolls,
            actions: RowActions {
                root: None,
                writing: false,
                on_open: None,
                on_move: None,
                on_discard: None,
                on_history: None,
                on_fold: None,
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

    pub(crate) fn on_history(
        mut self,
        callback: impl Fn(&FileChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_history = Some(handler(callback));
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

    pub(crate) fn on_fold(
        mut self,
        callback: impl Fn(&(Side, SharedString), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_fold = Some(handler(callback));
        self
    }

    fn side_section(&self, side: Side, changes: &Arc<WorkingChanges>, cx: &App) -> AnyElement {
        let count = changes.files(side).len();
        let content = if count == 0 {
            ui::hint(side.labels().empty, cx)
        } else {
            self.side_list(side, changes.clone())
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

    fn side_list(&self, side: Side, changes: Arc<WorkingChanges>) -> AnyElement {
        let scroll = self.scrolls.of(side).clone();
        let rows = self.rows.of(side).clone();
        let open = self.open.clone();
        let actions = self.actions.clone();
        div()
            .id(("changes", side as usize))
            .relative()
            .size_full()
            .child(
                uniform_list(
                    ("change-list", side as usize),
                    rows.len(),
                    move |range, _, cx| {
                        let files = changes.files(side);
                        rows[range]
                            .iter()
                            .filter_map(|row| match row {
                                FileTreeRow::Folder {
                                    path,
                                    name,
                                    depth,
                                    open,
                                } => Some(change_folder_row(
                                    side,
                                    Folder {
                                        path,
                                        name,
                                        depth: *depth,
                                        open: *open,
                                    },
                                    &changes,
                                    &actions,
                                    cx,
                                )),
                                &FileTreeRow::File { ix, place } => {
                                    let change = files.get(ix)?;
                                    let selected = open.as_ref().is_some_and(|open| {
                                        open.side == side && open.path == change.path
                                    });
                                    Some(change_row(change, side, place, selected, &actions, cx))
                                }
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
            std::iter::once(files_bar(&changes, cx))
                .chain(
                    SIDES_TOP_DOWN
                        .iter()
                        .map(|&side| self.side_section(side, &changes, cx)),
                )
                .collect()
        };
        v_flex()
            .pane()
            .child(self.form)
            .children(lists)
            .into_any_element()
    }
}

fn files_bar(changes: &WorkingChanges, cx: &App) -> AnyElement {
    div()
        .flex_none()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            SectionHeader::new(counted(
                changes.file_count(),
                "changed file",
                "changed files",
            ))
            .action(FileViewToggle),
        )
        .into_any_element()
}

struct Folder<'a> {
    path: &'a SharedString,
    name: &'a str,
    depth: usize,
    open: bool,
}

fn change_folder_row(
    side: Side,
    folder: Folder,
    changes: &Arc<WorkingChanges>,
    actions: &RowActions,
    cx: &App,
) -> AnyElement {
    let labels = side.labels();
    let id = format!("{side:?}/{}/", folder.path);
    let move_button = Button::new(SharedString::from(format!("stage/{id}")))
        .ghost()
        .xsmall()
        .icon(labels.icon)
        .tooltip(labels.folder_tooltip)
        .disabled(actions.writing)
        .when_some(actions.on_move.clone(), |button, on_move| {
            let changes = changes.clone();
            let path = folder.path.clone();
            button.on_click(move |_, window, cx| {
                cx.stop_propagation();
                let inside = changes
                    .files(side)
                    .iter()
                    .filter(|change| is_inside(&change.path, &path))
                    .cloned()
                    .collect();
                on_move(&(side, Scope::Files(inside)), window, cx)
            })
        });
    let folded = (side, folder.path.clone());
    folder_row(
        SharedString::from(id),
        folder.name,
        folder.depth,
        folder.open,
        Some(move_button.into_any_element()),
        cx,
    )
    .when_some(actions.on_fold.clone(), |row, on_fold| {
        row.on_click(move |_, window, cx| on_fold(&folded, window, cx))
    })
    .into_any_element()
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
    place: FilePlace,
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
        place,
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
