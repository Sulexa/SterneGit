use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{ActiveTheme as _, diff::DiffFile, h_flex, scroll::Scrollbar},
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::patch_view::change_kind;
use crate::ui::{self, Handler, Look as _, file_row, handler};

#[derive(IntoElement)]
pub(crate) struct CommitFiles {
    files: Arc<[DiffFile]>,
    open: Option<SharedString>,
    scroll: UniformListScrollHandle,
    on_open: Option<Handler<DiffFile>>,
}

impl CommitFiles {
    pub(crate) fn new(files: Arc<[DiffFile]>, scroll: UniformListScrollHandle) -> Self {
        Self {
            files,
            open: None,
            scroll,
            on_open: None,
        }
    }

    pub(crate) fn open(mut self, path: Option<SharedString>) -> Self {
        self.open = path;
        self
    }

    pub(crate) fn on_open(
        mut self,
        callback: impl Fn(&DiffFile, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(handler(callback));
        self
    }
}

impl RenderOnce for CommitFiles {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let Self {
            files,
            open,
            scroll,
            on_open,
        } = self;
        div()
            .id("commit-files")
            .relative()
            .size_full()
            .child(
                uniform_list("commit-file-list", files.len(), move |range, _, cx| {
                    files[range]
                        .iter()
                        .map(|file| {
                            let selected = open.as_ref() == Some(file.path());
                            commit_file_row(file, selected, &on_open, cx)
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll))
    }
}

fn commit_file_row(
    file: &DiffFile,
    selected: bool,
    on_open: &Option<Handler<DiffFile>>,
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
        selected,
        counts,
        cx,
    )
    .when_some(on_open.clone(), |row, on_open| {
        row.on_click(move |_, window, cx| on_open(&opened, window, cx))
    })
    .into_any_element()
}
