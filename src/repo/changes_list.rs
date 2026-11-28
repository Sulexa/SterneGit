use std::{path::Path, rc::Rc, sync::Arc};

use gpui_kit::{
    Anchor, AnyElement, AnyView, App, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, UniformListScrollHandle, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem},
        scroll::Scrollbar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, relative, uniform_list,
};

use super::{
    file_menu::{history_item, many_on_disk_items, on_disk_items},
    file_tree::{FileTreeRow, file_rows, is_inside},
    navigation::OpenFile,
};
use crate::{
    git::{ChangeKind, FileChange, Scope, Side, WorkingChanges, side_label},
    ui::{
        self, BySide, FilePlace, FileView, FileViewToggle, Folded, Handler, Look as _, Pick,
        Picked, ROW_HEIGHT, SectionHeader, SideUi as _, counted, file_name, file_row, file_row_tagged, folder_row, handler,
        side_tag, tags_of, thousands,
    },
};

const SIDES_TOP_DOWN: [Side; 2] = [Side::Staged, Side::Unstaged];
const CONFLICTS_HEIGHT: f32 = 0.4;
const STAGED_HEIGHT: f32 = 0.4;
const EMPTY_HINT_HEIGHT: Pixels = px(68.);
const DISCARD_MENU_WIDTH: f32 = 300.;
const ROW_GROUP: &str = "change-row";
const DISCARD_FOLDER: &str = "Discard folder…";
const DISCARD_TO_LAST_COMMIT: &str = "Discard all changes…";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiscardReach {
    Unstaged,
    Everything,
    Untracked,
}

#[derive(Debug, PartialEq, Eq)]
struct DiscardEntry {
    reach: DiscardReach,
    label: String,
    about: &'static str,
    count: usize,
}

fn discard_entries(changes: &WorkingChanges) -> [DiscardEntry; 3] {
    let entry = |reach, label: &str, about, count| DiscardEntry {
        reach,
        label: format!("{label} ({})…", thousands(count)),
        about,
        count,
    };
    [
        entry(
            DiscardReach::Unstaged,
            "Discard unstaged changes",
            "Staged changes stay as they are",
            changes.unstaged.len(),
        ),
        entry(
            DiscardReach::Everything,
            "Discard all changes",
            "Every file goes back to the last commit, staged ones too",
            changes.file_count(),
        ),
        entry(
            DiscardReach::Untracked,
            "Delete untracked files",
            "Only the files Git does not track yet",
            untracked_count(&changes.unstaged),
        ),
    ]
}

fn untracked_files(changes: &[FileChange]) -> Vec<FileChange> {
    changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Untracked)
        .cloned()
        .collect()
}

fn is_new(change: &FileChange) -> bool {
    matches!(change.kind, ChangeKind::Added | ChangeKind::Untracked)
}

pub(crate) fn last_commit_question(changes: &[FileChange]) -> DiscardQuestion {
    const LOST: &str = "Git keeps no copy of them, so this can't be undone.";
    if let [change] = changes {
        let name = change.path.rsplit('/').next().unwrap_or(&change.path);
        let description = if is_new(change) {
            format!(
                "{} is new, so it is deleted, staged changes included. Git keeps no copy of it, so this can't be undone.",
                change.path
            )
        } else {
            format!(
                "{} goes back to how it was in the last commit: its staged and unstaged changes are thrown away. {LOST}",
                change.path
            )
        };
        return DiscardQuestion {
            title: format!("Discard all changes to {name}?"),
            description,
            confirm: "Discard",
        };
    }
    let count = thousands(changes.len());
    let created = changes.iter().filter(|change| is_new(change)).count();
    let back = changes.len() - created;
    let what = match (back, created) {
        (_, 0) => format!(
            "These {count} files go back to how they were in the last commit: their staged and unstaged changes are thrown away."
        ),
        (0, _) => format!("These {count} files are new, so they are deleted, staged changes included."),
        _ => format!(
            "{} back to how they were in the last commit and {} deleted, staged changes included.",
            counted(back, "file goes", "files go"),
            counted(created, "new file is", "new files are"),
        ),
    };
    DiscardQuestion {
        title: format!("Discard all changes to {count} files?"),
        description: format!("{what} {LOST}"),
        confirm: "Discard",
    }
}

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
    on_discard_to_last_commit: Option<Handler<Vec<FileChange>>>,
    on_discard_all: Option<Handler<()>>,
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
        let menu = match (side, &self.on_discard, &self.on_discard_to_last_commit) {
            (Side::Unstaged, Some(_), _) => menu.item(self.item(
                discard_files_label(&changes),
                &self.on_discard,
                changes.clone(),
            )),
            (Side::Staged, _, Some(_)) => menu.item(self.item(
                format!("Discard all changes to {count} files…"),
                &self.on_discard_to_last_commit,
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
        let (label, on_discard) = match side {
            Side::Unstaged => (discard_label(change), &self.on_discard),
            Side::Staged => (Some(DISCARD_TO_LAST_COMMIT), &self.on_discard_to_last_commit),
        };
        let menu = match (label, on_discard) {
            (Some(label), Some(_)) => menu.item(self.item(label, on_discard, vec![change.clone()])),
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
    underway: bool,
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
            underway: false,
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
                on_discard_to_last_commit: None,
                on_discard_all: None,
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

    pub(crate) fn on_discard_to_last_commit(
        mut self,
        callback: impl Fn(&Vec<FileChange>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_discard_to_last_commit = Some(handler(callback));
        self
    }

    pub(crate) fn on_discard_all(mut self, callback: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.actions.on_discard_all = Some(handler(callback));
        self
    }

    pub(crate) fn on_history(
        mut self,
        callback: impl Fn(&FileChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_history = Some(handler(callback));
        self
    }

    pub(crate) fn underway(mut self, underway: bool) -> Self {
        self.underway = underway;
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
        let labels = side.labels();
        v_flex()
            .map(|this| match side {
                Side::Staged => this
                    .flex_none()
                    .h(fitted_height(self.rows.of(side).len()))
                    .max_h(relative(STAGED_HEIGHT)),
                Side::Unstaged => this.fill_height().border_t_1().border_color(cx.theme().border),
            })
            .child(SectionHeader::new(format!("{} ({})", labels.files, thousands(count))))
            .child(div().fill_height().child(content))
            .into_any_element()
    }

    fn files_bar(&self, changes: &Arc<WorkingChanges>, cx: &App) -> AnyElement {
        let actions = h_flex()
            .flex_none()
            .gap_0p5()
            .child(self.all_button(Side::Unstaged, changes))
            .child(self.all_button(Side::Staged, changes))
            .when(!self.underway, |this| this.child(self.discard_button(changes)))
            .child(div().flex_none().mx_1().w(px(1.)).h(px(16.)).bg(cx.theme().border))
            .child(FileViewToggle);
        div()
            .flex_none()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                SectionHeader::new(counted(changes.file_count(), "changed file", "changed files"))
                    .action(actions),
            )
            .into_any_element()
    }

    fn all_button(&self, side: Side, changes: &WorkingChanges) -> Button {
        let labels = side.labels();
        let everything = everything_on(side, changes);
        Button::new(SharedString::from(format!("all-{side:?}")))
            .ghost()
            .xsmall()
            .icon(labels.icon)
            .label(labels.all)
            .disabled(self.actions.writing || changes.files(side).is_empty())
            .when_some(self.actions.on_move.clone(), |button, on_move| {
                button.on_click(move |_, window, cx| on_move(&(side, everything.clone()), window, cx))
            })
    }

    fn discard_button(&self, changes: &Arc<WorkingChanges>) -> AnyElement {
        let actions = self.actions.clone();
        let changes = changes.clone();
        Button::new("discard-menu")
            .ghost()
            .xsmall()
            .icon(IconName::Undo2)
            .label("Discard")
            .dropdown_caret(true)
            .tooltip("Throw changes away")
            .disabled(self.actions.writing || changes.is_empty())
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                discard_menu(menu.min_w(px(DISCARD_MENU_WIDTH)), &changes, &actions)
            })
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
            std::iter::once(self.files_bar(&changes, cx))
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
            .when(!self.underway, |this| this.child(self.form))
            .children(lists)
            .into_any_element()
    }
}

fn fitted_height(rows: usize) -> Pixels {
    match rows {
        0 => ROW_HEIGHT + EMPTY_HINT_HEIGHT,
        rows => ROW_HEIGHT * (rows as f32 + 1.),
    }
}

fn discard_menu(menu: PopupMenu, changes: &WorkingChanges, actions: &RowActions) -> PopupMenu {
    discard_entries(changes)
        .into_iter()
        .fold(menu, |menu, entry| {
            let menu = match entry.reach {
                DiscardReach::Untracked => menu.separator(),
                _ => menu,
            };
            menu.item(discard_item(entry, changes, actions))
        })
}

fn discard_item(entry: DiscardEntry, changes: &WorkingChanges, actions: &RowActions) -> PopupMenuItem {
    let DiscardEntry { reach, label, about, count } = entry;
    let icon = match reach {
        DiscardReach::Untracked => IconName::Trash,
        _ => IconName::Undo2,
    };
    let chosen = match reach {
        DiscardReach::Unstaged => Some(changes.unstaged.clone()),
        DiscardReach::Untracked => Some(untracked_files(&changes.unstaged)),
        DiscardReach::Everything => None,
    };
    let (on_discard, on_discard_all) = (actions.on_discard.clone(), actions.on_discard_all.clone());
    let reachable = match reach {
        DiscardReach::Everything => on_discard_all.is_some(),
        _ => on_discard.is_some(),
    };
    let label = SharedString::from(label);
    PopupMenuItem::element(move |_, cx| {
        v_flex().child(label.clone()).child(
            div()
                .xs_text()
                .text_color(cx.theme().muted_foreground)
                .child(about),
        )
    })
    .icon(Icon::new(icon))
    .disabled(count == 0 || actions.writing || !reachable)
    .on_click(move |_, window, cx| match (&chosen, &on_discard, &on_discard_all) {
        (Some(chosen), Some(on_discard), _) => on_discard(chosen, window, cx),
        (None, _, Some(on_discard_all)) => on_discard_all(&(), window, cx),
        _ => {}
    })
}

fn row_discard(
    id: String,
    tooltip: &'static str,
    writing: bool,
    discarded: impl Fn() -> Vec<FileChange> + 'static,
    on_discard: Handler<Vec<FileChange>>,
) -> AnyElement {
    div()
        .flex_none()
        .invisible()
        .group_hover(ROW_GROUP, |style| style.visible())
        .child(
            Button::new(SharedString::from(id))
                .ghost()
                .xsmall()
                .icon(IconName::Undo2)
                .tooltip(tooltip)
                .disabled(writing)
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    on_discard(&discarded(), window, cx)
                }),
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
    let inside = {
        let (changes, path) = (changes.clone(), folder.path.clone());
        move || -> Vec<FileChange> {
            changes
                .files(side)
                .iter()
                .filter(|change| is_inside(&change.path, &path))
                .cloned()
                .collect()
        }
    };
    let move_button = Button::new(SharedString::from(format!("stage/{id}")))
        .ghost()
        .xsmall()
        .icon(labels.icon)
        .tooltip(labels.folder_tooltip)
        .disabled(actions.writing)
        .when_some(actions.on_move.clone(), |button, on_move| {
            let inside = inside.clone();
            button.on_click(move |_, window, cx| {
                cx.stop_propagation();
                on_move(&(side, Scope::Files(inside())), window, cx)
            })
        });
    let discard = match (side, actions.on_discard.clone()) {
        (Side::Unstaged, Some(on_discard)) => Some(row_discard(
            format!("discard/{id}"),
            DISCARD_FOLDER,
            actions.writing,
            inside,
            on_discard,
        )),
        _ => None,
    };
    let folded = (side, folder.path.clone());
    folder_row(
        SharedString::from(id),
        folder.name,
        folder.depth,
        folder.open,
        Some(h_flex().flex_none().children(discard).child(move_button).into_any_element()),
        cx,
    )
    .group(ROW_GROUP)
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
    let tags = change.side_files().iter().map(|side| {
        side_tag(
            format!("side/{id}/{}", side.path),
            side_label(&change.path, &side.path),
            format!("{} changed too: it is staged, unstaged and discarded with this file", file_name(&side.path)),
            cx,
        )
    });
    let tags = tags_of(tags);
    let discard = match (side, discard_label(change), actions.on_discard.clone()) {
        (Side::Unstaged, Some(label), Some(on_discard)) => {
            let discarded = change.clone();
            Some(row_discard(
                format!("discard/{id}"),
                label,
                writing,
                move || vec![discarded.clone()],
                on_discard,
            ))
        }
        _ => None,
    };
    file_row_tagged(
        SharedString::from(id),
        change.kind,
        &change.path,
        place,
        selected,
        tags,
        h_flex().flex_none().children(discard).child(move_button),
        cx,
    )
    .group(ROW_GROUP)
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
    use super::{
        DiscardEntry, DiscardReach, EMPTY_HINT_HEIGHT, discard_entries, discard_files_label, discard_question,
        fitted_height, last_commit_question,
    };
    use crate::{
        git::{ChangeKind, FileChange, WorkingChanges},
        ui::ROW_HEIGHT,
    };

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

    #[test]
    fn the_discard_menu_counts_what_each_entry_throws_away() {
        let changes = WorkingChanges {
            staged: vec![change(ChangeKind::Added, "src/ui/toast.rs"), change(ChangeKind::Modified, "src/git/command.rs")],
            unstaged: [tracked(), untracked()].concat(),
            ..WorkingChanges::default()
        };
        let entries = discard_entries(&changes);
        let shown: Vec<_> = entries.iter().map(|entry| (entry.reach, entry.label.as_str(), entry.count)).collect();
        assert_eq!(
            shown,
            [
                (DiscardReach::Unstaged, "Discard unstaged changes (4)…", 4),
                (DiscardReach::Everything, "Discard all changes (5)…", 5),
                (DiscardReach::Untracked, "Delete untracked files (2)…", 2),
            ]
        );
    }

    #[test]
    fn an_entry_with_nothing_to_throw_away_counts_zero() {
        let changes = WorkingChanges {
            staged: tracked(),
            ..WorkingChanges::default()
        };
        let [unstaged, everything, untracked] = discard_entries(&changes);
        assert_eq!((unstaged.count, everything.count, untracked.count), (0, 2, 0));
        assert_eq!(
            everything,
            DiscardEntry {
                reach: DiscardReach::Everything,
                label: "Discard all changes (2)…".into(),
                about: "Every file goes back to the last commit, staged ones too",
                count: 2,
            }
        );
    }

    #[test]
    fn a_staged_file_goes_back_to_the_last_commit_and_a_new_one_is_deleted() {
        let modified = last_commit_question(&tracked()[..1]);
        assert_eq!(modified.title, "Discard all changes to command.rs?");
        assert_eq!(
            modified.description,
            "src/git/command.rs goes back to how it was in the last commit: its staged and unstaged changes are thrown away. Git keeps no copy of them, so this can't be undone."
        );
        let added = last_commit_question(&[change(ChangeKind::Added, "src/ui/toast.rs")]);
        assert_eq!(
            added.description,
            "src/ui/toast.rs is new, so it is deleted, staged changes included. Git keeps no copy of it, so this can't be undone."
        );
    }

    #[test]
    fn several_staged_files_count_what_goes_back_and_what_is_deleted() {
        let mixed = [tracked(), vec![change(ChangeKind::Added, "src/ui/toast.rs")]].concat();
        let question = last_commit_question(&mixed);
        assert_eq!(question.title, "Discard all changes to 3 files?");
        assert_eq!(
            question.description,
            "2 files go back to how they were in the last commit and 1 new file is deleted, staged changes included. Git keeps no copy of them, so this can't be undone."
        );
        assert!(last_commit_question(&tracked()).description.starts_with("These 2 files go back"));
    }

    #[test]
    fn the_staged_list_is_as_tall_as_its_rows() {
        assert_eq!(fitted_height(0), ROW_HEIGHT + EMPTY_HINT_HEIGHT);
        assert_eq!(fitted_height(3), ROW_HEIGHT * 4.);
    }
}
