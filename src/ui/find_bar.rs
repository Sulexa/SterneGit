use std::{ops::Range, rc::Rc};

use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, HighlightStyle, Hsla,
    InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, Styled as _, StyledText, Subscription,
    Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Escape, Input, InputEvent, InputState},
        orange_400,
    },
    div, px,
};

use super::{
    Look as _,
    search::{Search, SearchOptions},
    text::{counted, thousands},
};
use crate::actions::{CloseDiff, FIND_CONTEXT, ToggleMatchCase, TogglePattern, ToggleWholeWords};

const HEIGHT: f32 = 40.;
const FIELD_WIDTH: f32 = 380.;
const FIELD_MIN_WIDTH: f32 = 180.;
const COUNT_WIDTH: f32 = 72.;
const FOUND_TINT: f32 = 0.32;

const NO_RESULTS: &str = "No results";
const BAD_PATTERN: &str = "Not a valid pattern";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Found {
    pub(crate) count: usize,
    pub(crate) current: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindEvent {
    Searched,
    Next,
    Previous,
    Closed,
}

type Results = Rc<dyn Fn(&App) -> Found>;

pub(crate) struct FindBar {
    input: Entity<InputState>,
    options: SearchOptions,
    bad_pattern: bool,
    note: Option<SharedString>,
    results: Results,
    _subscription: Subscription,
}

impl EventEmitter<FindEvent> for FindBar {}

impl FindBar {
    pub(crate) fn new(
        placeholder: &'static str,
        results: impl Fn(&App) -> Found + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let subscription = cx.subscribe(&input, |this, _, event, cx| match event {
            InputEvent::Change => this.searched(cx),
            InputEvent::PressEnter { shift: false, .. } => cx.emit(FindEvent::Next),
            InputEvent::PressEnter { shift: true, .. } => cx.emit(FindEvent::Previous),
            _ => {}
        });
        Self {
            input,
            options: SearchOptions::default(),
            bad_pattern: false,
            note: None,
            results: Rc::new(results),
            _subscription: subscription,
        }
    }

    pub(crate) fn search(&self, cx: &App) -> Search {
        Search {
            text: self.input.read(cx).value().to_string(),
            options: self.options,
        }
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
    }

    pub(crate) fn set_note(&mut self, note: Option<SharedString>, cx: &mut Context<Self>) {
        if self.note != note {
            self.note = note;
            cx.notify();
        }
    }

    fn searched(&mut self, cx: &mut Context<Self>) {
        self.bad_pattern = self.search(cx).matcher().is_err();
        cx.emit(FindEvent::Searched);
        cx.notify();
    }

    fn toggle(&mut self, flip: fn(&mut SearchOptions), cx: &mut Context<Self>) {
        flip(&mut self.options);
        self.searched(cx);
    }

    fn count(&self, typed: bool, found: Found) -> Option<String> {
        if !typed {
            return None;
        }
        if self.bad_pattern {
            return Some(BAD_PATTERN.into());
        }
        Some(match (found.count, found.current) {
            (0, _) => NO_RESULTS.into(),
            (count, Some(current)) => format!("{} of {}", thousands(current + 1), thousands(count)),
            (count, None) => counted(count, "result", "results"),
        })
    }
}

impl Render for FindBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let typed = !self.input.read(cx).value().is_empty();
        let found = (self.results)(cx);
        let count = self.count(typed, found);
        let missed = typed && (self.bad_pattern || found.count == 0);
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let options = self.options;
        let toggle = |id: &'static str, icon: IconName, tooltip: &'static str, on: bool, flip: fn(&mut SearchOptions)| {
            Button::new(id)
                .ghost()
                .xsmall()
                .compact()
                .icon(icon)
                .tooltip(tooltip)
                .selected(on)
                .on_click(cx.listener(move |this, _, _, cx| this.toggle(flip, cx)))
        };
        let toggles = h_flex()
            .gap_px()
            .child(toggle(
                "find-match-case",
                IconName::CaseSensitive,
                "Match case  Alt+C",
                options.match_case,
                |options| options.match_case = !options.match_case,
            ))
            .child(toggle(
                "find-whole-words",
                IconName::WholeWord,
                "Whole words only  Alt+W",
                options.whole_words,
                |options| options.whole_words = !options.whole_words,
            ))
            .child(toggle(
                "find-pattern",
                IconName::Regex,
                "Regular expression  Alt+R",
                options.pattern,
                |options| options.pattern = !options.pattern,
            ));
        let field = Input::new(&self.input)
            .small()
            .prefix(Icon::new(IconName::Search).small().text_color(muted))
            .suffix(toggles);
        let field = if missed { field.border_color(theme.danger) } else { field };
        let step = |id: &'static str, icon: IconName, tooltip: &'static str, event: FindEvent| {
            Button::new(id)
                .ghost()
                .xsmall()
                .icon(icon)
                .tooltip(tooltip)
                .disabled(found.count == 0)
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(event)))
        };
        h_flex()
            .key_context(FIND_CONTEXT)
            .flex_none()
            .w_full()
            .h(px(HEIGHT))
            .pl_3()
            .pr_2()
            .gap_1()
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.background)
            .capture_action(cx.listener(|_, _: &Escape, _, cx| {
                cx.stop_propagation();
                cx.emit(FindEvent::Closed);
            }))
            .capture_action(cx.listener(|_, _: &CloseDiff, _, cx| {
                cx.stop_propagation();
                cx.emit(FindEvent::Closed);
            }))
            .on_action(cx.listener(|this, _: &ToggleMatchCase, _, cx| {
                this.toggle(|options| options.match_case = !options.match_case, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleWholeWords, _, cx| {
                this.toggle(|options| options.whole_words = !options.whole_words, cx)
            }))
            .on_action(cx.listener(|this, _: &TogglePattern, _, cx| {
                this.toggle(|options| options.pattern = !options.pattern, cx)
            }))
            .child(
                div()
                    .w(px(FIELD_WIDTH))
                    .min_w(px(FIELD_MIN_WIDTH))
                    .mr_1p5()
                    .child(field),
            )
            .child(
                div()
                    .flex_none()
                    .min_w(px(COUNT_WIDTH))
                    .xs_text()
                    .whitespace_nowrap()
                    .text_color(if missed { theme.danger } else { muted })
                    .children(count),
            )
            .child(step("find-previous", IconName::ArrowUp, "Previous result  Shift+Enter", FindEvent::Previous))
            .child(step("find-next", IconName::ArrowDown, "Next result  Enter", FindEvent::Next))
            .children(self.note.clone().map(|note| {
                div().ml_2().xs_text().text_color(muted).ellipsis().child(note)
            }))
            .child(div().flex_1())
            .child(
                Button::new("find-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip("Close the search  Esc")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(FindEvent::Closed))),
            )
    }
}

pub(crate) fn found_color() -> Hsla {
    orange_400()
}

pub(crate) fn found_text(text: SharedString, found: Vec<Range<usize>>) -> StyledText {
    let style = HighlightStyle {
        background_color: Some(found_color().opacity(FOUND_TINT)),
        ..Default::default()
    };
    StyledText::new(text).with_highlights(found.into_iter().map(|range| (range, style)))
}
