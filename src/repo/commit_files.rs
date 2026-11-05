use std::{path::Path, sync::Arc};

use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _,
        diff::DiffFile,
        h_flex,
        menu::{ContextMenuExt as _, PopupMenu},
        scroll::Scrollbar,
    },
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::{
    file_menu::{history_item, on_disk_items},
    file_tree::{FileTreeRow, file_rows},
    patch_view::change_kind,
};
use crate::{
    git::ChangeKind,
    ui::{self, FilePlace, FileView, Folded, Handler, Look as _, file_row, folder_row, handler},
};

#[derive(Clone, Default)]
struct FileActions {
    root: Option<Arc<Path>>,
    on_open: Option<Handler<DiffFile>>,
    on_history: Option<Handler<DiffFile>>,
}

impl FileActions {
    fn menu(&self, menu: PopupMenu, file: &DiffFile) -> PopupMenu {
        let menu = match &self.on_history {
            Some(on_history) => history_item(menu, on_history, file.clone()),
            None => menu,
        };
        match &self.root {
            Some(root) => on_disk_items(
                menu.separator(),
                root.join(file.path().as_ref()),
                change_kind(file) != ChangeKind::Deleted,
            ),
            None => menu,
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct CommitFiles {
    files: Arc<[DiffFile]>,
    rows: Vec<FileTreeRow>,
    open: Option<SharedString>,
    scroll: UniformListScrollHandle,
    actions: FileActions,
    on_fold: Option<Handler<SharedString>>,
}

impl CommitFiles {
    pub(crate) fn new(
        files: Arc<[DiffFile]>,
        view: FileView,
        folded: &Folded<SharedString>,
        scroll: UniformListScrollHandle,
    ) -> Self {
        Self {
            rows: file_rows(view, files.iter().map(|file| file.path().as_ref()), folded),
            files,
            open: None,
            scroll,
            actions: FileActions::default(),
            on_fold: None,
        }
    }

    pub(crate) fn root(mut self, root: Option<Arc<Path>>) -> Self {
        self.actions.root = root;
        self
    }

    pub(crate) fn on_history(
        mut self,
        callback: impl Fn(&DiffFile, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_history = Some(handler(callback));
        self
    }

    pub(crate) fn open(mut self, path: Option<SharedString>) -> Self {
        self.open = path;
        self
    }

    pub(crate) fn on_open(
        mut self,
        callback: impl Fn(&DiffFile, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.on_open = Some(handler(callback));
        self
    }

    pub(crate) fn on_fold(
        mut self,
        callback: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_fold = Some(handler(callback));
        self
    }
}

impl RenderOnce for CommitFiles {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let Self {
            files,
            rows,
            open,
            scroll,
            actions,
            on_fold,
        } = self;
        div()
            .id("commit-files")
            .relative()
            .size_full()
            .child(
                uniform_list("commit-file-list", rows.len(), move |range, _, cx| {
                    rows[range]
                        .iter()
                        .map(|row| match row {
                            FileTreeRow::Folder {
                                path,
                                name,
                                depth,
                                open,
                            } => commit_folder_row(path, name, *depth, *open, &on_fold, cx),
                            &FileTreeRow::File { ix, place } => {
                                let file = &files[ix];
                                let selected = open.as_ref() == Some(file.path());
                                commit_file_row(file, place, selected, &actions, cx)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll))
    }
}

fn commit_folder_row(
    path: &SharedString,
    name: &str,
    depth: usize,
    open: bool,
    on_fold: &Option<Handler<SharedString>>,
    cx: &App,
) -> AnyElement {
    let folded = path.clone();
    folder_row(
        SharedString::from(format!("commit-folder/{path}/")),
        name,
        depth,
        open,
        None,
        cx,
    )
    .when_some(on_fold.clone(), |row, on_fold| {
        row.on_click(move |_, window, cx| on_fold(&folded, window, cx))
    })
    .into_any_element()
}

fn commit_file_row(
    file: &DiffFile,
    place: FilePlace,
    selected: bool,
    actions: &FileActions,
    cx: &App,
) -> AnyElement {
    let counts = if file.is_binary() {
        div()
            .flex_none()
            .pr_2()
            .xs_text()
            .text_color(cx.theme().muted_foreground)
            .child("binary")
    } else {
        h_flex()
            .flex_none()
            .pr_2()
            .gap_1p5()
            .xs_text()
            .child(ui::additions(file.additions(), cx))
            .child(ui::deletions(file.deletions(), cx))
    };
    let opened = file.clone();
    file_row(
        SharedString::from(format!("commit-file/{}", file.path())),
        change_kind(file),
        file.path(),
        place,
        selected,
        counts,
        cx,
    )
    .when_some(actions.on_open.clone(), |row, on_open| {
        row.on_click(move |_, window, cx| on_open(&opened, window, cx))
    })
    .context_menu({
        let actions = actions.clone();
        let file = file.clone();
        move |menu, _, _| actions.menu(menu, &file)
    })
    .into_any_element()
}
