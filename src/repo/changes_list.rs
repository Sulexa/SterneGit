use std::{path::Path, rc::Rc, sync::Arc};

use gpui_kit::{
    AnyElement, AnyView, App, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _,
    UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem},
        scroll::Scrollbar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    relative, uniform_list,
};

use super::{
    file_menu::{history_item, many_on_disk_items, on_disk_items},
    file_tree::{FileTreeRow, file_rows, is_inside},
    navigation::OpenFile,
};
use crate::{
    git::{ChangeKind, FileChange, Scope, Side, WorkingChanges},
    ui::{
        self, BySide, FilePlace, FileView, FileViewToggle, Folded, Handler, Look as _, Pick,
        Picked, SectionHeader, SideUi as _, counted, file_row, folder_row, handler, thousands,
    },
};

const SIDES_TOP_DOWN: [Side; 2] = [Side::Staged, Side::Unstaged];
const CONFLICTS_HEIGHT: f32 = 0.4;

pub(crate) struct DiscardQuestion {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) confirm: &'static str,
}

pub(crate) fn discard_question(changes: &[FileChange]) -> DiscardQuestion {
    match changes {
        [change] => file_question(change),
        _ => files_question(changes),
    }
}

fn file_question(change: &FileChange) -> DiscardQuestion {
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

fn files_question(changes: &[FileChange]) -> DiscardQuestion {
    let count = thousands(changes.len());
    let untracked = untracked_count(changes);
    let tracked = changes.len() - untracked;
    if tracked == 0 {
        return DiscardQuestion {
            title: format!("Delete {count} files?"),
            description: format!(
                "These {count} files aren't tracked by Git, so they can't be brought back once deleted."
            ),
            confirm: "Delete",
        };
    }
    let thrown_away = if untracked == 0 {
        format!("The unstaged changes in these {count} files are thrown away.")
    } else {
        format!(
            "The unstaged changes in {} are thrown away and {} deleted.",
            counted(tracked, "file", "files"),
            counted(untracked, "untracked file is", "untracked files are"),
        )
    };
    DiscardQuestion {
        title: format!("Discard the changes to {count} files?"),
        description: format!("{thrown_away} Git keeps no copy of them, so this can't be undone."),
        confirm: "Discard",
    }
}

fn untracked_count(changes: &[FileChange]) -> usize {
    changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Untracked)
        .count()
}

fn discard_files_label(changes: &[FileChange]) -> String {
    let count = thousands(changes.len());
    if untracked_count(changes) == changes.len() {
        format!("Delete {count} files…")
    } else {
        format!("Discard {count} files…")
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
struct SideRows {
    side: Side,
    rows: Rc<[FileTreeRow]>,
    changes: Arc<WorkingChanges>,
}

impl SideRows {
    fn key(&self, change: &FileChange) -> OpenFile {
        OpenFile {
            side: self.side,
            path: change.path.clone(),
        }
    }

    fn shown(&self) -> impl Iterator<Item = &FileChange> {
        let files = self.changes.files(self.side);
        self.rows.iter().filter_map(move |row| match row {
            &FileTreeRow::File { ix, .. } => files.get(ix),
            FileTreeRow::Folder { .. } => None,
        })
    }

    fn keys(&self) -> Vec<OpenFile> {
        self.shown().map(|change| self.key(change)).collect()
    }

    fn picked(&self, picked: &Picked<OpenFile>) -> Vec<FileChange> {
        self.shown()
            .filter(|change| picked.contains(&self.key(change)))
            .cloned()
            .collect()
    }
}

#[derive(Clone)]
struct RowActions {
    root: Option<Arc<Path>>,
    writing: bool,
    picked: Rc<Picked<OpenFile>>,
    on_open: Option<Handler<(Side, FileChange)>>,
    on_pick: Option<Handler<Picked<OpenFile>>>,
    on_move: Option<Handler<(Side, Scope)>>,
    on_discard: Option<Handler<Vec<FileChange>>>,
    on_history: Option<Handler<FileChange>>,
    on_fold: Option<Handler<(Side, SharedString)>>,
}

impl RowActions {
    fn click(
        &self,
        rows: &SideRows,
        change: &FileChange,
        pick: Pick,
        window: &mut Window,
        cx: &mut App,
    ) {
        if pick == Pick::Only {
            if let Some(on_open) = &self.on_open {
                on_open(&(rows.side, change.clone()), window, cx)
            }
        } else if let Some(on_pick) = &self.on_pick {
            let picked = self.picked.after(rows.key(change), pick, &rows.keys());
            on_pick(&picked, window, cx)
        }
    }

    fn point(&self, rows: &SideRows, change: &FileChange, window: &mut Window, cx: &mut App) {
        let key = rows.key(change);
        if let Some(on_pick) = &self.on_pick
            && !self.picked.contains(&key)
        {
            on_pick(&Picked::only(key), window, cx)
        }
    }

    fn menu(&self, menu: PopupMenu, rows: &SideRows, change: &FileChange) -> PopupMenu {
        let picked = if self.picked.contains(&rows.key(change)) {
            rows.picked(&self.picked)
        } else {
            Vec::new()
        };
        if picked.len() > 1 {
            self.files_menu(menu, rows.side, picked)
        } else {
            self.file_menu(menu, rows.side, change)
        }
    }

    fn files_menu(&self, menu: PopupMenu, side: Side, changes: Vec<FileChange>) -> PopupMenu {
        let count = thousands(changes.len());
        let moved = (side, Scope::Files(changes.clone()));
        let move_label = format!("{} {count} files", side.labels().verb);
        let menu = menu.item(self.item(move_label, &self.on_move, moved));
        let menu = match (side, &self.on_discard) {
            (Side::Unstaged, Some(_)) => menu.item(self.item(
                discard_files_label(&changes),
                &self.on_discard,
                changes.clone(),
            )),
            _ => menu,
        };
        match &self.root {
            Some(root) => many_on_disk_items(
                menu.separator(),
                changes
                    .iter()
                    .map(|change| root.join(change.exact_path()))
                    .collect(),
                changes.iter().all(|change| change.kind != ChangeKind::Deleted),
            ),
            None => menu,
        }
    }

    fn file_menu(&self, menu: PopupMenu, side: Side, change: &FileChange) -> PopupMenu {
        let moved = (side, Scope::Files(vec![change.clone()]));
        let menu = menu.item(self.item(side.labels().file_tooltip, &self.on_move, moved));
        let discard = discard_label(change).filter(|_| side == Side::Unstaged);
        let menu = match (discard, &self.on_discard) {
            (Some(label), Some(_)) => {
                menu.item(self.item(label, &self.on_discard, vec![change.clone()]))
            }
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

    fn conflict_menu(&self, menu: PopupMenu, change: &FileChange) -> PopupMenu {
        let menu = match &self.on_history {
            Some(on_history) => history_item(menu, on_history, change.clone()).separator(),
            None => menu,
        };
        match &self.root {
            Some(root) => on_disk_items(menu, root.join(change.exact_path()), true),
            None => menu,
        }
    }

    fn item<T: 'static>(
        &self,
        label: impl Into<SharedString>,
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
    form_hidden: bool,
    header: Option<AnyElement>,
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
            form_hidden: false,
            header: None,
            open: None,
            scrolls,
            actions: RowActions {
                root: None,
                writing: false,
                picked: Rc::default(),
                on_open: None,
                on_pick: None,
                on_move: None,
                on_discard: None,
                on_history: None,
                on_fold: None,
            },
        }
    }

    pub(crate) fn header(mut self, header: Option<AnyElement>) -> Self {
        self.header = header;
        self
    }

    pub(crate) fn root(mut self, root: Option<Arc<Path>>) -> Self {
        self.actions.root = root;
        self
    }

    pub(crate) fn picked(mut self, picked: Rc<Picked<OpenFile>>) -> Self {
        self.actions.picked = picked;
        self
    }

    pub(crate) fn on_pick(
        mut self,
        callback: impl Fn(&Picked<OpenFile>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_pick = Some(handler(callback));
        self
    }

    pub(crate) fn on_discard(
        mut self,
        callback: impl Fn(&Vec<FileChange>, &mut Window, &mut App) + 'static,
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

    pub(crate) fn form_hidden(mut self, hidden: bool) -> Self {
        self.form_hidden = hidden;
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
                everything_on(side, changes),
                self.actions.writing,
                self.actions.on_move.clone(),
            ))
            .child(div().fill_height().child(content))
            .into_any_element()
    }

    fn conflicts_section(&self, changes: &WorkingChanges, cx: &App) -> AnyElement {
        let rows = changes.conflicted.iter().map(|change| {
            let selected = self
                .open
                .as_ref()
                .is_some_and(|open| open.side == Side::Unstaged && open.path == change.path);
            let opened = (Side::Unstaged, change.clone());
            file_row(
                SharedString::from(format!("conflicted/{}", change.path)),
                change.kind,
                &change.path,
                FilePlace::Listed,
                selected,
                div(),
                cx,
            )
            .when_some(self.actions.on_open.clone(), |row, on_open| {
                row.on_click(move |_, window, cx| on_open(&opened, window, cx))
            })
            .context_menu({
                let actions = self.actions.clone();
                let change = change.clone();
                move |menu, _, _| actions.conflict_menu(menu, &change)
            })
        });
        v_flex()
            .flex_none()
            .max_h(relative(CONFLICTS_HEIGHT))
            .child(
                SectionHeader::new(format!("Conflicts ({})", thousands(changes.conflicted.len())))
                    .color(cx.theme().magenta),
            )
            .child(
                v_flex()
                    .id("conflicted-files")
                    .overflow_y_scroll()
                    .children(rows),
            )
            .into_any_element()
    }

    fn side_list(&self, side: Side, changes: Arc<WorkingChanges>) -> AnyElement {
        let scroll = self.scrolls.of(side).clone();
        let shown = SideRows {
            side,
            rows: self.rows.of(side).clone(),
            changes,
        };
        let actions = Rc::new(self.actions.clone());
        div()
            .id(("changes", side as usize))
            .relative()
            .size_full()
            .child(
                uniform_list(
                    ("change-list", side as usize),
                    shown.rows.len(),
                    move |range, _, cx| {
                        let files = shown.changes.files(side);
                        shown.rows[range]
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
                                    &shown.changes,
                                    &actions,
                                    cx,
                                )),
                                &FileTreeRow::File { ix, place } => {
                                    let change = files.get(ix)?;
                                    Some(change_row(change, place, &shown, &actions, cx))
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
                    (!changes.conflicted.is_empty()).then(|| self.conflicts_section(&changes, cx)),
                )
                .chain(
                    SIDES_TOP_DOWN
                        .iter()
                        .map(|&side| self.side_section(side, &changes, cx)),
                )
                .collect()
        };
        v_flex()
            .pane()
            .children(self.header)
            .when(!self.form_hidden, |this| this.child(self.form))
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

fn everything_on(side: Side, changes: &WorkingChanges) -> Scope {
    match side {
        Side::Unstaged if !changes.conflicted.is_empty() => {
            Scope::Files(changes.unstaged.clone())
        }
        _ => Scope::Everything,
    }
}

fn side_header(
    side: Side,
    count: usize,
    everything: Scope,
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
                        on_move(&(side, everything.clone()), window, cx)
                    })
                }),
        )
        .into_any_element()
}

fn change_row(
    change: &FileChange,
    place: FilePlace,
    rows: &SideRows,
    actions: &Rc<RowActions>,
    cx: &App,
) -> AnyElement {
    let side = rows.side;
    let writing = actions.writing;
    let on_move = &actions.on_move;
    let labels = side.labels();
    let id = format!("{side:?}/{}", change.path);
    let moved = (side, Scope::Files(vec![change.clone()]));
    let selected = actions.picked.contains(&rows.key(change));
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
    .on_click({
        let (actions, rows, change) = (actions.clone(), rows.clone(), change.clone());
        move |event, window, cx| {
            let modifiers = event.modifiers();
            let pick = Pick::new(modifiers.secondary(cx), modifiers.shift);
            actions.click(&rows, &change, pick, window, cx)
        }
    })
    .on_mouse_down(MouseButton::Right, {
        let (actions, rows, change) = (actions.clone(), rows.clone(), change.clone());
        move |_, window, cx| actions.point(&rows, &change, window, cx)
    })
    .context_menu({
        let (actions, rows, change) = (actions.clone(), rows.clone(), change.clone());
        move |menu, _, _| actions.menu(menu, &rows, &change)
    })
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{discard_files_label, discard_question};
    use crate::git::{ChangeKind, FileChange};

    fn change(kind: ChangeKind, path: &str) -> FileChange {
        FileChange::new(path.as_bytes(), None, kind)
    }

    fn tracked() -> Vec<FileChange> {
        vec![
            change(ChangeKind::Modified, "src/git/command.rs"),
            change(ChangeKind::Deleted, "src/git/old.rs"),
        ]
    }

    fn untracked() -> Vec<FileChange> {
        vec![
            change(ChangeKind::Untracked, "docs/a.md"),
            change(ChangeKind::Untracked, "docs/b.md"),
        ]
    }

    #[test]
    fn one_file_is_named_in_the_question() {
        let question = discard_question(&tracked()[..1]);
        assert_eq!(question.title, "Discard the changes to command.rs?");
        assert_eq!(question.confirm, "Discard");
        assert_eq!(discard_question(&untracked()[..1]).title, "Delete a.md?");
    }

    #[test]
    fn several_tracked_files_are_counted_and_discarded() {
        let question = discard_question(&tracked());
        assert_eq!(question.title, "Discard the changes to 2 files?");
        assert_eq!(
            question.description,
            "The unstaged changes in these 2 files are thrown away. Git keeps no copy of them, so this can't be undone."
        );
        assert_eq!(question.confirm, "Discard");
        assert_eq!(discard_files_label(&tracked()), "Discard 2 files…");
    }

    #[test]
    fn several_untracked_files_are_deleted() {
        let question = discard_question(&untracked());
        assert_eq!(question.title, "Delete 2 files?");
        assert_eq!(question.confirm, "Delete");
        assert_eq!(discard_files_label(&untracked()), "Delete 2 files…");
    }

    #[test]
    fn a_mix_says_how_many_untracked_files_it_deletes() {
        let mixed = [tracked(), untracked()[..1].to_vec()].concat();
        let question = discard_question(&mixed);
        assert_eq!(question.title, "Discard the changes to 3 files?");
        assert_eq!(
            question.description,
            "The unstaged changes in 2 files are thrown away and 1 untracked file is deleted. Git keeps no copy of them, so this can't be undone."
        );
        assert_eq!(discard_files_label(&mixed), "Discard 3 files…");
    }
}
