use std::sync::Arc;

use gpui_kit::{
    AnyElement, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{
        Disableable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        diff::{DiffLinePosition, DiffLineRange, DiffSide},
        h_flex,
        menu::{PopupMenu, PopupMenuItem},
    },
};

use crate::{
    git::{Hunks, LineAt, LineSide, LineSpan, Part, PartAction, Side},
    ui::{Handler, counted},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pressed {
    Line(LineAt),
    Hunk(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    Hunk,
    Lines(usize),
    Line,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Offer {
    action: PartAction,
    reach: Reach,
    part: Part,
}

impl Offer {
    fn label(&self) -> String {
        let verb = verb(self.action);
        match self.reach {
            Reach::Hunk => format!("{verb} hunk"),
            Reach::Lines(n) => format!("{verb} {}", counted(n, "line", "lines")),
            Reach::Line => format!("{verb} this line"),
        }
    }

    fn tooltip(&self) -> &'static str {
        match (self.action, self.reach) {
            (PartAction::Stage, Reach::Hunk) => "Stage this hunk",
            (PartAction::Unstage, Reach::Hunk) => "Unstage this hunk",
            (PartAction::Discard, Reach::Hunk) => "Throw away this hunk",
            (PartAction::Stage, _) => "Stage the selected lines",
            (PartAction::Unstage, _) => "Unstage the selected lines",
            (PartAction::Discard, _) => "Throw away the selected lines",
        }
    }
}

#[derive(Clone)]
pub(crate) struct Offers {
    hunks: Arc<Hunks>,
    side: Side,
    selection: Option<LineSpan>,
}

impl Offers {
    pub(crate) fn new(hunks: Arc<Hunks>, side: Side, selection: Option<LineSpan>) -> Self {
        Self {
            hunks,
            side,
            selection,
        }
    }

    fn actions(&self) -> &'static [PartAction] {
        match self.side {
            Side::Unstaged => &[PartAction::Stage, PartAction::Discard],
            Side::Staged => &[PartAction::Unstage],
        }
    }

    fn each_action(&self, reach: Reach, part: &Part) -> Vec<Offer> {
        self.actions()
            .iter()
            .map(|&action| Offer {
                action,
                reach,
                part: part.clone(),
            })
            .collect()
    }

    fn for_hunk(&self, hunk: usize) -> Vec<Offer> {
        let selected = self
            .selection
            .map(|span| self.hunks.within(span).in_hunk(hunk))
            .unwrap_or_default();
        if selected.is_empty() {
            self.each_action(Reach::Hunk, &self.hunks.hunk(hunk))
        } else {
            self.each_action(Reach::Lines(selected.len()), &selected)
        }
    }

    fn for_press(&self, pressed: Pressed) -> Vec<Vec<Offer>> {
        match pressed {
            Pressed::Line(at) => self.for_line(at),
            Pressed::Hunk(hunk) if hunk < self.hunks.len() => vec![self.for_hunk(hunk)],
            Pressed::Hunk(_) => Vec::new(),
        }
    }

    fn for_line(&self, at: LineAt) -> Vec<Vec<Offer>> {
        let Some(hunk) = self.hunks.hunk_at(at) else {
            return Vec::new();
        };
        let whole = self.each_action(Reach::Hunk, &self.hunks.hunk(hunk));
        let selected = self
            .selection
            .filter(|span| self.hunks.covers(*span, at))
            .map(|span| self.hunks.within(span))
            .filter(|part| !part.is_empty());
        if let Some(selected) = selected {
            return vec![
                self.each_action(Reach::Lines(selected.len()), &selected),
                whole,
            ];
        }
        let line = self.hunks.line_at(at);
        if line.is_empty() {
            vec![whole]
        } else {
            vec![whole, self.each_action(Reach::Line, &line)]
        }
    }
}

#[derive(Clone)]
pub(crate) struct HunkActions {
    offers: Offers,
    enabled: bool,
    on_apply: Handler<(PartAction, Part)>,
}

impl HunkActions {
    pub(crate) fn new(
        offers: Offers,
        enabled: bool,
        on_apply: Handler<(PartAction, Part)>,
    ) -> Self {
        Self {
            offers,
            enabled,
            on_apply,
        }
    }

    pub(crate) fn buttons(&self, hunk: usize) -> AnyElement {
        let offers = if hunk < self.offers.hunks.len() {
            self.offers.for_hunk(hunk)
        } else {
            Vec::new()
        };
        h_flex()
            .gap_1()
            .children(
                offers
                    .into_iter()
                    .rev()
                    .map(|offer| self.button(hunk, offer)),
            )
            .into_any_element()
    }

    pub(crate) fn menu(&self, menu: PopupMenu, pressed: Option<Pressed>) -> PopupMenu {
        let groups = pressed
            .map(|pressed| self.offers.for_press(pressed))
            .unwrap_or_default();
        groups
            .into_iter()
            .enumerate()
            .fold(menu, |menu, (ix, group)| {
                let menu = if ix == 0 { menu } else { menu.separator() };
                group
                    .into_iter()
                    .fold(menu, |menu, offer| menu.item(self.menu_item(offer)))
            })
    }

    fn button(&self, hunk: usize, offer: Offer) -> Button {
        let on_apply = self.on_apply.clone();
        Button::new((verb(offer.action), hunk))
            .ghost()
            .xsmall()
            .icon(icon(offer.action))
            .label(offer.label())
            .tooltip(offer.tooltip())
            .disabled(!self.enabled)
            .on_click(move |_, window, cx| {
                on_apply(&(offer.action, offer.part.clone()), window, cx)
            })
    }

    fn menu_item(&self, offer: Offer) -> PopupMenuItem {
        let on_apply = self.on_apply.clone();
        PopupMenuItem::new(offer.label())
            .disabled(!self.enabled)
            .on_click(move |_, window, cx| {
                on_apply(&(offer.action, offer.part.clone()), window, cx)
            })
    }
}

pub(crate) fn line_at(position: &DiffLinePosition) -> LineAt {
    LineAt {
        side: line_side(position.side()),
        line: position.line(),
    }
}

pub(crate) fn line_span(range: &DiffLineRange) -> LineSpan {
    LineSpan {
        from: LineAt {
            side: line_side(range.side()),
            line: range.start(),
        },
        to: LineAt {
            side: line_side(range.end_side()),
            line: range.end(),
        },
    }
}

fn line_side(side: DiffSide) -> LineSide {
    match side {
        DiffSide::Original => LineSide::Old,
        DiffSide::Modified => LineSide::New,
    }
}

fn verb(action: PartAction) -> &'static str {
    match action {
        PartAction::Stage => "Stage",
        PartAction::Unstage => "Unstage",
        PartAction::Discard => "Discard",
    }
}

fn icon(action: PartAction) -> IconName {
    match action {
        PartAction::Stage => IconName::Plus,
        PartAction::Unstage => IconName::Minus,
        PartAction::Discard => IconName::Undo2,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Offer, Offers, Pressed};
    use crate::git::{Hunks, LineAt, LineSide, LineSpan, Side};

    const PATCH: &str = "diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
@@ -1,4 +1,5 @@
 one
-two
+TWO
+two and a half
 three
 four
@@ -10,3 +11,3 @@
 ten
-eleven
+ELEVEN
 twelve
";

    fn new(line: usize) -> LineAt {
        LineAt {
            side: LineSide::New,
            line,
        }
    }

    fn offers(side: Side, selection: Option<LineSpan>) -> Offers {
        let hunks = Hunks::parse(PATCH.as_bytes()).expect("the patch parses");
        Offers::new(Arc::new(hunks), side, selection)
    }

    fn labels(offers: &[Offer]) -> Vec<String> {
        offers.iter().map(Offer::label).collect()
    }

    fn groups(groups: &[Vec<Offer>]) -> Vec<Vec<String>> {
        groups.iter().map(|group| labels(group)).collect()
    }

    #[test]
    fn each_hunk_offers_to_stage_or_discard_it() {
        let unstaged = offers(Side::Unstaged, None);
        assert_eq!(
            labels(&unstaged.for_hunk(0)),
            ["Stage hunk", "Discard hunk"]
        );
        let staged = offers(Side::Staged, None);
        assert_eq!(labels(&staged.for_hunk(1)), ["Unstage hunk"]);
    }

    #[test]
    fn selected_lines_replace_the_hunk_they_are_in() {
        let selection = LineSpan {
            from: new(2),
            to: new(3),
        };
        let unstaged = offers(Side::Unstaged, Some(selection));
        assert_eq!(
            labels(&unstaged.for_hunk(0)),
            ["Stage 2 lines", "Discard 2 lines"]
        );
        assert_eq!(
            labels(&unstaged.for_hunk(1)),
            ["Stage hunk", "Discard hunk"]
        );
        assert_eq!(
            groups(&unstaged.for_line(new(3))),
            [
                vec!["Stage 2 lines", "Discard 2 lines"],
                vec!["Stage hunk", "Discard hunk"]
            ]
        );
    }

    #[test]
    fn a_changed_line_offers_its_hunk_then_itself() {
        let staged = offers(Side::Staged, None);
        assert_eq!(
            groups(&staged.for_line(new(12))),
            [vec!["Unstage hunk"], vec!["Unstage this line"]]
        );
        assert_eq!(groups(&staged.for_line(new(1))), [vec!["Unstage hunk"]]);
        assert!(staged.for_line(new(7)).is_empty());
    }

    #[test]
    fn a_hunk_header_offers_what_its_buttons_offer() {
        let selection = LineSpan {
            from: new(2),
            to: new(2),
        };
        let unstaged = offers(Side::Unstaged, Some(selection));
        assert_eq!(
            groups(&unstaged.for_press(Pressed::Hunk(0))),
            [vec!["Stage 1 line", "Discard 1 line"]]
        );
        assert_eq!(
            groups(&unstaged.for_press(Pressed::Hunk(1))),
            [vec!["Stage hunk", "Discard hunk"]]
        );
        assert!(unstaged.for_press(Pressed::Hunk(2)).is_empty());
    }

    #[test]
    fn one_selected_line_is_counted_in_the_singular() {
        let selection = LineSpan {
            from: new(12),
            to: new(12),
        };
        let unstaged = offers(Side::Unstaged, Some(selection));
        assert_eq!(
            labels(&unstaged.for_hunk(1)),
            ["Stage 1 line", "Discard 1 line"]
        );
    }
}
