use fastgit_protocol::{Merge, MergeMethod, PullDetails};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Input, InputState, Textarea, TextareaState},
        radio::Radio,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    looks::{makes_a_commit, method_about, method_button, method_label},
    overview::PullView,
};
use crate::{
    app::connectors::note_failure,
    connect::{CallError, Forge},
    hosting::Trouble,
    ui::Look as _,
};

const WIDTH: f32 = 600.;
const MESSAGE_ROWS: usize = 4;
const METHOD: &str = "Method";
const COMMIT_TITLE: &str = "Commit title";
const COMMIT_MESSAGE: &str = "Commit message";
const CANCEL: &str = "Cancel";
const SHOWN_METHODS: [MergeMethod; 5] = [
    MergeMethod::Merge,
    MergeMethod::Squash,
    MergeMethod::Rebase,
    MergeMethod::SemiLinear,
    MergeMethod::FastForward,
];

struct MergeForm {
    view: Entity<PullView>,
    forge: Forge,
    details: PullDetails,
    head: String,
    subjects: Vec<String>,
    methods: Vec<MergeMethod>,
    method: MergeMethod,
    title: Entity<InputState>,
    message: Entity<TextareaState>,
    problem: Option<SharedString>,
    busy: Option<Task<()>>,
}

pub(crate) fn squash_title(title: &str, number: u64) -> String {
    format!("{title} (#{number})")
}

pub(crate) fn merge_title(number: u64, owner: &str, head: &str) -> String {
    format!("Merge pull request #{number} from {owner}/{head}")
}

pub(crate) fn squash_message(subjects: &[String]) -> String {
    subjects
        .iter()
        .map(|subject| format!("* {subject}"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl MergeForm {
    fn fill(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let summary = &self.details.summary;
        let owner = summary
            .head
            .repo
            .as_ref()
            .map_or_else(|| self.head.clone(), |repo| repo.owner.clone());
        let (title, message) = match self.method {
            MergeMethod::Squash => (squash_title(&summary.title, summary.number), squash_message(&self.subjects)),
            _ => (merge_title(summary.number, &owner, &summary.head.branch), summary.title.clone()),
        };
        self.title.update(cx, |state, cx| state.set_value(title, window, cx));
        self.message.update(cx, |state, cx| state.set_value(message, window, cx));
    }

    fn choose(&mut self, method: MergeMethod, window: &mut Window, cx: &mut Context<Self>) {
        if self.method != method {
            self.method = method;
            self.fill(window, cx);
            cx.notify();
        }
    }

    fn merge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        let words = makes_a_commit(self.method);
        let text = |value: SharedString| Some(value.trim().to_owned()).filter(|text| !text.is_empty());
        let merge = Merge {
            number: self.details.summary.number,
            method: self.method,
            title: words.then(|| text(self.title.read(cx).value())).flatten(),
            message: words.then(|| text(self.message.read(cx).value())).flatten(),
            expected_head: self.details.summary.head.sha.clone(),
        };
        let account = self.forge.account().clone();
        let merging = self.forge.merge(merge);
        let view = self.view.clone();
        self.problem = None;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let merged = merging.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                match merged {
                    Ok(_) => {
                        window.close_dialog(cx);
                        view.update(cx, |view, cx| view.merged(window, cx));
                    }
                    Err(error) => {
                        this.problem = Some(problem_of(&error, &account, cx));
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }
}

fn problem_of(error: &CallError, account: &crate::accounts::AccountId, cx: &mut App) -> SharedString {
    note_failure(cx, account, error);
    let trouble = Trouble::of(error, account, cx);
    format!("{} {}", trouble.title, trouble.message).into()
}

impl Render for MergeForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, red, border, hover) = (theme.muted_foreground, theme.red, theme.border, theme.list_hover);
        let busy = self.busy.is_some();
        let summary = &self.details.summary;
        let choices = self.methods.iter().map(|&method| {
            let chosen = method == self.method;
            h_flex()
                .gap_2p5()
                .px_2p5()
                .py_2()
                .border_1()
                .border_color(border)
                .rounded(px(6.))
                .when(chosen, |row| row.bg(hover))
                .child(
                    div().flex_none().w(px(190.)).child(
                        Radio::new(SharedString::from(method_label(method)))
                            .label(method_label(method))
                            .checked(chosen)
                            .on_click(cx.listener(move |this, _, window, cx| this.choose(method, window, cx))),
                    ),
                )
                .child(div().flex_1().xs_text().text_color(muted).child(method_about(method)))
        });
        let words = if makes_a_commit(self.method) {
            v_flex()
                .gap_4()
                .child(field(COMMIT_TITLE, Input::new(&self.title).into_any_element()))
                .child(field(COMMIT_MESSAGE, Textarea::new(&self.message).into_any_element()))
        } else {
            v_flex().child(div().sm_text().text_color(muted).child(format!(
                "Each commit is replayed onto {}; the server keeps their messages.",
                summary.base.branch
            )))
        };
        v_flex()
            .gap_4()
            .child(div().sm_text().text_color(muted).child(format!(
                "{} is merged into {} on the server, only if its head is still {}.",
                self.head,
                summary.base.branch,
                summary.head.sha.chars().take(7).collect::<String>()
            )))
            .child(field(METHOD, v_flex().gap_1p5().children(choices).into_any_element()))
            .child(words)
            .children(self.problem.clone().map(|problem| div().sm_text().text_color(red).child(problem)))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("merge-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("merge-confirm")
                            .primary()
                            .icon(IconName::GitMerge)
                            .label(method_button(self.method))
                            .loading(busy)
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, window, cx| this.merge(window, cx))),
                    ),
            )
    }
}

fn field(label: &'static str, control: gpui_kit::AnyElement) -> impl IntoElement {
    v_flex()
        .gap_1p5()
        .child(div().sm_text().font_medium().child(label))
        .child(control)
}

pub(crate) fn open_merge_dialog(
    view: Entity<PullView>,
    forge: Forge,
    details: PullDetails,
    head: String,
    subjects: Vec<String>,
    window: &mut Window,
    cx: &mut App,
) {
    let methods: Vec<MergeMethod> = SHOWN_METHODS
        .into_iter()
        .filter(|method| details.merge_methods.contains(method))
        .collect();
    let Some(&first) = methods.first() else {
        return;
    };
    let title = format!("Merge pull request #{}", details.summary.number);
    let form = cx.new(|cx| {
        let mut form = MergeForm {
            view,
            forge,
            details,
            head,
            subjects,
            methods,
            method: first,
            title: cx.new(|cx| InputState::new(window, cx)),
            message: cx.new(|cx| TextareaState::new(window, cx).rows(MESSAGE_ROWS)),
            problem: None,
            busy: None,
        };
        form.fill(window, cx);
        form
    });
    window.open_dialog(cx, move |dialog, _, _| {
        let merging = form.clone();
        dialog
            .w(px(WIDTH))
            .title(title.clone())
            .child(form.clone())
            .on_ok(move |_, window, cx| {
                merging.update(cx, |form, cx| form.merge(window, cx));
                false
            })
    });
}

#[cfg(test)]
mod tests {
    use super::{merge_title, squash_message, squash_title};

    #[test]
    fn the_commit_words_are_filled_in_as_github_would() {
        assert_eq!(squash_title("Render side-by-side diffs", 12), "Render side-by-side diffs (#12)");
        assert_eq!(
            merge_title(12, "fastgit-org", "feature/diff-view"),
            "Merge pull request #12 from fastgit-org/feature/diff-view"
        );
        assert_eq!(
            squash_message(&["Add two".to_owned(), "Add a file".to_owned()]),
            "* Add two\n* Add a file"
        );
    }
}
