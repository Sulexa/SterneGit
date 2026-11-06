use std::sync::Arc;

use gix::ObjectId;
use gpui_kit::{
    App, AppContext as _, Context, Entity, Global, IntoElement, ParentElement as _, Render,
    Styled as _, Subscription, Window,
    component::{
        ActiveTheme as _, Disableable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        dialog::{DialogAction, DialogClose, DialogDescription, DialogFooter},
        input::{Input, InputEvent, InputState, Textarea, TextareaState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{branch_dialog::REFUSED, repository::Repository};
use crate::{
    git::{History, NameProblem, new_tag_name, short_id},
    ui::Look as _,
};

const WIDTH: f32 = 420.;
const TITLE: &str = "Create a tag";
const CREATE: &str = "Create tag";
const NAME_PLACEHOLDER: &str = "Tag name";
const MESSAGE_PLACEHOLDER: &str = "Message (optional)";
const MESSAGE_ROWS: usize = 3;

#[derive(Clone, Copy, Default)]
struct PushTags(bool);

impl Global for PushTags {}

fn pushes_tags(cx: &App) -> bool {
    cx.try_global::<PushTags>().is_some_and(|push| push.0)
}

struct TagForm {
    name: Entity<InputState>,
    message: Entity<TextareaState>,
    history: Arc<History>,
    remote: Option<String>,
    _edits: Subscription,
}

impl TagForm {
    fn new(
        history: Arc<History>,
        remote: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(NAME_PLACEHOLDER));
        let message = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(MESSAGE_PLACEHOLDER)
                .rows(MESSAGE_ROWS)
        });
        let edits = cx.subscribe(&name, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify()
            }
        });
        Self {
            name,
            message,
            history,
            remote,
            _edits: edits,
        }
    }

    fn checked(&self, cx: &App) -> Result<String, NameProblem> {
        let typed = self.name.read(cx).value();
        new_tag_name(&typed, &self.history.refs).map(str::to_owned)
    }

    fn message(&self, cx: &App) -> Option<String> {
        let typed = self.message.read(cx).value();
        let message = typed.trim();
        (!message.is_empty()).then(|| message.to_owned())
    }

    fn push_to(&self, cx: &App) -> Option<String> {
        self.remote.clone().filter(|_| pushes_tags(cx))
    }

    fn problem(&self, cx: &App) -> Option<String> {
        match self.checked(cx) {
            Err(NameProblem::Taken) => Some(format!(
                "There is already a tag named {}.",
                self.name.read(cx).value().trim()
            )),
            Err(NameProblem::Refused) => Some(REFUSED.to_owned()),
            Err(NameProblem::Empty) | Ok(_) => None,
        }
    }
}

impl Render for TagForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let problem = self.problem(cx);
        let pushes = pushes_tags(cx);
        v_flex()
            .gap_3()
            .mt_3()
            .child(
                v_flex()
                    .gap_1p5()
                    .child(Input::new(&self.name))
                    .when_some(problem, |this, problem| {
                        this.child(div().xs_text().text_color(cx.theme().red).child(problem))
                    }),
            )
            .child(Textarea::new(&self.message))
            .when_some(self.remote.clone(), |this, remote| {
                this.child(
                    Checkbox::new("push-tag")
                        .label(format!("Push it to {remote}"))
                        .checked(pushes)
                        .on_click(cx.listener(|_, checked: &bool, _, cx| {
                            cx.set_global(PushTags(*checked));
                            cx.notify();
                        })),
                )
            })
    }
}

fn description(history: &History, id: ObjectId) -> String {
    let subject = history
        .row_of(&id)
        .map(|row| history.commits[row].subject.clone())
        .unwrap_or_default();
    format!(
        "It points to {}, “{subject}”. With a message it is an annotated tag, without one a lightweight tag.",
        short_id(id)
    )
}

pub(crate) fn open_tag_dialog(
    repository: Entity<Repository>,
    id: ObjectId,
    window: &mut Window,
    cx: &mut App,
) {
    let (history, remote) = {
        let repository = repository.read(cx);
        let Some(history) = repository.history().cloned() else {
            return;
        };
        (history, repository.repo().and_then(|repo| repo.push_remote()))
    };
    let description = description(&history, id);
    let form = cx.new(|cx| TagForm::new(history, remote, window, cx));
    let name = form.read(cx).name.clone();
    window.open_dialog(cx, move |dialog, _, cx| {
        let ready = form.read(cx).checked(cx).is_ok();
        let form_on_ok = form.clone();
        let repository = repository.clone();
        dialog
            .title(TITLE)
            .w(px(WIDTH))
            .child(DialogDescription::new().child(description.clone()))
            .child(form.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(
                        DialogAction::new().child(
                            Button::new("create-tag")
                                .primary()
                                .label(CREATE)
                                .disabled(!ready),
                        ),
                    ),
            )
            .on_ok(move |_, _, cx| {
                let form = form_on_ok.read(cx);
                let Ok(name) = form.checked(cx) else {
                    return false;
                };
                let message = form.message(cx);
                let push_to = form.push_to(cx);
                repository.update(cx, |repository, cx| {
                    repository.create_tag(name, id, message, push_to, cx)
                });
                true
            })
    });
    window.defer(cx, move |window, cx| {
        name.update(cx, |state, cx| state.focus(window, cx))
    });
}
