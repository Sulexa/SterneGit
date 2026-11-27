use gix::ObjectId;
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        h_flex,
        input::{Enter, Input, InputEvent, InputState, Textarea, TextareaState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use crate::ui::Nodding as _;
use super::repository::{Repository, RepositoryEvent};
use crate::{
    git::{CommitMode, commit_message},
    ui::{Failure, Look as _, counted, listed},
};

const DESCRIPTION_ROWS: usize = 4;
const AMEND: &str = "Amend";
const AMEND_TOOLTIP: &str =
    "Change the last commit instead of adding one: its message, and the staged files";
const NOTHING_TO_AMEND: &str = "No commit to amend yet";

pub(crate) struct CommitForm {
    repository: Entity<Repository>,
    summary: Entity<InputState>,
    description: Entity<TextareaState>,
    amending: Option<Amending>,
    _subscriptions: [Subscription; 3],
}

struct Amending {
    head: ObjectId,
    written: Option<String>,
    typed_before: (SharedString, SharedString),
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
            cx.subscribe_in(&repository, window, |this, repository, event, window, cx| {
                match event {
                    RepositoryEvent::Committed => this.clear(window, cx),
                    RepositoryEvent::ChangesRead => {
                        let head = repository.read(cx).head_commit();
                        if this
                            .amending
                            .as_ref()
                            .is_some_and(|amending| Some(amending.head) != head)
                        {
                            this.stop_amending(window, cx)
                        }
                    }
                    _ => {}
                }
            }),
        ];
        Self {
            repository,
            summary,
            description,
            amending: None,
            _subscriptions: subscriptions,
        }
    }

    fn typed(&self, cx: &Context<Self>) -> (SharedString, SharedString) {
        (
            self.summary.read(cx).value().into(),
            self.description.read(cx).value().into(),
        )
    }

    fn message(&self, cx: &Context<Self>) -> Option<String> {
        let (summary, description) = self.typed(cx);
        commit_message(&summary, &description)
    }

    fn ready(&self, cx: &Context<Self>) -> Option<(String, CommitMode)> {
        let repository = self.repository.read(cx);
        if repository.is_writing() {
            return None;
        }
        let message = self.message(cx)?;
        let staged = repository.staged_count() > 0;
        match &self.amending {
            None => staged.then_some((message, CommitMode::New)),
            Some(amending) => (staged || amending.written.as_ref() != Some(&message))
                .then_some((message, CommitMode::AmendLast)),
        }
    }

    fn commit(&mut self, cx: &mut Context<Self>) {
        let Some((message, mode)) = self.ready(cx) else {
            return;
        };
        self.repository
            .update(cx, |repository, cx| repository.commit(message, mode, cx));
    }

    fn set_amending(&mut self, amend: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !amend {
            self.stop_amending(window, cx);
            return;
        }
        let repository = self.repository.read(cx);
        let Some((repo, head)) = repository.repo().zip(repository.head_commit()) else {
            return;
        };
        let last = repo.summary_and_description(head).ok();
        let written = last
            .as_ref()
            .and_then(|(summary, description)| commit_message(summary, description));
        self.amending = Some(Amending {
            head,
            written,
            typed_before: self.typed(cx),
        });
        if let Some((summary, description)) = last {
            self.show(summary.into(), description.into(), window, cx);
        }
        self.summary.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    fn stop_amending(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(Amending {
            typed_before: (summary, description),
            ..
        }) = self.amending.take()
        {
            self.show(summary, description, window, cx);
        }
        cx.notify();
    }

    fn show(
        &mut self,
        summary: SharedString,
        description: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.summary
            .update(cx, |state, cx| state.set_value(summary, window, cx));
        self.description
            .update(cx, |state, cx| state.set_value(description, window, cx));
    }

    fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.amending = None;
        self.show(SharedString::default(), SharedString::default(), window, cx);
        cx.notify();
    }
}

impl Render for CommitForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let repository = self.repository.read(cx);
        let staged = repository.staged_count();
        let committing = repository.is_committing();
        let has_commit = repository.head_commit().is_some();
        let failure = repository.write_failure().map(Failure::message);
        let pushed = self.amending.as_ref().and_then(|amending| {
            let remotes = repository.history()?.reword_scope(&amending.head)?.remotes;
            (!remotes.is_empty()).then(|| pushed_already(&remotes))
        });
        let amending = self.amending.is_some();
        let ready = self.ready(cx).is_some();
        let theme = cx.theme();
        let (yellow, red) = (theme.yellow, theme.red);
        v_flex()
            .flex_none()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(theme.border)
            .capture_action(cx.listener(|this, enter: &Enter, _, cx| {
                if enter.secondary {
                    cx.stop_propagation();
                    this.commit(cx);
                }
            }))
            .child(Input::new(&self.summary).disabled(committing))
            .child(Textarea::new(&self.description).disabled(committing))
            .when_some(pushed, |this, pushed| {
                this.child(
                    h_flex()
                        .gap_1p5()
                        .xs_text()
                        .text_color(yellow)
                        .child(Icon::new(IconName::TriangleAlert).xsmall())
                        .child(pushed),
                )
            })
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        Checkbox::new("amend")
                            .label(AMEND)
                            .checked(amending)
                            .disabled(committing || !has_commit)
                            .tooltip(if has_commit { AMEND_TOOLTIP } else { NOTHING_TO_AMEND })
                            .on_click(cx.listener(|this, amend: &bool, window, cx| {
                                this.set_amending(*amend, window, cx)
                            })),
                    )
                    .child(
                        Button::new("commit")
                            .primary()
                            .flex_1()
                            .label(label(amending, committing, staged))
                            .nodding(committing)
                            .disabled(!ready && !committing)
                            .tooltip_with_action(
                                if amending {
                                    "Amend the last commit"
                                } else {
                                    "Commit the staged files"
                                },
                                &Enter {
                                    secondary: true,
                                    shift: false,
                                },
                                Some("Input"),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.commit(cx))),
                    ),
            )
            .when_some(failure, |this, failure| {
                this.child(div().xs_text().text_color(red).child(failure))
            })
    }
}

fn label(amending: bool, committing: bool, staged: usize) -> String {
    match (amending, committing, staged) {
        (true, true, _) => "Amending…".to_owned(),
        (true, false, 0) => "Amend last commit".to_owned(),
        (true, false, staged) => format!("Amend with {}", counted(staged, "file", "files")),
        (false, true, _) => "Committing…".to_owned(),
        (false, false, 0) => "Commit".to_owned(),
        (false, false, staged) => format!("Commit {}", counted(staged, "file", "files")),
    }
}

fn pushed_already(remotes: &[SharedString]) -> String {
    let has = if remotes.len() > 1 { "have" } else { "has" };
    format!(
        "{} already {has} the last commit: the next push needs Force push.",
        listed(remotes)
    )
}

#[cfg(test)]
mod tests {
    use gpui_kit::SharedString;

    use super::{label, pushed_already};

    #[test]
    fn the_button_says_what_it_writes() {
        assert_eq!(label(false, false, 0), "Commit");
        assert_eq!(label(false, false, 2), "Commit 2 files");
        assert_eq!(label(false, true, 2), "Committing…");
        assert_eq!(label(true, false, 0), "Amend last commit");
        assert_eq!(label(true, false, 1), "Amend with 1 file");
        assert_eq!(label(true, true, 1), "Amending…");
    }

    #[test]
    fn the_warning_names_every_remote_branch_holding_the_last_commit() {
        let names = |names: &[&str]| -> Vec<SharedString> {
            names.iter().map(|name| SharedString::from((*name).to_owned())).collect()
        };
        assert_eq!(
            pushed_already(&names(&["origin/main"])),
            "origin/main already has the last commit: the next push needs Force push."
        );
        assert_eq!(
            pushed_already(&names(&["origin/main", "upstream/main"])),
            "origin/main and upstream/main already have the last commit: the next push needs Force push."
        );
    }
}
