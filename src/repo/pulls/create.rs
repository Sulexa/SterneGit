use sternegit_protocol::{NewPull, PullSummary};
use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        h_flex,
        input::{Enter, Input, InputEvent, InputState, Textarea, TextareaState},
        menu::PopupMenuItem,
        notification::Notification,
        v_flex,
    },
    div, px,
};

use super::list::Hosting;
use crate::{
    app::connectors::note_failure,
    git::{BranchSpan, RefKind, RefLabel},
    hosting::{Note, Picker, Trouble, field},
    repo::repository::Repository,
    ui::{Handler, Look as _, counted, run_in_background},
};

const WIDTH: f32 = 600.;
const DESCRIPTION_ROWS: usize = 5;
const FROM: &str = "From";
const INTO: &str = "Into";
const TITLE: &str = "Title";
const DESCRIPTION: &str = "Description";
const OPTIONAL: &str = "Optional";
const DRAFT: &str = "Create as draft";
const DRAFT_ABOUT: &str = "Can't be merged until it is marked ready for review";
const CANCEL: &str = "Cancel";
const CREATE: &str = "Create pull request";
const HEAD_REF: &str = "HEAD";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Push {
    Nothing,
    Commits(usize),
    Publish,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Head {
    pub(crate) branch: String,
    pub(crate) push: Push,
}

pub(crate) fn head_of(label: &RefLabel, remote: &str) -> Head {
    match label.kind {
        RefKind::RemoteBranch => Head {
            branch: label.local_name().to_owned(),
            push: Push::Nothing,
        },
        _ => {
            let on_remote = label
                .upstream
                .as_deref()
                .and_then(|upstream| upstream.strip_prefix(&format!("{remote}/")));
            match on_remote {
                Some(branch) => Head {
                    branch: branch.to_owned(),
                    push: match label.from_upstream.map_or(0, |apart| apart.ahead) {
                        0 => Push::Nothing,
                        ahead => Push::Commits(ahead),
                    },
                },
                None => Head {
                    branch: label.name.to_string(),
                    push: Push::Publish,
                },
            }
        }
    }
}

pub(crate) fn push_note(label: &str, push: &Push, remote: &str) -> Option<String> {
    match push {
        Push::Nothing => None,
        Push::Commits(count) => Some(format!(
            "{label} has {} that {remote} doesn't. SterneGit pushes {} first.",
            counted(*count, "commit", "commits"),
            if *count == 1 { "it" } else { "them" }
        )),
        Push::Publish => Some(format!(
            "{label} isn't on {remote} yet. SterneGit pushes it first and makes {remote}/{label} its upstream."
        )),
    }
}

pub(crate) fn described(subjects: &[String]) -> String {
    match subjects {
        [] | [_] => String::new(),
        several => several
            .iter()
            .map(|subject| format!("- {subject}"))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

pub(crate) struct CreatePull {
    pub(crate) hosting: Hosting,
    pub(crate) repository: Entity<Repository>,
    pub(crate) from: RefLabel,
    pub(crate) into: Option<String>,
    pub(crate) on_created: Handler<PullSummary>,
}

struct CreateForm {
    request: CreatePull,
    sources: Vec<RefLabel>,
    targets: Vec<String>,
    from: RefLabel,
    into: Option<String>,
    title: Entity<InputState>,
    body: Entity<TextareaState>,
    draft: bool,
    span: Option<BranchSpan>,
    edited: bool,
    reading: Option<Task<()>>,
    problem: Option<SharedString>,
    busy: Option<Task<()>>,
    _subscriptions: [Subscription; 2],
}

impl CreateForm {
    fn read_span(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(into), Some(repo)) = (self.into.clone(), self.request.repository.read(cx).repo().cloned()) else {
            return;
        };
        let base = format!("{}/{into}", self.request.hosting.remote);
        let tip = self.from.full_name();
        self.span = None;
        let window_handle = window.window_handle();
        self.reading = Some(run_in_background(
            cx,
            move || repo.branch_span(&base, &tip),
            move |this, result, cx| {
                this.reading = None;
                let Ok(span) = result else {
                    return;
                };
                if !this.edited {
                    let title = span.subjects.first().cloned().unwrap_or_default();
                    let body = described(&span.subjects);
                    let (title_state, body_state) = (this.title.clone(), this.body.clone());
                    let _ = window_handle.update(cx, |_, window, cx| {
                        title_state.update(cx, |state, cx| state.set_value(title, window, cx));
                        body_state.update(cx, |state, cx| state.set_value(body, window, cx));
                    });
                    this.edited = false;
                }
                this.span = Some(span);
                cx.notify();
            },
        ));
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.title.read(cx).value().trim().to_owned();
        let Some(into) = self.into.clone() else {
            return;
        };
        if title.is_empty() || self.busy.is_some() {
            return;
        }
        let Some(repo) = self.request.repository.read(cx).reached_repo() else {
            return;
        };
        let head = head_of(&self.from, &self.request.hosting.remote);
        let pushed = (head.push != Push::Nothing).then(|| self.from.name.to_string());
        let new = NewPull {
            base: into,
            head: head.branch,
            head_owner: None,
            title,
            body: self.body.read(cx).value().trim().to_owned(),
            draft: self.draft,
        };
        let forge = self.request.hosting.forge.clone();
        let account = self.request.hosting.account.clone();
        let repository = self.request.repository.clone();
        let on_created = self.request.on_created.clone();
        self.problem = None;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let pushing = cx
                .background_spawn(async move {
                    match pushed {
                        Some(branch) => repo.push_branch(&branch).map(drop).map_err(|error| format!("{error:#}")),
                        None => Ok(()),
                    }
                })
                .await;
            let created = match pushing {
                Ok(()) => Some(forge.create_pull(new).await),
                Err(failure) => {
                    let _ = this.update(cx, |this, cx| {
                        this.busy = None;
                        this.problem = Some(format!("Couldn't push first: {failure}").into());
                        cx.notify();
                    });
                    None
                }
            };
            let Some(created) = created else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                repository.update(cx, |repository, cx| repository.refresh(cx));
                match created {
                    Ok(summary) => {
                        window.close_dialog(cx);
                        window.push_notification(Notification::success(format!("Created pull request #{}", summary.number)), cx);
                        on_created(&summary, window, cx);
                    }
                    Err(error) => {
                        note_failure(cx, &account, &error);
                        let trouble = Trouble::of(&error, &account, cx);
                        this.problem = Some(trouble.message);
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }

    fn from_picker(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let sources = self.sources.clone();
        let chosen = self.from.full_name();
        Picker::new("create-pull-from", self.from.name.clone())
            .lead(Icon::new(IconName::GitBranch).xsmall())
            .menu(move |menu, _, _| {
                sources.iter().fold(menu, |menu, label| {
                    let this = this.clone();
                    let picked = label.clone();
                    menu.item(
                        PopupMenuItem::new(label.name.clone())
                            .checked(label.full_name() == chosen)
                            .on_click(move |_, window, cx| {
                                let _ = this.update(cx, |this, cx| {
                                    this.from = picked.clone();
                                    this.edited = false;
                                    this.read_span(window, cx);
                                    cx.notify();
                                });
                            }),
                    )
                })
            })
    }

    fn into_picker(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let this = cx.entity().downgrade();
        let targets = self.targets.clone();
        let chosen = self.into.clone();
        Picker::new("create-pull-into", self.into.clone().unwrap_or_default())
            .lead(Icon::new(IconName::GitBranch).xsmall())
            .menu(move |menu, _, _| {
                targets.iter().fold(menu, |menu, target| {
                    let this = this.clone();
                    let picked = target.clone();
                    menu.item(
                        PopupMenuItem::new(target.clone())
                            .checked(chosen.as_deref() == Some(target.as_str()))
                            .on_click(move |_, window, cx| {
                                let _ = this.update(cx, |this, cx| {
                                    this.into = Some(picked.clone());
                                    this.read_span(window, cx);
                                    cx.notify();
                                });
                            }),
                    )
                })
            })
    }
}

impl Render for CreateForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, blue) = (theme.muted_foreground, theme.blue);
        let remote = self.request.hosting.remote.clone();
        let head = head_of(&self.from, &remote);
        let note = push_note(&self.from.name, &head.push, &remote);
        let busy = self.busy.is_some();
        let ready = !self.title.read(cx).value().trim().is_empty() && self.into.is_some() && !busy;
        let compared = self
            .span
            .as_ref()
            .map(|span| {
                format!(
                    "{} and {} changed since it branched off",
                    counted(span.subjects.len(), "commit", "commits"),
                    counted(span.files, "file", "files")
                )
            })
            .map(|text| Note::Plain(text.into()));
        let (from, into) = (self.from_picker(cx), self.into_picker(cx));
        v_flex()
            .gap_4()
            .capture_action(cx.listener(|this, enter: &Enter, window, cx| {
                if enter.secondary {
                    cx.stop_propagation();
                    this.create(window, cx);
                }
            }))
            .child(div().sm_text().text_color(muted).child(format!(
                "Asks {} to merge a branch into another. Reviewers and labels are set on {}.",
                self.request.hosting.service, self.request.hosting.service
            )))
            .child(
                h_flex()
                    .items_end()
                    .gap_2()
                    .child(div().flex_1().min_w_0().child(field(FROM, from, None, cx)))
                    .child(div().pb_2().child(Icon::new(IconName::ArrowRight).small().text_color(muted)))
                    .child(div().flex_1().min_w_0().child(field(INTO, into, None, cx))),
            )
            .children(note.map(|note| {
                h_flex()
                    .gap_2()
                    .px_2p5()
                    .py_2()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(blue.opacity(0.3))
                    .bg(blue.opacity(0.08))
                    .xs_text()
                    .child(Icon::new(IconName::ArrowUpFromLine).xsmall().text_color(blue))
                    .child(div().flex_1().min_w_0().child(note))
            }))
            .child(field(TITLE, Input::new(&self.title), self.problem.clone().map(Note::Problem), cx))
            .child(field(DESCRIPTION, Textarea::new(&self.body), compared, cx))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Checkbox::new("create-pull-draft")
                            .label(DRAFT)
                            .checked(self.draft)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.draft = *checked;
                                cx.notify();
                            })),
                    )
                    .child(div().xs_text().text_color(muted).child(DRAFT_ABOUT)),
            )
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("create-pull-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("create-pull-confirm")
                            .primary()
                            .icon(IconName::GitPullRequest)
                            .label(CREATE)
                            .loading(busy)
                            .disabled(!ready)
                            .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
                    ),
            )
    }
}

pub(crate) fn open_create_pull(request: CreatePull, window: &mut Window, cx: &mut App) {
    let (sources, targets) = {
        let remote = format!("{}/", request.hosting.remote);
        let refs = request
            .repository
            .read(cx)
            .history()
            .map(|history| history.refs.clone())
            .unwrap_or_default();
        let mut sources: Vec<RefLabel> = refs.iter().filter(|label| label.kind == RefKind::LocalBranch).cloned().collect();
        if request.from.kind == RefKind::RemoteBranch {
            sources.push(request.from.clone());
        }
        let targets: Vec<String> = refs
            .iter()
            .filter(|label| label.kind == RefKind::RemoteBranch)
            .filter_map(|label| label.name.strip_prefix(&remote).map(str::to_owned))
            .filter(|branch| branch != HEAD_REF)
            .collect();
        (sources, targets)
    };
    let into = request
        .into
        .clone()
        .filter(|into| targets.contains(into))
        .or_else(|| targets.first().cloned());
    let from = request.from.clone();
    let form = cx.new(|cx| {
        let title = cx.new(|cx| InputState::new(window, cx));
        let body = cx.new(|cx| TextareaState::new(window, cx).placeholder(OPTIONAL).rows(DESCRIPTION_ROWS));
        let subscriptions = [
            cx.subscribe(&title, |this: &mut CreateForm, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    this.edited = true;
                    cx.notify()
                }
            }),
            cx.subscribe(&body, |this: &mut CreateForm, _, event, _| {
                if matches!(event, InputEvent::Change) {
                    this.edited = true;
                }
            }),
        ];
        let mut form = CreateForm {
            request,
            sources,
            targets,
            from,
            into,
            title,
            body,
            draft: false,
            span: None,
            edited: false,
            reading: None,
            problem: None,
            busy: None,
            _subscriptions: subscriptions,
        };
        form.read_span(window, cx);
        form
    });
    window.open_dialog(cx, move |dialog, _, _| {
        let creating = form.clone();
        dialog
            .w(px(WIDTH))
            .title("Create a pull request")
            .child(form.clone())
            .on_ok(move |_, window, cx| {
                creating.update(cx, |form, cx| form.create(window, cx));
                false
            })
    });
}

#[cfg(test)]
mod tests {
    use gix::ObjectId;

    use super::{Head, Push, described, head_of, push_note};
    use crate::git::{AheadBehind, RefKind, RefLabel};

    fn label(name: &str, kind: RefKind, upstream: Option<&str>, ahead: usize) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind,
            target: ObjectId::null(gix::hash::Kind::Sha1),
            is_head: false,
            upstream: upstream.map(|upstream| upstream.to_owned().into()),
            from_upstream: Some(AheadBehind { ahead, behind: 0 }),
            worktree: None,
        }
    }

    #[test]
    fn the_head_is_the_upstream_branch_and_unpushed_commits_go_first() {
        let tracked = label("diff-view", RefKind::LocalBranch, Some("origin/feature/diff-view"), 2);
        assert_eq!(head_of(&tracked, "origin"), Head { branch: "feature/diff-view".into(), push: Push::Commits(2) });
        let level = label("diff-view", RefKind::LocalBranch, Some("origin/feature/diff-view"), 0);
        assert_eq!(head_of(&level, "origin").push, Push::Nothing);
        let unpublished = label("topic/tabs", RefKind::LocalBranch, None, 0);
        assert_eq!(head_of(&unpublished, "origin"), Head { branch: "topic/tabs".into(), push: Push::Publish });
        let remote = label("origin/feature/blame", RefKind::RemoteBranch, None, 0);
        assert_eq!(head_of(&remote, "origin"), Head { branch: "feature/blame".into(), push: Push::Nothing });
    }

    #[test]
    fn the_dialog_says_what_it_pushes_first() {
        assert_eq!(
            push_note("topic/tabs", &Push::Publish, "origin").as_deref(),
            Some("topic/tabs isn't on origin yet. SterneGit pushes it first and makes origin/topic/tabs its upstream.")
        );
        assert_eq!(
            push_note("feature/stash", &Push::Commits(1), "origin").as_deref(),
            Some("feature/stash has 1 commit that origin doesn't. SterneGit pushes it first.")
        );
        assert_eq!(push_note("main", &Push::Nothing, "origin"), None);
    }

    #[test]
    fn several_commits_are_listed_in_the_description() {
        assert_eq!(described(&["Only one".to_owned()]), "");
        assert_eq!(described(&["Newer".to_owned(), "Older".to_owned()]), "- Newer\n- Older");
    }
}
