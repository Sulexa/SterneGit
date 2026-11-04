use std::sync::Arc;

use gpui_kit::{
    AnyElement, AnyView, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
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
    ui::{self, Handler, Look as _, SectionHeader, SideUi as _, file_row, handler, thousands},
};

const SIDES_TOP_DOWN: [Side; 2] = [Side::Staged, Side::Unstaged];

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
    writing: bool,
    scrolls: SideScrolls,
    on_open: Option<Handler<(Side, FileChange)>>,
    on_move: Option<Handler<(Side, Scope)>>,
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
            writing: false,
            scrolls,
            on_open: None,
            on_move: None,
        }
    }

    pub(crate) fn open(mut self, file: Option<OpenFile>) -> Self {
        self.open = file;
        self
    }

    pub(crate) fn writing(mut self, writing: bool) -> Self {
        self.writing = writing;
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
            .child(side_header(side, count, self.writing, self.on_move.clone()))
            .child(div().fill_height().child(content))
            .into_any_element()
    }

    fn side_list(&self, side: Side, count: usize, changes: Arc<WorkingChanges>) -> AnyElement {
        let scroll = self.scrolls.of(side).clone();
        let open = self.open.clone();
        let writing = self.writing;
        let on_open = self.on_open.clone();
        let on_move = self.on_move.clone();
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
                                change_row(change, side, selected, writing, &on_open, &on_move, cx)
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
    writing: bool,
    on_open: &Option<Handler<(Side, FileChange)>>,
    on_move: &Option<Handler<(Side, Scope)>>,
    cx: &App,
) -> AnyElement {
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
    .into_any_element()
}
