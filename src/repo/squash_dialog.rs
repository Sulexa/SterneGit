use gix::ObjectId;
use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        dialog::{DialogAction, DialogClose, DialogDescription, DialogFooter},
        h_flex,
        input::{Enter, Input, InputEvent, InputState, Textarea, TextareaState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::repository::Repository;
use crate::{
    git::{RewordScope, commit_message, short_id},
    ui::{Look as _, counted, listed, thousands},
};

const WIDTH: f32 = 460.;
const DESCRIPTION_ROWS: usize = 4;
pub(crate) const NOT_SQUASHED: &str = "Couldn't squash the commits";

pub(crate) fn squashed(count: usize, now: ObjectId) -> String {
    format!("Squashed {} commits into {}", thousands(count), short_id(now))
}

struct SquashForm {
    repository: Entity<Repository>,
    newest: ObjectId,
    oldest: ObjectId,
    count: usize,
    scope: RewordScope,
    summary: Entity<InputState>,
    description: Entity<TextareaState>,
    _edits: Subscription,
}

impl SquashForm {
    fn message(&self, cx: &App) -> Option<String> {
        commit_message(&self.summary.read(cx).value(), &self.description.read(cx).value())
    }

    fn squash(&self, cx: &mut App) -> bool {
        let Some(message) = self.message(cx) else {
            return false;
        };
        let (newest, oldest, count) = (self.newest, self.oldest, self.count);
        self.repository.update(cx, |repository, cx| {
            repository.squash(newest, oldest, count, message, cx)
        });
        true
    }
}

impl Render for SquashForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, yellow) = (theme.muted_foreground, theme.yellow);
        let notes = Notes::of(&self.scope);
        v_flex()
            .gap_3()
            .mt_3()
            .capture_action(cx.listener(|this, enter: &Enter, window, cx| {
                if enter.secondary {
                    cx.stop_propagation();
                    if this.squash(cx) {
                        window.close_dialog(cx);
                    }
                }
            }))
            .child(Input::new(&self.summary))
            .child(Textarea::new(&self.description))
            .child(
                v_flex()
                    .gap_1p5()
                    .child(div().xs_text().text_color(muted).child(notes.moves))
                    .when_some(notes.pushed, |this, pushed| {
                        this.child(
                            h_flex()
                                .gap_1p5()
                                .xs_text()
                                .text_color(yellow)
                                .child(Icon::new(IconName::TriangleAlert).xsmall())
                                .child(pushed),
                        )
                    })
                    .when_some(notes.tags, |this, tags| {
                        this.child(div().xs_text().text_color(muted).child(tags))
                    }),
            )
    }
}

struct Notes {
    moves: String,
    pushed: Option<String>,
    tags: Option<String>,
}

impl Notes {
    fn of(scope: &RewordScope) -> Self {
        let branches = listed(&scope.branches);
        let moves = match scope.after {
            0 => format!("Squashing moves {branches} to the new commit."),
            after => format!(
                "Squashing gives the {} after them new hashes, and moves {branches}.",
                counted(after, "commit", "commits")
            ),
        };
        let pushed = (!scope.remotes.is_empty()).then(|| {
            let has = if scope.remotes.len() > 1 { "have" } else { "has" };
            format!("{} already {has} them: the next push needs Force push.", listed(&scope.remotes))
        });
        let tags = (!scope.tags.is_empty()).then(|| match scope.tags.len() {
            1 => format!("{} stays on the old commit.", listed(&scope.tags)),
            _ => format!("{} stay on the old commits.", listed(&scope.tags)),
        });
        Self { moves, pushed, tags }
    }
}

fn prefilled(oldest_first: &[String]) -> (String, String) {
    let Some((oldest, newer)) = oldest_first.split_first() else {
        return (String::new(), String::new());
    };
    let (subject, body) = oldest.split_once('\n').unwrap_or((oldest, ""));
    let description = std::iter::once(body)
        .chain(newer.iter().map(String::as_str))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    (subject.trim().to_owned(), description)
}

fn title(count: usize) -> String {
    format!("Squash {} commits into one", thousands(count))
}

fn description(newest: ObjectId, oldest: ObjectId) -> String {
    format!(
        "One commit takes their place, with the files of {}, the author and date of {}, and this message.",
        short_id(newest),
        short_id(oldest)
    )
}

pub(crate) fn open_squash_dialog(
    repository: Entity<Repository>,
    newest_first: Vec<ObjectId>,
    window: &mut Window,
    cx: &mut App,
) {
    let (line, scope, messages) = {
        let repository = repository.read(cx);
        let (Some(history), Some(repo)) = (repository.history(), repository.repo()) else {
            return;
        };
        let Some(line) = history.line_of(&newest_first) else {
            return;
        };
        let Some(scope) = history.squash_scope(&line, &newest_first) else {
            return;
        };
        let messages: Vec<String> = newest_first
            .iter()
            .rev()
            .map(|id| repo.message(*id).unwrap_or_default())
            .collect();
        (line, scope, messages)
    };
    let count = newest_first.len();
    let (subject, body) = prefilled(&messages);
    let form = cx.new(|cx| {
        let summary = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Commit message");
            state.set_value(subject, window, cx);
            state
        });
        let description = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .placeholder("Description")
                .rows(DESCRIPTION_ROWS);
            state.set_value(body, window, cx);
            state
        });
        let edits = cx.subscribe(&summary, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify()
            }
        });
        SquashForm {
            repository,
            newest: line.newest,
            oldest: line.oldest,
            count,
            scope,
            summary,
            description,
            _edits: edits,
        }
    });
    let summary = form.read(cx).summary.clone();
    let explained = description(line.newest, line.oldest);
    window.open_dialog(cx, move |dialog, _, cx| {
        let ready = form.read(cx).message(cx).is_some() && !form.read(cx).repository.read(cx).is_writing();
        let form_on_ok = form.clone();
        dialog
            .title(title(count))
            .w(px(WIDTH))
            .child(DialogDescription::new().child(explained.clone()))
            .child(form.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(
                        DialogAction::new().child(
                            Button::new("squash-commits")
                                .primary()
                                .label(format!("Squash {} commits", thousands(count)))
                                .disabled(!ready),
                        ),
                    ),
            )
            .on_ok(move |_, _, cx| form_on_ok.update(cx, |form, cx| form.squash(cx)))
    });
    window.defer(cx, move |window, cx| {
        summary.update(cx, |state, cx| state.focus(window, cx))
    });
}

#[cfg(test)]
mod tests {
    use super::{Notes, prefilled, title};
    use crate::git::RewordScope;

    fn names(names: &[&str]) -> Vec<gpui_kit::SharedString> {
        names.iter().map(|name| (*name).to_owned().into()).collect()
    }

    #[test]
    fn the_message_starts_from_the_oldest_subject_then_every_other_message_oldest_first() {
        let messages = [
            "Add b\n\nWith a body.".to_owned(),
            "Add c".to_owned(),
            "Add d\n\nAnd its body.".to_owned(),
        ];
        assert_eq!(
            prefilled(&messages),
            ("Add b".to_owned(), "With a body.\n\nAdd c\n\nAdd d\n\nAnd its body.".to_owned())
        );
        assert_eq!(prefilled(&["Add b".to_owned(), "Add c".to_owned()]), ("Add b".to_owned(), "Add c".to_owned()));
        assert_eq!(title(2), "Squash 2 commits into one");
    }

    #[test]
    fn the_notes_say_what_moves_what_was_pushed_and_which_tags_stay() {
        let notes = Notes::of(&RewordScope {
            after: 0,
            branches: names(&["main"]),
            remotes: Vec::new(),
            tags: Vec::new(),
        });
        assert_eq!(notes.moves, "Squashing moves main to the new commit.");
        assert_eq!((notes.pushed, notes.tags), (None, None));
        let notes = Notes::of(&RewordScope {
            after: 2,
            branches: names(&["feature/diff-view", "topic"]),
            remotes: names(&["origin/feature/diff-view"]),
            tags: names(&["v0.1.1"]),
        });
        assert_eq!(
            notes.moves,
            "Squashing gives the 2 commits after them new hashes, and moves feature/diff-view and topic."
        );
        assert_eq!(
            notes.pushed.as_deref(),
            Some("origin/feature/diff-view already has them: the next push needs Force push.")
        );
        assert_eq!(notes.tags.as_deref(), Some("v0.1.1 stays on the old commit."));
    }
}
