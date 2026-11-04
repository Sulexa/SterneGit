use std::sync::Arc;

use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Window,
    component::{
        ActiveTheme as _, Disableable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        dialog::{DialogAction, DialogClose, DialogDescription, DialogFooter},
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::repository::Repository;
use crate::{
    git::{History, NameProblem, RefLabel, new_branch_name, short_id},
    ui::Look as _,
};

const WIDTH: f32 = 420.;
const TITLE: &str = "Create a branch";
const CREATE: &str = "Create branch";
const PLACEHOLDER: &str = "Branch name";
const REFUSED: &str = "Git doesn't accept this name: no spaces, and none of ~ ^ : ? * [ \\ or ..";

struct BranchForm {
    name: Entity<InputState>,
    history: Arc<History>,
    _edits: Subscription,
}

impl BranchForm {
    fn new(history: Arc<History>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(PLACEHOLDER));
        let edits = cx.subscribe(&name, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify()
            }
        });
        Self {
            name,
            history,
            _edits: edits,
        }
    }

    fn checked(&self, cx: &App) -> Result<String, NameProblem> {
        let typed = self.name.read(cx).value();
        new_branch_name(&typed, &self.history.refs).map(str::to_owned)
    }

    fn problem(&self, cx: &App) -> Option<String> {
        match self.checked(cx) {
            Err(NameProblem::Taken) => Some(format!(
                "There is already a branch named {}.",
                self.name.read(cx).value().trim()
            )),
            Err(NameProblem::Refused) => Some(REFUSED.to_owned()),
            Err(NameProblem::Empty) | Ok(_) => None,
        }
    }
}

impl Render for BranchForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let problem = self.problem(cx);
        v_flex()
            .gap_1p5()
            .mt_3()
            .child(Input::new(&self.name))
            .when_some(problem, |this, problem| {
                this.child(div().xs_text().text_color(cx.theme().red).child(problem))
            })
    }
}

pub(crate) fn open_branch_dialog(
    repository: Entity<Repository>,
    start: Option<RefLabel>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(history) = repository.read(cx).history().cloned() else {
        return;
    };
    let description = match &start {
        None => format!(
            "It starts at {}, where HEAD is, and is checked out right away. \
             Uncommitted changes stay as they are.",
            starting_point(&history)
        ),
        Some(label) => format!(
            "It starts at {} ({}) and is checked out right away. \
             Uncommitted changes come along unless they clash with it.",
            label.name,
            short_id(label.target)
        ),
    };
    let start = start.map(|label| label.revision(&history.refs));
    let form = cx.new(|cx| BranchForm::new(history, window, cx));
    let name = form.read(cx).name.clone();
    window.open_dialog(cx, move |dialog, _, cx| {
        let ready = form.read(cx).checked(cx).is_ok();
        let form_on_ok = form.clone();
        let repository = repository.clone();
        let start = start.clone();
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
                            Button::new("create-branch")
                                .primary()
                                .label(CREATE)
                                .disabled(!ready),
                        ),
                    ),
            )
            .on_ok(move |_, _, cx| {
                let Ok(name) = form_on_ok.read(cx).checked(cx) else {
                    return false;
                };
                repository.update(cx, |repository, cx| {
                    repository.create_branch(name, start.clone(), cx)
                });
                true
            })
    });
    name.update(cx, |state, cx| state.focus(window, cx));
}

fn starting_point(history: &History) -> String {
    match (&history.branch, history.head_id) {
        (Some(branch), Some(id)) => format!("{branch} ({})", short_id(id)),
        (None, Some(id)) => short_id(id).to_string(),
        (Some(branch), None) => branch.to_string(),
        (None, None) => "HEAD".to_owned(),
    }
}
