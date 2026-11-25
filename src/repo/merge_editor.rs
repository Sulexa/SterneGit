use std::{ops::Range, rc::Rc};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, Focusable as _, Hsla, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, WeakEntity, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        diff::{
            Diff, DiffChangeIndicator, DiffDecoration, DiffFile, DiffHunkSeparator, DiffLineRange,
            DiffMatcher, DiffMode, DiffSide, DiffState,
        },
        h_flex,
        input::{
            Editor, EditorState, GutterDecoration, GutterDecorationCollection, InputEvent,
            Position, RangeDecoration, RangeDecorationCollection, RangeDecorationStyle,
        },
        orange_400,
        resizable::{resizable_panel, v_resizable},
        v_flex,
    },
    div, point,
    prelude::FluentBuilder as _,
    px, relative,
};

use super::{
    compare::{Direction, Position as Place, TextOptions},
    merge_text::{MergeText, Shape, Span, Tint, versions},
};
use crate::{
    git::{FileChange, Version},
    ui::{self, Found, Look as _, Matcher, counted},
};

const HEADER_HEIGHT: f32 = 28.;
const DOT: f32 = 8.;
const ROWS_ABOVE_A_CONFLICT: usize = 2;
const ROWS_ABOVE_A_CHANGE: usize = 4;
const LINE_TINT: f32 = 0.16;
const MARKER_TINT: f32 = 0.2;
const CRLF: &str = "\r\n";
const EDITOR_LINE_HEIGHT: f32 = 1.5;
const FOUND_TINT: f32 = 0.32;
const CURRENT_FOUND_TINT: f32 = 0.6;

pub(crate) struct MergeEditor {
    change: FileChange,
    original: String,
    crlf: bool,
    labels: [SharedString; 2],
    merge: MergeText,
    spans: Rc<[Span]>,
    touched: bool,
    stepped: Option<usize>,
    versions: Option<Entity<DiffState>>,
    result: Entity<EditorState>,
    tints: RangeDecorationCollection,
    bars: GutterDecorationCollection,
    matcher: Option<Matcher>,
    result_found: Vec<Range<usize>>,
    result_current: Option<usize>,
    found_fills: RangeDecorationCollection,
    _subscription: Subscription,
}

impl MergeEditor {
    pub(crate) fn new(
        change: FileChange,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let crlf = text.contains(CRLF);
        let original = text.replace(CRLF, "\n");
        let merge = MergeText::new(&original).unwrap_or_else(|| MergeText::plain(&original));
        let shown = versions(&change.path, &original);
        let labels = merge.regions().first().map_or_else(
            || [SharedString::default(), SharedString::default()],
            |region| {
                let (current, incoming) = region.labels();
                [current.into(), incoming.into()]
            },
        );
        let files = shown
            .as_ref()
            .and_then(|shown| DiffFile::parse(&shown.patch).ok())
            .unwrap_or_default();
        let language = files
            .first()
            .map_or_else(|| SharedString::from("text"), DiffFile::language);
        let versions = (!merge.regions().is_empty()).then(|| {
            let context = TextOptions::current(cx).context_lines();
            cx.new(|cx| {
                DiffState::new(files, cx)
                    .with_mode(DiffMode::Split)
                    .with_context_lines(context)
            })
        });
        let result = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(language)
                .line_number(true)
                .soft_wrap(false)
                .default_value(original.clone())
        });
        let (tints, bars, found_fills) = result.update(cx, |state, cx| {
            (
                state.create_range_decorations_collection(Vec::new(), cx),
                state.create_gutter_decorations_collection(Vec::new(), cx),
                state.create_range_decorations_collection(Vec::new(), cx),
            )
        });
        let subscription = cx.subscribe_in(&result, window, |this, state, event, _, cx| {
            if let InputEvent::Change = event {
                let text = state.read(cx).value();
                this.merge.edited(&text);
                this.touched = true;
                this.paint(cx);
                this.find_in_result(cx);
                cx.notify();
            }
        });
        let mut editor = Self {
            change,
            original,
            crlf,
            labels,
            merge,
            spans: shown.map(|shown| shown.spans).unwrap_or_default().into(),
            touched: false,
            stepped: None,
            versions,
            result,
            tints,
            bars,
            matcher: None,
            result_found: Vec::new(),
            result_current: None,
            found_fills,
            _subscription: subscription,
        };
        editor.paint(cx);
        if !editor.merge.regions().is_empty() {
            editor.reveal(0, cx);
        }
        editor
    }

    pub(crate) fn change(&self) -> &FileChange {
        &self.change
    }

    pub(crate) fn is_touched(&self) -> bool {
        self.touched
    }

    pub(crate) fn total(&self) -> usize {
        self.merge.regions().len()
    }

    pub(crate) fn resolved_count(&self) -> usize {
        self.total() - self.merge.conflicts_left()
    }

    pub(crate) fn is_resolved(&self) -> bool {
        self.merge.is_resolved()
    }

    pub(crate) fn resolved_bytes(&self) -> Vec<u8> {
        let text = self.merge.text();
        if self.crlf {
            text.replace('\n', CRLF).into_bytes()
        } else {
            text.as_bytes().to_vec()
        }
    }

    pub(crate) fn position(&self) -> Option<Place> {
        let count = self.total();
        (count > 0).then(|| Place {
            index: self.stepped.unwrap_or(0),
            count,
        })
    }

    pub(crate) fn find(&mut self, matcher: Option<Matcher>, cx: &mut Context<Self>) {
        self.matcher = matcher;
        if let Some(versions) = &self.versions {
            let matcher = self.matcher.clone().map(|matcher| {
                Rc::new(move |text: &str| matcher.ranges(text)) as DiffMatcher
            });
            versions.update(cx, |state, cx| state.find(matcher, cx));
        }
        self.result_current = None;
        self.find_in_result(cx);
        cx.notify();
    }

    pub(crate) fn selected_text(&self, window: &Window, cx: &App) -> Option<String> {
        let result = self.result.read(cx);
        (result.focus_handle(cx).is_focused(window) && !result.selected_range().is_empty())
            .then(|| result.selected_text().to_string())
    }

    pub(crate) fn found(&self, cx: &App) -> Found {
        let (count, current) = self
            .versions
            .as_ref()
            .map_or((0, None), |versions| {
                let state = versions.read(cx);
                (state.found_count(), state.current_found())
            });
        Found {
            count: count + self.result_found.len(),
            current: current.or(self.result_current.map(|ix| count + ix)),
        }
    }

    pub(crate) fn step_found(&mut self, forward: bool, cx: &mut Context<Self>) {
        let found = self.found(cx);
        if found.count == 0 {
            return;
        }
        let next = match found.current {
            Some(at) if forward => (at + 1) % found.count,
            Some(at) => (at + found.count - 1) % found.count,
            None if forward => 0,
            None => found.count - 1,
        };
        self.show_found(next, cx);
    }

    fn show_found(&mut self, ix: usize, cx: &mut Context<Self>) {
        let in_versions = self
            .versions
            .as_ref()
            .map_or(0, |versions| versions.read(cx).found_count());
        if let Some(versions) = &self.versions {
            versions.update(cx, |state, cx| {
                if ix < in_versions {
                    state.show_found(ix, ROWS_ABOVE_A_CHANGE, cx);
                } else {
                    state.clear_current_found(cx);
                }
            });
        }
        self.result_current = ix.checked_sub(in_versions);
        if let Some(range) = self.result_current.and_then(|ix| self.result_found.get(ix)).cloned() {
            let line = self.result.read(cx).value()[..range.start].matches('\n').count();
            self.result.update(cx, |state, cx| state.set_selected_range(range, cx));
            self.scroll_result_to(line, cx);
        }
        self.paint_found(cx);
        cx.notify();
    }

    fn find_in_result(&mut self, cx: &mut Context<Self>) {
        let text = self.result.read(cx).value();
        let mut found = Vec::new();
        if let Some(matcher) = &self.matcher {
            let mut start = 0;
            for line in text.split_inclusive('\n') {
                let content = line.strip_suffix('\n').unwrap_or(line);
                found.extend(
                    matcher
                        .ranges(content)
                        .into_iter()
                        .map(|range| range.start + start..range.end + start),
                );
                start += line.len();
            }
        }
        self.result_current = self.result_current.filter(|ix| *ix < found.len());
        self.result_found = found;
        self.paint_found(cx);
    }

    fn paint_found(&mut self, cx: &mut Context<Self>) {
        let color = orange_400();
        let fills = self
            .result_found
            .iter()
            .enumerate()
            .flat_map(|(ix, range)| {
                let current = Some(ix) == self.result_current;
                let tint = if current { CURRENT_FOUND_TINT } else { FOUND_TINT };
                let fill = RangeDecoration::new(range.clone())
                    .with_style(RangeDecorationStyle::Fill)
                    .with_color(color.opacity(tint));
                let frame = current.then(|| {
                    RangeDecoration::new(range.clone())
                        .with_style(RangeDecorationStyle::Frame)
                        .with_color(color)
                });
                std::iter::once(fill).chain(frame)
            })
            .collect();
        self.found_fills.set(fills, cx);
    }

    pub(crate) fn step(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.total();
        if count == 0 {
            return;
        }
        let next = match (self.stepped, direction) {
            (None, Direction::Forward) => 0,
            (None, Direction::Back) => count - 1,
            (Some(at), Direction::Forward) => (at + 1) % count,
            (Some(at), Direction::Back) => (at + count - 1) % count,
        };
        self.show_conflict(next, window, cx);
    }

    fn show_conflict(&mut self, region: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.stepped = Some(region);
        if let Some(versions) = &self.versions {
            let mark = versions.read(cx).change_marks().get(region).copied();
            if let Some(mark) = mark {
                let row = mark.first_row.saturating_sub(ROWS_ABOVE_A_CHANGE);
                versions.update(cx, |state, cx| state.scroll_to_row(row, cx));
            }
        }
        let line = self.merge.line_of(region) as u32;
        self.result.update(cx, |state, cx| {
            state.set_cursor_position(Position::new(line, 0), window, cx)
        });
        self.reveal(region, cx);
        cx.notify();
    }

    fn reveal(&self, region: usize, cx: &mut Context<Self>) {
        self.scroll_result_to(self.merge.line_of(region), cx);
    }

    fn scroll_result_to(&self, line: usize, cx: &mut Context<Self>) {
        let line = line.saturating_sub(ROWS_ABOVE_A_CONFLICT);
        let fallback = cx.theme().mono_font_size * EDITOR_LINE_HEIGHT;

        self.result.update(cx, |state, cx| {
            let height = state.line_height().unwrap_or(fallback);
            state.set_scroll_offset(point(px(0.), -(height * line as f32)), cx);
        });
    }

    fn toggle(&mut self, region: usize, version: Version, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.merge.toggle(region, version) else {
            return;
        };
        self.result.update(cx, |state, cx| {
            state.set_selected_range(edit.range.clone(), cx);
            state.replace(edit.text, window, cx);
        });
        self.touched = true;
        self.stepped = Some(region);
        self.reveal(region, cx);
        self.paint(cx);
        cx.notify();
    }

    fn start_over(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(merge) = MergeText::new(&self.original) {
            self.merge = merge;
        }
        let original = self.original.clone();
        self.result
            .update(cx, |state, cx| state.set_value(original, window, cx));
        self.touched = false;
        self.paint(cx);
        cx.notify();
    }

    fn paint(&mut self, cx: &mut Context<Self>) {
        let theme = cx.theme();
        let color = |tint: Tint| match tint {
            Tint::Marker => theme.magenta,
            Tint::Current => theme.green,
            Tint::Incoming => theme.blue,
            Tint::ByHand => theme.yellow,
        };
        let opacity = |tint: Tint| match tint {
            Tint::Marker => MARKER_TINT,
            _ => LINE_TINT,
        };
        let tints = self.merge.tints();
        let fills = tints
            .iter()
            .map(|(range, tint)| {
                RangeDecoration::new(range.clone())
                    .with_style(RangeDecorationStyle::Fill)
                    .with_color(color(*tint).opacity(opacity(*tint)))
            })
            .collect();
        let bars = tints
            .iter()
            .map(|(range, tint)| GutterDecoration::new(range.clone()).with_color(color(*tint)))
            .collect();
        self.tints.set(fills, cx);
        self.bars.set(bars, cx);
    }

    fn decorations(&self, cx: &App) -> Vec<DiffDecoration> {
        let theme = cx.theme();
        let path = self.change.path.clone();
        self.spans
            .iter()
            .flat_map(|span| {
                [
                    span.current.map(|(start, end)| (DiffSide::Original, start, end, theme.green)),
                    span.incoming.map(|(start, end)| (DiffSide::Modified, start, end, theme.blue)),
                ]
            })
            .flatten()
            .map(|(side, start, end, color)| {
                DiffDecoration::new(DiffLineRange::new(path.clone(), side, start, end))
                    .with_background(color.opacity(LINE_TINT))
            })
            .collect()
    }

    fn version_heads(&self, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        let head = |version: Version, color: Hsla, title: &'static str, label: SharedString| {
            h_flex()
                .flex_1()
                .min_w_0()
                .h(px(HEADER_HEIGHT))
                .px_3()
                .gap_2()
                .xs_text()
                .child(div().flex_none().size(px(DOT)).rounded_full().bg(color))
                .child(div().flex_none().font_medium().child(title))
                .child(
                    div()
                        .min_w_0()
                        .ellipsis()
                        .text_color(theme.muted_foreground)
                        .child(label),
                )
                .when(version == Version::Incoming, |this| {
                    this.border_l_1().border_color(theme.border)
                })
        };
        h_flex()
            .flex_none()
            .border_b_1()
            .border_color(theme.border)
            .child(head(Version::Current, theme.green, "Current", self.labels[0].clone()))
            .child(head(Version::Incoming, theme.blue, "Incoming", self.labels[1].clone()))
    }

    fn versions_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = match &self.versions {
            Some(versions) => {
                let shapes: Rc<[Shape]> = self.merge.regions().iter().map(|region| region.shape()).collect();
                let editor = cx.entity().downgrade();
                Diff::new(versions)
                    .line_number(true)
                    .soft_wrap(TextOptions::current(cx).wraps())
                    .header_visible(false)
                    .hunk_separator(DiffHunkSeparator::LineInfo)
                    .change_background(false)
                    .change_indicator(DiffChangeIndicator::None)
                    .decorations(self.decorations(cx))
                    .render_hunk_suffix(move |_, hunk, _, _| {
                        choices(editor.clone(), hunk, shapes.get(hunk).copied())
                    })
                    .border_0()
                    .into_any_element()
            }
            None => ui::hint("No conflict markers are left in this file.", cx),
        };
        v_flex()
            .size_full()
            .child(self.version_heads(cx))
            .child(div().flex_1().min_h_0().child(content))
            .into_any_element()
    }

    fn result_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let left = self.merge.conflicts_left();
        let state = if self.merge.is_resolved() {
            h_flex()
                .gap_2()
                .child(div().text_color(theme.green).child("No conflicts left"))
        } else {
            h_flex()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .flex_none()
                        .text_color(theme.magenta)
                        .child(format!("{} left", counted(left.max(1), "conflict", "conflicts"))),
                )
                .child(
                    div()
                        .min_w_0()
                        .ellipsis()
                        .text_color(theme.muted_foreground)
                        .child("Tick a side above, or edit here"),
                )
        };
        let header = h_flex()
            .flex_none()
            .h(px(HEADER_HEIGHT))
            .px_3()
            .gap_2()
            .xs_text()
            .border_b_1()
            .border_color(theme.border)
            .child(div().flex_none().font_medium().child("Result"))
            .child(div().flex_1().min_w_0().child(state))
            .child(
                Button::new("merge-start-over")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Undo2)
                    .label("Start over")
                    .tooltip("Put the conflict markers back and forget every choice and edit")
                    .disabled(!self.touched)
                    .on_click(cx.listener(|this, _, window, cx| this.start_over(window, cx))),
            );
        v_flex()
            .size_full()
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Editor::new(&self.result).bordered(false).h(relative(1.))),
            )
            .into_any_element()
    }
}

fn choices(editor: WeakEntity<MergeEditor>, hunk: usize, shape: Option<Shape>) -> AnyElement {
    let shape = shape.unwrap_or(Shape::ByHand);
    let taken = match shape {
        Shape::Taken(taken) => taken,
        Shape::Markers | Shape::ByHand => Default::default(),
    };
    let by_hand = shape == Shape::ByHand;
    let choice = |version: Version, id: &'static str, label: &'static str| {
        let editor = editor.clone();
        Checkbox::new((id, hunk))
            .label(label)
            .checked(taken.has(version))
            .disabled(by_hand)
            .tooltip(if by_hand {
                "Edited by hand in the result: Start over to choose again"
            } else {
                match version {
                    Version::Current => "Put these lines in the result",
                    Version::Incoming => {
                        "Put these lines in the result, after the current ones when both are ticked"
                    }
                }
            })
            .on_click(move |_, window, cx| {
                let _ = editor.update(cx, |editor, cx| editor.toggle(hunk, version, window, cx));
            })
    };
    h_flex()
        .gap_4()
        .child(choice(Version::Current, "accept-current", "Accept current"))
        .child(choice(Version::Incoming, "accept-incoming", "Accept incoming"))
        .into_any_element()
}

impl Render for MergeEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let versions = self.versions_view(cx);
        let result = self.result_view(cx);
        v_resizable("merge-editor")
            .child(resizable_panel().child(versions))
            .child(resizable_panel().child(result))
    }
}
