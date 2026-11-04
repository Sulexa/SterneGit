use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, Styled as _, Subscription, Window,
    component::{
        ActiveTheme as _, Disableable as _,
        button::{Button, ButtonVariants as _},
        input::{Enter, Input, InputEvent, InputState, Textarea, TextareaState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use super::repository::{Repository, RepositoryEvent};
use crate::{
    git::commit_message,
    ui::{Failure, Look as _, counted},
};

const DESCRIPTION_ROWS: usize = 4;

pub(crate) struct CommitForm {
    repository: Entity<Repository>,
    summary: Entity<InputState>,
    description: Entity<TextareaState>,
    _subscriptions: [Subscription; 3],
}

impl CommitForm {
    pub(crate) fn new(
        repository: Entity<Repository>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let summary = cx.new(|cx| InputState::new(window, cx).placeholder("Commit message"));
        let description = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Description")
                .rows(DESCRIPTION_ROWS)
        });
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
            cx.subscribe_in(&repository, window, |this, _, event, window, cx| {
                if matches!(event, RepositoryEvent::Committed) {
                    this.clear(window, cx)
                }
            }),
        ];
        Self {
            repository,
            summary,
            description,
            _subscriptions: subscriptions,
        }
    }

    fn message(&self, cx: &Context<Self>) -> Option<String> {
        commit_message(
            &self.summary.read(cx).value(),
            &self.description.read(cx).value(),
        )
    }

    fn commit(&mut self, cx: &mut Context<Self>) {
        let Some(message) = self.message(cx) else {
            return;
        };
        self.repository
            .update(cx, |repository, cx| repository.commit(message, cx));
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.summary
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.description
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }
}

impl Render for CommitForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let repository = self.repository.read(cx);
        let staged = repository.staged_count();
        let committing = repository.is_committing();
        let busy = repository.is_writing();
        let failure = repository.write_failure().map(Failure::message);
        let ready = !busy && staged > 0 && self.message(cx).is_some();
        let label = if committing {
            "Committing…".to_owned()
        } else if staged > 0 {
            format!("Commit {}", counted(staged, "file", "files"))
        } else {
            "Commit".to_owned()
        };
        v_flex()
            .flex_none()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .capture_action(cx.listener(|this, enter: &Enter, _, cx| {
                if enter.secondary {
                    cx.stop_propagation();
                    this.commit(cx);
                }
            }))
            .child(Input::new(&self.summary).disabled(committing))
            .child(Textarea::new(&self.description).disabled(committing))
            .child(
                Button::new("commit")
                    .primary()
                    .w_full()
                    .label(label)
                    .loading(committing)
                    .disabled(!ready && !committing)
                    .tooltip_with_action(
                        "Commit the staged files",
                        &Enter {
                            secondary: true,
                            shift: false,
                        },
                        Some("Input"),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.commit(cx))),
            )
            .when_some(failure, |this, failure| {
                this.child(div().xs_text().text_color(cx.theme().red).child(failure))
            })
    }
}
