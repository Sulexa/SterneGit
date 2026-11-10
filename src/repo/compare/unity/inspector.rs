use std::ops::Range;

use gpui_kit::{
    AnyElement, App, Context, Div, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div, list,
    prelude::FluentBuilder as _,
    px, uniform_list,
};

use super::{
    UnityCompare,
    cells::{cell_tint, hatched, state_badge, value_cell},
    guid,
};
use crate::{
    repo::compare::{Version, VersionTag},
    ui::{Look as _, ROW_HEIGHT, counted, list_row},
    unity::{Anchor, ObjectChange, PartChange, Row, State, TreeRow},
};

const TREE_WIDTH: f32 = 232.;
const PROPERTY_WIDTH: f32 = 176.;
const SIDES_HEIGHT: f32 = 32.;
const OBJECT_HEIGHT: f32 = 34.;
const PART_HEIGHT: f32 = 26.;
const INDENT: f32 = 16.;
const PART_INDENT: f32 = 28.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flat {
    Object(usize),
    Part(usize, usize),
    Row(usize, usize, usize),
}

impl From<Anchor> for Flat {
    fn from(anchor: Anchor) -> Self {
        match (anchor.part, anchor.row) {
            (Some(part), Some(row)) => Flat::Row(anchor.object, part, row),
            (Some(part), None) => Flat::Part(anchor.object, part),
            _ => Flat::Object(anchor.object),
        }
    }
}

fn is_open(compare: &UnityCompare, object: &ObjectChange) -> bool {
    (object.state != State::Removed) ^ compare.unfolded.contains(&object.key)
}

fn flat_rows(compare: &UnityCompare) -> Vec<Flat> {
    let mut rows = Vec::new();
    for (object_ix, object) in compare.comparison.objects.iter().enumerate() {
        rows.push(Flat::Object(object_ix));
        if !is_open(compare, object) {
            continue;
        }
        for (part_ix, part) in object.parts.iter().enumerate() {
            rows.push(Flat::Part(object_ix, part_ix));
            rows.extend((0..part.rows.len()).map(|row_ix| Flat::Row(object_ix, part_ix, row_ix)));
        }
    }
    rows
}

pub(super) fn relist(compare: &mut UnityCompare) {
    let top = compare.rows.logical_scroll_top();
    compare.flat = flat_rows(compare);
    compare.rows.reset(compare.flat.len());
    compare.rows.scroll_to(top);
}

pub(super) fn refold(compare: &mut UnityCompare) {
    let flat = flat_rows(compare);
    let (replaced, count) = changed_span(&compare.flat, &flat);
    compare.rows.splice(replaced, count);
    compare.flat = flat;
}

fn changed_span(old: &[Flat], new: &[Flat]) -> (Range<usize>, usize) {
    let start = old.iter().zip(new).take_while(|(old, new)| old == new).count();
    let end = old[start..].iter().rev().zip(new[start..].iter().rev()).take_while(|(old, new)| old == new).count();
    (start..old.len() - end, new.len() - start - end)
}

pub(super) fn flat_index(compare: &UnityCompare, anchor: Anchor) -> Option<usize> {
    let wanted = Flat::from(anchor);
    compare.flat.iter().position(|row| *row == wanted)
}

fn current_flat(compare: &UnityCompare) -> Option<Flat> {
    compare.comparison.anchors.get(compare.current).copied().map(Flat::from)
}

pub(super) fn sides_row(sources: &[SharedString; 2], property: f32, cx: &App) -> AnyElement {
    let border = cx.theme().border;
    let half = || h_flex().flex_1().min_w_0().h_full().px_2().border_l_1().border_color(border);
    h_flex()
        .flex_none()
        .w_full()
        .h(px(SIDES_HEIGHT))
        .border_b_1()
        .border_color(border)
        .child(
            div()
                .flex_none()
                .w(px(property))
                .pl_3()
                .xs_text()
                .text_color(cx.theme().muted_foreground)
                .child("Property"),
        )
        .child(half().child(VersionTag::new(Version::Before, sources[0].clone())))
        .child(half().child(VersionTag::new(Version::After, sources[1].clone())))
        .into_any_element()
}

pub(super) fn property_row(row: &Row, property: f32, indent: f32, current: bool, cx: &App) -> Div {
    let theme = cx.theme();
    let cell = |value: Option<&crate::unity::Value>, other: Option<&crate::unity::Value>, version: Version| {
        let base = v_flex()
            .flex_1()
            .min_w_0()
            .justify_center()
            .items_start()
            .overflow_hidden()
            .px_2()
            .py_1()
            .border_l_1()
            .border_color(theme.border);
        match value {
            None => base.bg(hatched(cx)),
            Some(value) => {
                let tinted = row.state != State::Same;
                base.when(tinted, |this| this.bg(cell_tint(version, cx)))
                    .child(value_cell(value, other.filter(|_| row.state == State::Changed), version, cx))
            }
        }
    };
    h_flex()
        .w_full()
        .min_h(ROW_HEIGHT)
        .sm_text()
        .when(current, |this| this.border_l_2().border_color(theme.blue))
        .child(
            div()
                .flex_none()
                .w(px(property))
                .pl(px(indent + row.depth as f32 * INDENT))
                .pr_2()
                .ellipsis()
                .text_color(theme.muted_foreground)
                .child(row.label.clone()),
        )
        .child(cell(row.before.as_ref(), row.after.as_ref(), Version::Before))
        .child(cell(row.after.as_ref(), row.before.as_ref(), Version::After))
}

pub(super) fn part_head(part: &PartChange, indent: f32, cx: &App) -> Div {
    let theme = cx.theme();
    h_flex()
        .w_full()
        .h(px(PART_HEIGHT))
        .pl(px(indent))
        .pr_3()
        .gap_1p5()
        .border_t_1()
        .border_color(theme.border)
        .when(part.overrides, |this| this.border_l_2().border_color(theme.blue))
        .when(part.script.is_some(), |this| {
            this.child(Icon::new(IconName::FileCode).small().text_color(theme.muted_foreground))
        })
        .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(part.title.clone()))
        .when(part.script.is_some(), |this| this.child(div().xs_text().text_color(theme.muted_foreground).child("(Script)")))
        .when(part.overrides, |this| {
            this.child(
                div()
                    .px_1()
                    .rounded(px(3.))
                    .text_size(px(10.))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .text_color(theme.blue)
                    .bg(theme.blue.opacity(0.14))
                    .child("Override"),
            )
        })
        .children(state_badge(part.state, cx).filter(|_| matches!(part.state, State::Added | State::Removed)))
}

fn object_head(compare: &UnityCompare, object: &ObjectChange, object_ix: usize, current: bool, cx: &mut Context<UnityCompare>) -> AnyElement {
    let theme = cx.theme();
    let open = is_open(compare, object);
    let key = object.key;
    let summary = if !open && object.state == State::Removed {
        let names: Vec<&str> = object.parts.iter().map(|part| part.title.as_str()).collect();
        format!("{}: {}", counted(names.len(), "component", "components"), names.join(", "))
    } else if object.state == State::Changed {
        counted(object.parts.len(), "component changed", "components changed")
    } else {
        counted(object.parts.len(), "component", "components")
    };
    h_flex()
        .id(("unity-object", object_ix))
        .w_full()
        .h(px(OBJECT_HEIGHT))
        .pl_2()
        .pr_3()
        .gap_1p5()
        .bg(theme.secondary.opacity(0.55))
        .when(current, |this| this.border_l_2().border_color(theme.blue))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_unfolded(key, cx)))
        .child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).xsmall().text_color(theme.muted_foreground))
        .children(state_badge(object.state, cx))
        .child(Icon::new(IconName::Box).small().text_color(if object.prefab { theme.blue } else { theme.muted_foreground }))
        .child(
            div()
                .sm_text()
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .when(object.state == State::Removed, |this| this.line_through().text_color(theme.muted_foreground))
                .child(object.name.clone()),
        )
        .when_some(object.parent.clone(), |this, parent| {
            this.child(div().xs_text().text_color(theme.muted_foreground).child(format!("in {parent}")))
        })
        .when_some(object.what.clone(), |this, what| this.child(div().xs_text().ellipsis().text_color(theme.muted_foreground).child(what)))
        .child(div().flex_1())
        .child(div().flex_none().xs_text().text_color(theme.muted_foreground).child(summary))
        .into_any_element()
}

fn visible_tree(compare: &UnityCompare) -> Vec<&TreeRow> {
    let tree = &compare.comparison.tree;
    let mut shown = vec![true; tree.len()];
    if compare.changed_only {
        let mut keep = vec![false; tree.len()];
        for ix in 0..tree.len() {
            if tree[ix].state == State::Same {
                continue;
            }
            keep[ix] = true;
            let mut depth = tree[ix].depth;
            let mut at = ix;
            while depth > 0 && at > 0 {
                at -= 1;
                if tree[at].depth < depth {
                    keep[at] = true;
                    depth = tree[at].depth;
                }
            }
        }
        shown = keep;
    }
    let mut out = Vec::new();
    let mut hidden_below: Option<usize> = None;
    for (ix, row) in tree.iter().enumerate() {
        if let Some(depth) = hidden_below {
            if row.depth > depth {
                continue;
            }
            hidden_below = None;
        }
        if !shown[ix] {
            continue;
        }
        out.push(row);
        if compare.collapsed.contains(&row.key) {
            hidden_below = Some(row.depth);
        }
    }
    out
}

fn tree_rows(compare: &UnityCompare, range: Range<usize>, cx: &mut Context<UnityCompare>) -> Vec<AnyElement> {
    let theme = cx.theme();
    let muted = theme.muted_foreground;
    visible_tree(compare)
        .into_iter()
        .skip(range.start)
        .take(range.len())
        .map(|row| {
            let key = row.key;
            let open = !compare.collapsed.contains(&key);
            let twisty = if row.has_children {
                div()
                    .id(("unity-fold", key as u64))
                    .flex_none()
                    .child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).xsmall().text_color(muted))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_tree(key, cx)
                    }))
                    .into_any_element()
            } else {
                div().flex_none().w(px(12.)).into_any_element()
            };
            list_row(("unity-tree", key as u64), compare.picked == Some(key))
                .px_2()
                .child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap_1()
                        .pl(px(row.depth as f32 * INDENT))
                        .child(twisty)
                        .child(Icon::new(IconName::Box).small().text_color(if row.prefab { theme.blue } else { muted }))
                        .child(
                            div()
                                .flex_1()
                                .ellipsis()
                                .sm_text()
                                .when(row.state == State::Same, |this| this.text_color(muted))
                                .when(row.state == State::Removed, |this| this.line_through().text_color(muted))
                                .child(row.name.clone()),
                        )
                        .children(state_badge(row.state, cx)),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.reveal(key, cx)))
                .into_any_element()
        })
        .collect()
}

fn hierarchy(compare: &UnityCompare, cx: &mut Context<UnityCompare>) -> AnyElement {
    let theme = cx.theme();
    let muted = theme.muted_foreground;
    let shown = visible_tree(compare).len();
    let tally = [(State::Added, "added"), (State::Removed, "removed"), (State::Changed, "changed")]
        .iter()
        .map(|(state, word)| (compare.comparison.tree.iter().filter(|row| row.state == *state).count(), *word))
        .filter(|(count, _)| *count > 0)
        .map(|(count, word)| format!("{count} {word}"))
        .collect::<Vec<_>>()
        .join(" · ");
    v_flex()
        .flex_none()
        .w(px(TREE_WIDTH))
        .h_full()
        .border_r_1()
        .border_color(theme.border)
        .bg(theme.sidebar)
        .child(
            h_flex()
                .flex_none()
                .h(px(SIDES_HEIGHT))
                .pl_3()
                .pr_1()
                .gap_2()
                .border_b_1()
                .border_color(theme.border)
                .child(div().flex_1().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child("Hierarchy"))
                .child(
                    Button::new("unity-changed-only")
                        .ghost()
                        .xsmall()
                        .icon(IconName::ListFilter)
                        .label("Changed only")
                        .selected(compare.changed_only)
                        .tooltip("Hide the objects that did not change")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.changed_only = !this.changed_only;
                            cx.notify();
                        })),
                ),
        )
        .child(
            uniform_list("unity-tree", shown, cx.processor(|this, range, _, cx| tree_rows(this, range, cx)))
                .flex_1()
                .min_h_0(),
        )
        .child(
            h_flex()
                .flex_none()
                .h(ROW_HEIGHT)
                .px_3()
                .border_t_1()
                .border_color(theme.border)
                .xs_text()
                .text_color(muted)
                .child(tally),
        )
        .into_any_element()
}

fn listed_row(compare: &UnityCompare, ix: usize, cx: &mut Context<UnityCompare>) -> AnyElement {
    let item = compare.flat[ix];
    let current = current_flat(compare) == Some(item);
    match item {
        Flat::Object(object_ix) => object_head(compare, &compare.comparison.objects[object_ix], object_ix, current, cx),
        Flat::Part(object_ix, part_ix) => {
            let part = &compare.comparison.objects[object_ix].parts[part_ix];
            part_head(part, PART_INDENT, cx).when(current, |this| this.border_l_2().border_color(cx.theme().blue)).into_any_element()
        }
        Flat::Row(object_ix, part_ix, row_ix) => {
            let row = &compare.comparison.objects[object_ix].parts[part_ix].rows[row_ix];
            property_row(row, PROPERTY_WIDTH, PART_INDENT, current, cx).into_any_element()
        }
    }
}

pub(super) fn render(compare: &mut UnityCompare, _: &mut Window, cx: &mut Context<UnityCompare>) -> AnyElement {
    let empty = compare.comparison.objects.is_empty();
    let body = if empty {
        let hidden = compare.comparison.hidden;
        let message = if hidden > 0 {
            format!("Only {} Unity made on its own: they are hidden.", counted(hidden, "change", "changes"))
        } else {
            "No changes to show.".to_owned()
        };
        crate::ui::hint(message, cx)
    } else {
        div()
            .flex_1()
            .min_h_0()
            .child(list(compare.rows.clone(), cx.processor(|this, ix, _, cx| listed_row(this, ix, cx))).size_full())
            .into_any_element()
    };
    let warning = compare.files.guid_change.clone().map(|change| guid::warning(change, cx));
    let inspector = v_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .children(warning)
        .child(sides_row(&compare.sources, PROPERTY_WIDTH, cx))
        .child(body);
    h_flex()
        .size_full()
        .when(!compare.comparison.tree.is_empty(), |this| this.child(hierarchy(compare, cx)))
        .child(inspector)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{Flat, changed_span};

    #[test]
    fn folding_an_object_replaces_only_its_own_rows() {
        let folded = [Flat::Object(0), Flat::Object(1), Flat::Object(2)];
        let unfolded = [Flat::Object(0), Flat::Object(1), Flat::Part(1, 0), Flat::Row(1, 0, 0), Flat::Object(2)];
        assert_eq!(changed_span(&folded, &unfolded), (2..2, 2));
        assert_eq!(changed_span(&unfolded, &folded), (2..4, 0));
        assert_eq!(changed_span(&folded, &folded), (3..3, 0));
    }
}
