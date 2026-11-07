use std::{path::PathBuf, sync::Arc};

use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, Global, IntoElement, ParentElement as _,
    PathPromptOptions, Render, SharedString, Styled as _, Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        dialog::{DialogAction, DialogClose, DialogFooter},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::PopupMenuItem,
        radio::Radio,
        v_flex,
    },
    div, px,
};

use super::{branch_dialog::REFUSED, repository::Repository};
use crate::{
    git::{
        History, NameProblem, RefKind, RefLabel, WorktreeStart, folder_is_free, folder_of_worktrees,
        new_branch_name, suggested_folder,
    },
    hosting::{Note, Picker, field},
    ui::Look as _,
};

const WIDTH: f32 = 600.;
const TITLE: &str = "New worktree";
const ABOUT: &str = "Another folder with its own checkout of this repository. Commits, branches and stashes are shared with every worktree; the files, the staged changes and HEAD are its own.";
const ONE_CHECKOUT: &str = "A branch can be checked out in one worktree at a time. A remote branch checks out its local branch, or gets one that tracks it.";
const NOT_EMPTY: &str = "This folder already exists and isn't empty";
const PICK_A_BRANCH: &str = "Pick a branch";
const MENU_HEIGHT: f32 = 320.;

#[derive(Clone, Copy, Default)]
struct OpensWorktreeTabs(bool);

impl Global for OpensWorktreeTabs {}

fn opens_tabs(cx: &App) -> bool {
    cx.try_global::<OpensWorktreeTabs>().is_some_and(|opens| opens.0)
}

#[derive(Clone)]
struct Choice {
    label: RefLabel,
    taken_by: Option<SharedString>,
}

struct WorktreeForm {
    history: Arc<History>,
    main: PathBuf,
    fresh: bool,
    branch: Option<RefLabel>,
    from: Option<RefLabel>,
    name: Entity<InputState>,
    folder: Entity<InputState>,
    suggested: String,
    _edits: [Subscription; 2],
}

impl WorktreeForm {
    fn new(
        history: Arc<History>,
        main: PathBuf,
        start: Option<RefLabel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("Branch name"));
        let folder = cx.new(|cx| InputState::new(window, cx));
        let edits = [
            cx.subscribe_in(&name, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.suggest_folder(window, cx);
                    cx.notify()
                }
            }),
            cx.subscribe(&folder, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify()
                }
            }),
        ];
        let head_branch = history
            .refs
            .iter()
            .find(|label| label.is_head)
            .cloned();
        let available = start.clone().filter(|label| taken_by(&history.refs, label).is_none());
        let mut form = Self {
            fresh: available.is_none(),
            branch: available,
            from: start.or(head_branch),
            history,
            main,
            name,
            folder,
            suggested: String::new(),
            _edits: edits,
        };
        form.suggest_folder(window, cx);
        form
    }

    fn choices(&self) -> Vec<Choice> {
        self.history
            .refs
            .iter()
            .filter(|label| label.is_branch())
            .map(|label| Choice {
                label: label.clone(),
                taken_by: taken_by(&self.history.refs, label),
            })
            .collect()
    }

    fn branch_name(&self, cx: &App) -> String {
        match (self.fresh, &self.branch) {
            (true, _) => self.name.read(cx).value().trim().to_owned(),
            (false, Some(label)) => label.local_name().to_owned(),
            (false, None) => String::new(),
        }
    }

    fn suggest_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let branch = self.branch_name(cx);
        let suggestion = match branch.is_empty() {
            true => String::new(),
            false => suggested_folder(&self.main, &branch).display().to_string(),
        };
        let typed = self.folder.read(cx).value().to_string();
        if typed == self.suggested || typed.is_empty() {
            self.folder
                .update(cx, |state, cx| state.set_value(suggestion.clone(), window, cx));
        }
        self.suggested = suggestion;
    }

    fn folder(&self, cx: &App) -> Option<PathBuf> {
        let typed = self.folder.read(cx).value().trim().to_owned();
        (!typed.is_empty()).then(|| PathBuf::from(typed))
    }

    fn name_problem(&self, cx: &App) -> Option<String> {
        if !self.fresh {
            return None;
        }
        let typed = self.name.read(cx).value();
        match new_branch_name(&typed, &self.history.refs) {
            Err(NameProblem::Taken) => Some(format!("There is already a branch named {}.", typed.trim())),
            Err(NameProblem::Refused) => Some(REFUSED.to_owned()),
            Err(NameProblem::Empty) | Ok(_) => None,
        }
    }

    fn folder_problem(&self, cx: &App) -> Option<&'static str> {
        self.folder(cx)
            .filter(|folder| !folder_is_free(folder))
            .map(|_| NOT_EMPTY)
    }

    fn start(&self, cx: &App) -> Option<WorktreeStart> {
        if self.fresh {
            let typed = self.name.read(cx).value();
            let name = new_branch_name(&typed, &self.history.refs).ok()?.to_owned();
            let from = self
                .from
                .as_ref()
                .map_or_else(|| "HEAD".to_owned(), |label| label.revision(&self.history.refs));
            return Some(WorktreeStart::NewBranch { name, from });
        }
        let label = self.branch.as_ref()?;
        Some(worktree_start(label, &self.history.refs))
    }

    fn ready(&self, cx: &App) -> Option<(PathBuf, WorktreeStart)> {
        if self.folder_problem(cx).is_some() {
            return None;
        }
        Some((self.folder(cx)?, self.start(cx)?))
    }

    fn choose_mode(&mut self, fresh: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.fresh = fresh;
        self.suggest_folder(window, cx);
        if fresh {
            self.name.update(cx, |state, cx| state.focus(window, cx));
        }
        cx.notify();
    }

    fn choose_branch(&mut self, label: RefLabel, window: &mut Window, cx: &mut Context<Self>) {
        self.branch = Some(label);
        self.suggest_folder(window, cx);
        cx.notify();
    }

    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Create the worktree in".into()),
        });
        let branch = self.branch_name(cx);
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else {
                return;
            };
            let Some(chosen) = paths.into_iter().next() else {
                return;
            };
            let folder = match folder_is_free(&chosen) || branch.is_empty() {
                true => chosen,
                false => chosen.join(branch.rsplit('/').next().unwrap_or(&branch)),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.folder
                    .update(cx, |state, cx| state.set_value(folder.display().to_string(), window, cx));
                cx.notify();
            });
        })
        .detach();
    }

    fn branch_picker(&self, id: &'static str, chosen: Option<&RefLabel>, fresh: bool, cx: &mut Context<Self>) -> AnyElement {
        let choices = self.choices();
        let this = cx.entity().downgrade();
        let value: SharedString = chosen.map_or_else(|| PICK_A_BRANCH.into(), |label| label.name.clone());
        Picker::new(id, value)
            .lead(Icon::new(IconName::GitBranch).small().text_color(cx.theme().muted_foreground))
            .menu(move |menu, _, _| {
                choices
                    .iter()
                    .fold(menu.max_h(px(MENU_HEIGHT)).scrollable(true), |menu, choice| {
                        let this = this.clone();
                        let label = choice.label.clone();
                        let text = match (&choice.taken_by, fresh) {
                            (Some(worktree), false) => format!("{} · in {worktree}", label.name),
                            _ => label.name.to_string(),
                        };
                        menu.item(
                            PopupMenuItem::new(text)
                                .disabled(choice.taken_by.is_some() && !fresh)
                                .on_click(move |_, window, cx| {
                                    let label = label.clone();
                                    let _ = this.update(cx, |form, cx| match fresh {
                                        true => {
                                            form.from = Some(label);
                                            cx.notify();
                                        }
                                        false => form.choose_branch(label, window, cx),
                                    });
                                }),
                        )
                    })
            })
            .into_any_element()
    }
}

fn local_branch<'a>(refs: &'a [RefLabel], label: &'a RefLabel) -> Option<&'a RefLabel> {
    if label.kind == RefKind::LocalBranch {
        return Some(label);
    }
    let local = label.local_name();
    refs.iter()
        .find(|other| other.kind == RefKind::LocalBranch && other.name.as_str() == local)
}

fn worktree_start(label: &RefLabel, refs: &[RefLabel]) -> WorktreeStart {
    match label.on_remote() {
        Some(remote) if local_branch(refs, label).is_none() => WorktreeStart::Tracking {
            branch: remote.branch.to_owned(),
            upstream: label.name.to_string(),
        },
        _ => WorktreeStart::Branch(label.local_name().to_owned()),
    }
}

pub(super) fn taken_by(refs: &[RefLabel], label: &RefLabel) -> Option<SharedString> {
    let local = local_branch(refs, label)?;
    match local.is_head {
        true => Some("this tab".into()),
        false => local.worktree.clone(),
    }
}

impl Render for WorktreeForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let fresh = self.fresh;
        let modes = h_flex()
            .gap_4()
            .h_8()
            .child(
                Radio::new("worktree-existing")
                    .label("Existing branch")
                    .checked(!fresh)
                    .on_click(cx.listener(|this, _, window, cx| this.choose_mode(false, window, cx))),
            )
            .child(
                Radio::new("worktree-new")
                    .label("New branch")
                    .checked(fresh)
                    .on_click(cx.listener(|this, _, window, cx| this.choose_mode(true, window, cx))),
            );
        let branch = match fresh {
            true => {
                let problem = self.name_problem(cx).map(|problem| Note::Problem(problem.into()));
                let from = self.from.clone();
                let picker = self.branch_picker("worktree-from", from.as_ref(), true, cx);
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(div().flex_1().child(field("Name", Input::new(&self.name), problem, cx)))
                    .child(
                        div()
                            .w(px(200.))
                            .flex_none()
                            .child(field("Starting from", picker, None, cx)),
                    )
                    .into_any_element()
            }
            false => {
                let chosen = self.branch.clone();
                let picker = self.branch_picker("worktree-branch", chosen.as_ref(), false, cx);
                field(
                    "Branch",
                    picker,
                    Some(Note::Plain(ONE_CHECKOUT.into())),
                    cx,
                )
                .into_any_element()
            }
        };
        let parent = folder_of_worktrees(&self.main);
        let folder_note = match self.folder_problem(cx) {
            Some(problem) => Note::Problem(problem.into()),
            None => Note::Plain(format!("Next to the repository, in {}, named after the branch", parent.display()).into()),
        };
        v_flex()
            .gap_4()
            .mt_1()
            .child(div().sm_text().text_color(muted).child(ABOUT))
            .child(modes)
            .child(branch)
            .child(field(
                "Folder",
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.folder)))
                    .child(
                        Button::new("worktree-browse")
                            .label("Browse…")
                            .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
                    ),
                Some(folder_note),
                cx,
            ))
            .child(
                Checkbox::new("worktree-open-tab")
                    .label("Open it in a new tab")
                    .checked(opens_tabs(cx))
                    .on_click(cx.listener(|_, checked: &bool, _, cx| {
                        cx.set_global(OpensWorktreeTabs(*checked));
                        cx.notify();
                    })),
            )
    }
}

pub(crate) fn open_worktree_dialog(
    repository: Entity<Repository>,
    start: Option<RefLabel>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(history) = repository.read(cx).history().cloned() else {
        return;
    };
    let main = history
        .main_worktree()
        .map(|worktree| worktree.location.to_path_buf())
        .unwrap_or_else(|| repository.read(cx).path().to_path_buf());
    let form = cx.new(|cx| WorktreeForm::new(history, main, start, window, cx));
    let name = form.read(cx).fresh.then(|| form.read(cx).name.clone());
    window.open_dialog(cx, move |dialog, _, cx| {
        let ready = form.read(cx).ready(cx).is_some();
        let form_on_ok = form.clone();
        let repository = repository.clone();
        dialog
            .title(TITLE)
            .w(px(WIDTH))
            .child(form.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(
                        DialogAction::new().child(
                            Button::new("create-worktree")
                                .primary()
                                .icon(IconName::FolderPlus)
                                .label("Create worktree")
                                .disabled(!ready),
                        ),
                    ),
            )
            .on_ok(move |_, _, cx| {
                let Some((folder, start)) = form_on_ok.read(cx).ready(cx) else {
                    return false;
                };
                let open = opens_tabs(cx);
                repository.update(cx, |repository, cx| repository.add_worktree(folder, start, open, cx));
                true
            })
    });
    if let Some(name) = name {
        window.defer(cx, move |window, cx| name.update(cx, |state, cx| state.focus(window, cx)));
    }
}

#[cfg(test)]
mod tests {
    use super::{taken_by, worktree_start};
    use crate::git::{RefKind, RefLabel, WorktreeStart, fixtures::{id, linear_history}};

    fn label(name: &str, kind: RefKind, is_head: bool, worktree: Option<&str>) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind,
            target: id(1),
            is_head,
            upstream: None,
            from_upstream: None,
            worktree: worktree.map(|name| name.to_owned().into()),
        }
    }

    #[test]
    fn a_branch_checked_out_anywhere_is_taken_and_says_where() {
        let mut history = linear_history(&[1], Some(1));
        history.refs = vec![
            label("main", RefKind::LocalBranch, true, None),
            label("topic", RefKind::LocalBranch, false, Some("topic-tree")),
            label("free", RefKind::LocalBranch, false, None),
            label("origin/main", RefKind::RemoteBranch, false, None),
            label("origin/topic", RefKind::RemoteBranch, false, None),
            label("origin/free", RefKind::RemoteBranch, false, None),
            label("origin/other", RefKind::RemoteBranch, false, None),
        ];
        let taken: Vec<Option<String>> = history
            .refs
            .iter()
            .map(|label| taken_by(&history.refs, label).map(|place| place.to_string()))
            .collect();
        assert_eq!(
            taken,
            [
                Some("this tab".to_owned()),
                Some("topic-tree".to_owned()),
                None,
                Some("this tab".to_owned()),
                Some("topic-tree".to_owned()),
                None,
                None
            ]
        );
    }

    #[test]
    fn a_remote_branch_checks_out_its_local_branch_when_there_is_one() {
        let refs = [
            label("free", RefKind::LocalBranch, false, None),
            label("origin/free", RefKind::RemoteBranch, false, None),
            label("origin/other", RefKind::RemoteBranch, false, None),
        ];
        let starts: Vec<String> = refs
            .iter()
            .map(|label| match worktree_start(label, &refs) {
                WorktreeStart::Branch(branch) => format!("checks out {branch}"),
                WorktreeStart::Tracking { branch, upstream } => format!("makes {branch} tracking {upstream}"),
                WorktreeStart::NewBranch { name, from } => format!("makes {name} from {from}"),
            })
            .collect();
        assert_eq!(
            starts,
            ["checks out free", "checks out free", "makes other tracking origin/other"]
        );
    }
}
