use gix::ObjectId;
use gpui_kit::{
    AppContext as _, Context, Entity, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Enter, Escape, Input, InputEvent, InputState, Position, Textarea, TextareaState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use super::repository::{Repository, RepositoryEvent, Reworded};
use crate::{
    actions::CloseDiff,
    git::{RewordScope, commit_message, short_id},
    ui::{Failure, Look as _, counted, listed},
};

const DESCRIPTION_ROWS: usize = 4;
const SAVE: &str = "Save message";
const SAVING: &str = "Saving…";
pub(crate) const NOT_SAVED: &str = "Couldn't save the message";

pub(crate) fn message_saved(old: ObjectId, now: ObjectId) -> SharedString {
    format!("Message saved: {} is now {}", short_id(old), short_id(now)).into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    Summary,
    Description,
}

pub(crate) struct StopEditing;

pub(crate) struct MessageEditor {
    repository: Entity<Repository>,
    id: ObjectId,
    scope: RewordScope,
    summary: Entity<InputState>,
    description: Entity<TextareaState>,
    shown: (SharedString, SharedString),
    saving: bool,
    failure: Option<Failure>,
    _subscriptions: [Subscription; 3],
}

impl EventEmitter<StopEditing> for MessageEditor {}

impl MessageEditor {
    pub(crate) fn new(
        repository: Entity<Repository>,
        id: ObjectId,
        scope: RewordScope,
        shown: (SharedString, SharedString),
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (subject, body) = shown.clone();
        let summary = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Commit message");
            state.set_value(subject, window, cx);
            state
        });
        let end = end_of(&body);
        let description = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx)
                .placeholder("Description")
                .rows(DESCRIPTION_ROWS);
            state.set_value(body, window, cx);
            state
        });
        match field {
            Field::Summary => summary.update(cx, |state, cx| state.focus(window, cx)),
            Field::Description => description
                .update(cx, |state, cx| state.set_cursor_position(end, window, cx)),
        }
        let subscriptions = [
            cx.subscribe_in(&summary, window, |this, _, event, window, cx| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter {
                    secondary: false, ..
                } => this
                    .description
                    .update(cx, |state, cx| state.focus(window, cx)),
                _ => {}
            }),
            cx.subscribe(&description, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify()
                }
            }),
            cx.subscribe(&repository, |this, _, event, cx| {
                if let RepositoryEvent::Reworded(reworded) = event {
                    this.on_reworded(reworded, cx)
                }
            }),
        ];
        Self {
            repository,
            id,
            scope,
            summary,
            description,
            shown,
            saving: false,
            failure: None,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn id(&self) -> ObjectId {
        self.id
    }

    pub(crate) fn rescope(&mut self, scope: RewordScope, cx: &mut Context<Self>) {
        if self.scope != scope {
            self.scope = scope;
            cx.notify();
        }
    }

    fn typed(&self, cx: &Context<Self>) -> (SharedString, SharedString) {
        (
            self.summary.read(cx).value().into(),
            self.description.read(cx).value().into(),
        )
    }

    fn message(&self, cx: &Context<Self>) -> Option<String> {
        let typed = self.typed(cx);
        if typed == self.shown {
            return None;
        }
        commit_message(&typed.0, &typed.1)
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.saving || self.repository.read(cx).is_writing() {
            return;
        }
        let Some(message) = self.message(cx) else {
            return;
        };
        self.saving = true;
        self.failure = None;
        let id = self.id;
        self.repository
            .update(cx, |repository, cx| repository.reword(id, message, cx));
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        if !self.saving {
            cx.emit(StopEditing);
        }
    }

    fn on_reworded(&mut self, reworded: &Reworded, cx: &mut Context<Self>) {
        if reworded.id != self.id {
            return;
        }
        self.saving = false;
        match &reworded.outcome {
            Ok(_) => cx.emit(StopEditing),
            Err(failure) => self.failure = Some(failure.clone()),
        }
        cx.notify();
    }
}

impl Render for MessageEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ready = !self.saving
            && !self.repository.read(cx).is_writing()
            && self.message(cx).is_some();
        let theme = cx.theme();
        let (muted, yellow, red) = (theme.muted_foreground, theme.yellow, theme.red);
        let lines = Lines::of(&self.scope);
        v_flex()
            .gap_2()
            .capture_action(cx.listener(|this, enter: &Enter, _, cx| {
                if enter.secondary {
                    cx.stop_propagation();
                    this.save(cx);
                }
            }))
            .capture_action(cx.listener(|this, _: &Escape, _, cx| {
                cx.stop_propagation();
                this.cancel(cx);
            }))
            .capture_action(cx.listener(|this, _: &CloseDiff, _, cx| {
                cx.stop_propagation();
                this.cancel(cx);
            }))
            .child(Input::new(&self.summary).disabled(self.saving))
            .child(Textarea::new(&self.description).disabled(self.saving))
            .child(div().xs_text().text_color(muted).child(lines.rewrites))
            .when_some(lines.pushed, |this, pushed| {
                this.child(
                    h_flex()
                        .gap_1p5()
                        .xs_text()
                        .text_color(yellow)
                        .child(Icon::new(IconName::TriangleAlert).xsmall())
                        .child(pushed),
                )
            })
            .when_some(lines.tags, |this, tags| {
                this.child(div().xs_text().text_color(muted).child(tags))
            })
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1())
                    .child(
                        Button::new("cancel-message")
                            .small()
                            .label("Cancel")
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                    )
                    .child(
                        Button::new("save-message")
                            .primary()
                            .small()
                            .label(if self.saving { SAVING } else { SAVE })
                            .loading(self.saving)
                            .disabled(!ready && !self.saving)
                            .tooltip_with_action(
                                "Save the message",
                                &Enter {
                                    secondary: true,
                                    shift: false,
                                },
                                Some("Input"),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    ),
            )
            .when_some(self.failure.as_ref(), |this, failure| {
                this.child(div().xs_text().text_color(red).child(failure.message()))
            })
    }
}

fn end_of(text: &str) -> Position {
    let last_line = text.lines().count().saturating_sub(1);
    Position::new(u32::try_from(last_line).unwrap_or(u32::MAX), u32::MAX)
}

struct Lines {
    rewrites: String,
    pushed: Option<String>,
    tags: Option<String>,
}

impl Lines {
    fn of(scope: &RewordScope) -> Self {
        let branches = listed(&scope.branches);
        let rewrites = match scope.after {
            0 => format!("Saving gives this commit a new hash and moves {branches} to it."),
            after => format!(
                "Saving gives this commit and the {} after it new hashes, and moves {branches}.",
                counted(after, "commit", "commits")
            ),
        };
        let pushed = (!scope.remotes.is_empty()).then(|| {
            let has = if scope.remotes.len() > 1 { "have" } else { "has" };
            format!(
                "{} already {has} it: the next push needs Force push.",
                listed(&scope.remotes)
            )
        });
        let tags = (!scope.tags.is_empty()).then(|| match scope.tags.len() {
            1 => format!("{} stays on the old commit.", listed(&scope.tags)),
            _ => format!("{} stay on the old commits.", listed(&scope.tags)),
        });
        Self {
            rewrites,
            pushed,
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Lines, end_of};
    use crate::git::RewordScope;

    fn scope(after: usize, branches: &[&str], remotes: &[&str], tags: &[&str]) -> RewordScope {
        let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned().into()).collect();
        RewordScope {
            after,
            branches: names(branches),
            remotes: names(remotes),
            tags: names(tags),
        }
    }

    #[test]
    fn the_lines_say_what_saving_rewrites() {
        let lines = Lines::of(&scope(1, &["main"], &[], &[]));
        assert_eq!(
            lines.rewrites,
            "Saving gives this commit and the 1 commit after it new hashes, and moves main."
        );
        assert_eq!((lines.pushed, lines.tags), (None, None));
        let lines = Lines::of(&scope(0, &["feature/diff-view"], &["origin/feature/diff-view"], &["v0.2.0"]));
        assert_eq!(lines.rewrites, "Saving gives this commit a new hash and moves feature/diff-view to it.");
        assert_eq!(
            lines.pushed.as_deref(),
            Some("origin/feature/diff-view already has it: the next push needs Force push.")
        );
        assert_eq!(lines.tags.as_deref(), Some("v0.2.0 stays on the old commit."));
        let lines = Lines::of(&scope(9, &["main", "feature/blame", "feature/stash"], &["origin/main", "origin/feature/stash"], &["v0.1.0", "v0.2.0"]));
        assert_eq!(
            lines.rewrites,
            "Saving gives this commit and the 9 commits after it new hashes, and moves main, feature/blame and feature/stash."
        );
        assert_eq!(
            lines.pushed.as_deref(),
            Some("origin/main and origin/feature/stash already have it: the next push needs Force push.")
        );
        assert_eq!(lines.tags.as_deref(), Some("v0.1.0 and v0.2.0 stay on the old commits."));
    }

    #[test]
    fn the_description_cursor_lands_after_its_last_line() {
        assert_eq!(end_of(""), gpui_kit::component::input::Position::new(0, u32::MAX));
        assert_eq!(end_of("one\ntwo"), gpui_kit::component::input::Position::new(1, u32::MAX));
    }
}
