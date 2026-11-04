use std::sync::Arc;

use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        scroll::Scrollbar,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    uniform_list,
};

use super::navigation::OpenFile;
use crate::{
    git::{FileChange, Scope, Side, WorkingChanges},
    ui::{
        self, Handler, Look as _, SectionHeader, SectionRow, Sections, SideUi as _, change_badge,
        handler, list_row, thousands,
    },
};

#[derive(IntoElement)]
pub(crate) struct ChangesList {
    changes: Option<Arc<WorkingChanges>>,
    open: Option<OpenFile>,
    writing: bool,
    failure: Option<SharedString>,
    scroll: UniformListScrollHandle,
    on_open: Option<Handler<(Side, FileChange)>>,
    on_move: Option<Handler<(Side, Scope)>>,
}

impl ChangesList {
    pub(crate) fn new(
        changes: Option<Arc<WorkingChanges>>,
        scroll: UniformListScrollHandle,
    ) -> Self {
        Self {
            changes,
            open: None,
            writing: false,
            failure: None,
            scroll,
            on_open: None,
            on_move: None,
        }
    }

    pub(crate) fn open(mut self, file: Option<OpenFile>) -> Self {
        self.open = file;
        self
    }

    pub(crate) fn writing(mut self, writing: bool, failure: Option<SharedString>) -> Self {
        self.writing = writing;
        self.failure = failure;
        self
    }

    pub(crate) fn on_open(
        mut self,
        callback: impl Fn(&(Side, FileChange), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(handler(callback));
        self
    }

    pub(crate) fn on_move(
        mut self,
        callback: impl Fn(&(Side, Scope), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(handler(callback));
        self
    }
}

impl RenderOnce for ChangesList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            changes,
            open,
            writing,
            failure,
            scroll,
            on_open,
            on_move,
        } = self;
        let Some(changes) = changes else {
            return ui::loading("Reading changes…");
        };
        let muted = cx.theme().muted_foreground;
        let header = v_flex()
            .flex_none()
            .px_4()
            .py_3()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().base_text().font_medium().child("Uncommitted changes"))
            .child(div().xs_text().text_color(muted).child(format!(
                "{} unstaged · {} staged",
                thousands(changes.unstaged.len()),
                thousands(changes.staged.len())
            )))
            .when_some(failure, |this, failure| {
                this.child(div().xs_text().text_color(cx.theme().red).child(failure))
            });
        if changes.is_empty() {
            return v_flex()
                .size_full()
                .child(header)
                .child(ui::hint("No uncommitted changes.", cx))
                .into_any_element();
        }
        let sections = Sections::new([
            (Side::Unstaged, changes.unstaged.len()),
            (Side::Staged, changes.staged.len()),
        ]);
        let list = div()
            .id("changes")
            .relative()
            .fill_height()
            .child(
                uniform_list("change-list", sections.rows(), move |range, _, cx| {
                    range
                        .filter_map(|ix| sections.row(ix))
                        .map(|row| match row {
                            SectionRow::Header(side, count) => {
                                side_header(side, count, writing, on_move.clone())
                            }
                            SectionRow::Item(side, ix) => {
                                let change = &changes.files(side)[ix];
                                let selected = open.as_ref().is_some_and(|open| {
                                    open.side == side && open.path == change.path
                                });
                                change_row(change, side, selected, writing, &on_open, &on_move, cx)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&scroll),
            )
            .child(Scrollbar::vertical(&scroll));
        v_flex().pane().child(header).child(list).into_any_element()
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
    writing: bool,
    on_open: &Option<Handler<(Side, FileChange)>>,
    on_move: &Option<Handler<(Side, Scope)>>,
    cx: &App,
) -> AnyElement {
    let (name, folder) = match change.path.rsplit_once('/') {
        Some((folder, name)) => (name.to_owned(), Some(folder.to_owned())),
        None => (change.path.to_string(), None),
    };
    let labels = side.labels();
    let id = format!("{side:?}/{}", change.path);
    let moved = (side, Scope::Files(vec![change.clone()]));
    let opened = (side, change.clone());
    list_row(SharedString::from(id.clone()), selected)
        .pl_3()
        .pr_1()
        .child(
            h_flex()
                .w_full()
                .min_w_0()
                .gap_2()
                .child(change_badge(change.kind, cx))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1p5()
                        .overflow_hidden()
                        .child(
                            div()
                                .flex_none()
                                .max_w_full()
                                .truncate()
                                .sm_text()
                                .child(name),
                        )
                        .when_some(folder, |this, folder| {
                            this.child(
                                div()
                                    .ellipsis()
                                    .xs_text()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(folder),
                            )
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("stage/{id}")))
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
                        }),
                ),
        )
        .when_some(on_open.clone(), |row, on_open| {
            row.on_click(move |_, window, cx| on_open(&opened, window, cx))
        })
        .into_any_element()
}
