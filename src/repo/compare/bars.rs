use gpui_kit::{
    AnyElement, App, ClickEvent, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Selectable as _, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    actions::{CloseDiff, NextChange, NextFile, PreviousChange, PreviousFile, WINDOW_CONTEXT, WholeWindow},
    git::ChangeKind,
    ui::{Handler, Look as _, change_badge, thousands},
};

const FILE_BAR_HEIGHT: f32 = 40.;
const VIEW_BAR_HEIGHT: f32 = 32.;
const SEPARATOR_HEIGHT: f32 = 16.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FilePlace {
    pub(crate) whole_window: bool,
    pub(crate) position: Option<Position>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Position {
    pub(crate) index: usize,
    pub(crate) count: usize,
}

#[derive(IntoElement)]
pub(crate) struct FileBar {
    kind: ChangeKind,
    path: SharedString,
    stored_with_lfs: bool,
    state: SharedString,
    facts: Option<AnyElement>,
    place: FilePlace,
    move_button: Option<AnyElement>,
    view: Option<ViewBar>,
    on_whole_window: Handler<()>,
    on_step_file: Handler<Direction>,
    on_close: Handler<()>,
}

impl FileBar {
    pub(crate) fn new(
        kind: ChangeKind,
        path: SharedString,
        state: SharedString,
        place: FilePlace,
        on_whole_window: Handler<()>,
        on_step_file: Handler<Direction>,
        on_close: Handler<()>,
    ) -> Self {
        Self {
            kind,
            path,
            stored_with_lfs: false,
            state,
            facts: None,
            place,
            move_button: None,
            view: None,
            on_whole_window,
            on_step_file,
            on_close,
        }
    }

    pub(crate) fn stored_with_lfs(mut self, stored: bool) -> Self {
        self.stored_with_lfs = stored;
        self
    }

    pub(crate) fn facts(mut self, facts: impl IntoElement) -> Self {
        self.facts = Some(facts.into_any_element());
        self
    }

    pub(crate) fn move_button(mut self, button: Option<AnyElement>) -> Self {
        self.move_button = button;
        self
    }

    pub(crate) fn view(mut self, view: ViewBar) -> Self {
        self.view = Some(view.inline());
        self
    }

    fn file_stepper(&self, position: Position, cx: &App) -> AnyElement {
        let step = |id: &'static str, icon: IconName, direction: Direction| {
            let on_step_file = self.on_step_file.clone();
            let button = Button::new(id).ghost().xsmall().icon(icon);
            let button = match direction {
                Direction::Back => {
                    button.tooltip_with_action("Previous file", &PreviousFile, Some(WINDOW_CONTEXT))
                }
                Direction::Forward => {
                    button.tooltip_with_action("Next file", &NextFile, Some(WINDOW_CONTEXT))
                }
            };
            button.on_click(move |_, window, cx| on_step_file(&direction, window, cx))
        };
        h_flex()
            .flex_none()
            .gap_0p5()
            .mr_1()
            .child(step("previous-file", IconName::ChevronLeft, Direction::Back))
            .child(
                div()
                    .px_1()
                    .xs_text()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!(
                        "File {} of {}",
                        thousands(position.index + 1),
                        thousands(position.count)
                    )),
            )
            .child(step("next-file", IconName::ChevronRight, Direction::Forward))
            .into_any_element()
    }
}

impl RenderOnce for FileBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let stepper = self
            .place
            .position
            .filter(|_| self.place.whole_window)
            .map(|position| self.file_stepper(position, cx));
        let on_whole_window = self.on_whole_window.clone();
        let on_close = self.on_close.clone();
        let border = cx.theme().border;
        let view = self.view.map(|view| {
            h_flex()
                .flex_none()
                .gap_2()
                .child(view)
                .child(separator(border))
        });
        h_flex()
            .flex_none()
            .w_full()
            .min_w_0()
            .h(px(FILE_BAR_HEIGHT))
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(change_badge(self.kind, cx))
            .child(
                div()
                    .ellipsis()
                    .sm_text()
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(self.path),
            )
            .when(self.stored_with_lfs, |this| {
                this.child(
                    div()
                        .flex_none()
                        .px_1()
                        .rounded(px(3.))
                        .border_1()
                        .border_color(cx.theme().border)
                        .text_size(px(10.))
                        .line_height(px(14.))
                        .font_semibold()
                        .text_color(muted)
                        .child("LFS"),
                )
            })
            .child(div().flex_none().xs_text().text_color(muted).child(self.state))
            .children(self.facts.map(|facts| div().flex_none().xs_text().child(facts)))
            .child(div().flex_1())
            .children(view)
            .children(stepper)
            .children(self.move_button)
            .child(
                Button::new("whole-window")
                    .ghost()
                    .xsmall()
                    .icon(if self.place.whole_window {
                        IconName::Minimize2
                    } else {
                        IconName::Maximize2
                    })
                    .selected(self.place.whole_window)
                    .tooltip_with_action(
                        if self.place.whole_window {
                            "Show the refs and the file list again"
                        } else {
                            "Use the whole window"
                        },
                        &WholeWindow,
                        Some(WINDOW_CONTEXT),
                    )
                    .on_click(move |_, window, cx| on_whole_window(&(), window, cx)),
            )
            .child(
                Button::new("diff-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip_with_action("Back to the graph", &CloseDiff, Some(WINDOW_CONTEXT))
                    .on_click(move |_, window, cx| on_close(&(), window, cx)),
            )
    }
}

#[derive(IntoElement)]
pub(crate) struct ChangeStepper {
    position: Option<Position>,
    noun: &'static str,
    on_step: Handler<Direction>,
}

impl ChangeStepper {
    pub(crate) fn new(position: Option<Position>, on_step: Handler<Direction>) -> Self {
        Self {
            position,
            noun: "Change",
            on_step,
        }
    }

    pub(crate) fn noun(mut self, noun: &'static str) -> Self {
        self.noun = noun;
        self
    }
}

impl RenderOnce for ChangeStepper {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = match self.position {
            Some(position) => format!(
                "{} {} of {}",
                self.noun,
                thousands(position.index + 1),
                thousands(position.count)
            ),
            None => "No changes".to_owned(),
        };
        let step = |id: &'static str, icon: IconName, direction: Direction| {
            let on_step = self.on_step.clone();
            let button = Button::new(id)
                .ghost()
                .xsmall()
                .icon(icon)
                .disabled(self.position.is_none());
            let button = match direction {
                Direction::Back => button.tooltip_with_action(
                    "Previous change",
                    &PreviousChange,
                    Some(WINDOW_CONTEXT),
                ),
                Direction::Forward => {
                    button.tooltip_with_action("Next change", &NextChange, Some(WINDOW_CONTEXT))
                }
            };
            button.on_click(move |_, window, cx| on_step(&direction, window, cx))
        };
        h_flex()
            .flex_none()
            .gap_0p5()
            .child(
                div()
                    .pr_1()
                    .xs_text()
                    .text_color(cx.theme().muted_foreground)
                    .child(text),
            )
            .child(step("previous-change", IconName::ArrowUp, Direction::Back))
            .child(step("next-change", IconName::ArrowDown, Direction::Forward))
    }
}

fn separator(color: gpui_kit::Hsla) -> AnyElement {
    div()
        .flex_none()
        .w(px(1.))
        .h(px(SEPARATOR_HEIGHT))
        .bg(color)
        .into_any_element()
}

#[derive(IntoElement)]
pub(crate) struct ViewBar {
    modes: Vec<Button>,
    picker: Option<AnyElement>,
    note: Option<AnyElement>,
    stepper: Option<ChangeStepper>,
    tools: Vec<AnyElement>,
    inline: bool,
}

impl ViewBar {
    pub(crate) fn new() -> Self {
        Self {
            modes: Vec::new(),
            picker: None,
            note: None,
            stepper: None,
            tools: Vec::new(),
            inline: false,
        }
    }

    fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    pub(crate) fn mode(
        mut self,
        id: &'static str,
        label: &'static str,
        icon: IconName,
        tooltip: &'static str,
        selected: bool,
        on_choose: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.modes.push(
            Button::new(id)
                .ghost()
                .xsmall()
                .icon(icon)
                .label(label)
                .selected(selected)
                .tooltip(tooltip)
                .on_click(on_choose),
        );
        self
    }

    pub(crate) fn note(mut self, note: impl IntoElement) -> Self {
        self.note = Some(note.into_any_element());
        self
    }

    pub(crate) fn picker(mut self, picker: impl IntoElement) -> Self {
        self.picker = Some(picker.into_any_element());
        self
    }

    pub(crate) fn without_stepper(mut self) -> Self {
        self.stepper = None;
        self
    }

    pub(crate) fn stepper(mut self, stepper: ChangeStepper) -> Self {
        self.stepper = Some(stepper);
        self
    }

    pub(crate) fn tool(mut self, tool: impl IntoElement) -> Self {
        self.tools.push(tool.into_any_element());
        self
    }
}

impl RenderOnce for ViewBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border = cx.theme().border;
        let separated = self.stepper.is_some() && !self.tools.is_empty();
        h_flex()
            .flex_none()
            .gap_2()
            .when(!self.inline, |this| {
                this.w_full()
                    .min_w_0()
                    .h(px(VIEW_BAR_HEIGHT))
                    .px_3()
                    .border_b_1()
                    .border_color(border)
            })
            .child(h_flex().flex_none().gap_0p5().children(self.modes))
            .when_some(self.picker, |this, picker| this.child(separator(border)).child(picker))
            .when(!self.inline, |this| this.child(div().flex_1()))
            .when_some(self.note, |this, note| this.child(note).child(separator(border)))
            .children(self.stepper)
            .when(separated, |this| this.child(separator(border)))
            .child(h_flex().flex_none().gap_0p5().children(self.tools))
    }
}
