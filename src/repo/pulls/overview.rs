use std::{path::Path, sync::Arc};

use sternegit_protocol::{Check, Comment, PullDetails, PullState, PullSummary};
use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, EventEmitter, Hsla, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, UniformListScrollHandle,
    Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        diff::DiffFile,
        h_flex,
        input::{Enter, InputEvent, Textarea, TextareaState},
        notification::Notification,
        scroll::ScrollableElement as _,
        spinner::Spinner,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    list::Hosting,
    looks::{Mark, check_mark, checks_summary, merge_words, readable_head, review_mark, rollup, same_repository, state_mark},
    marks::marks_for,
    merge::open_merge_dialog,
};
use crate::{
    app::connectors::note_failure,
    connect::CallError,
    git::{PullSource, RefKind, fork_branch_name, short_id},
    hosting::{Fix, Trouble},
    repo::{
        commit_files::CommitFiles,
        patch_view::{PatchStats, parse_patch},
        repository::{PullCheckOut, Repository, RepositoryEvent},
    },
    ui::{
        self, Failure, FileView, FileViewToggle, Folded, Load, Look as _, counted, run_async_in_background,
        run_in_background,
    },
};

const AVATAR_SIZE: Pixels = px(24.);
const CHECK_NAME_WIDTH: f32 = 220.;
const COMMENT_ROWS: usize = 3;
const CONVERSATION_PAGES: usize = 10;
const READING: &str = "Reading the pull request…";
const FETCHING: &str = "Fetching the pull request…";
const FETCH_FAILED: &str = "Couldn't fetch this pull request";
const NO_FILES: &str = "This pull request doesn't change any files.";
const NO_DESCRIPTION: &str = "No description.";
const NO_CHECKS: &str = "No checks ran on this commit.";
const NO_COMMENTS: &str = "No comments yet.";
const PLAIN_TEXT: &str = "Sent as you type it. SterneGit shows comments as plain text.";
const COMMENT: &str = "Comment";
const COMMENT_PLACEHOLDER: &str = "Write a comment";
const CHECK_OUT: &str = "Check out";
const CHECKED_OUT: &str = "Checked out";
const MERGE: &str = "Merge…";
const DELETE_BRANCH: &str = "Delete branch";
const DELETE_BRANCH_TOOLTIP: &str =
    "Delete it from the remote, and here unless it is checked out or holds other commits";
const RETRY: &str = "Retry";

pub(crate) enum PullViewEvent {
    OpenFile(DiffFile),
}

pub(crate) struct PullChanges {
    pub(crate) head: ObjectId,
    pub(crate) merge_base: ObjectId,
    pub(crate) subjects: Vec<String>,
    files: Arc<[DiffFile]>,
    stats: PatchStats,
}

struct Asked<T> {
    value: Option<T>,
    trouble: Option<Trouble>,
    task: Option<Task<()>>,
}

impl<T> Default for Asked<T> {
    fn default() -> Self {
        Self {
            value: None,
            trouble: None,
            task: None,
        }
    }
}

impl<T> Asked<T> {
    fn waits(&self) -> bool {
        self.value.is_none() && self.trouble.is_none()
    }

    fn answer(&mut self, answer: Result<T, Trouble>) {
        self.task = None;
        match answer {
            Ok(value) => {
                self.value = Some(value);
                self.trouble = None;
            }
            Err(trouble) => self.trouble = Some(trouble),
        }
    }
}

pub(crate) struct PullView {
    hosting: Hosting,
    repository: Entity<Repository>,
    summary: PullSummary,
    details: Asked<PullDetails>,
    checks: Asked<Vec<Check>>,
    checked: String,
    conversation: Asked<Vec<Comment>>,
    changes: Load<PullChanges>,
    comment: Entity<TextareaState>,
    posting: Option<Task<()>>,
    comment_problem: Option<SharedString>,
    open: Option<SharedString>,
    folded: Folded<SharedString>,
    files_scroll: UniformListScrollHandle,
    scroll: ScrollHandle,
    _subscriptions: [Subscription; 2],
}

impl EventEmitter<PullViewEvent> for PullView {}

impl PullView {
    pub(crate) fn new(
        hosting: Hosting,
        repository: Entity<Repository>,
        summary: PullSummary,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let comment = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(COMMENT_PLACEHOLDER)
                .rows(COMMENT_ROWS)
        });
        let subscriptions = [
            cx.subscribe(&comment, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify()
                }
            }),
            cx.subscribe(&repository, |this, _, event, cx| {
                if matches!(event, RepositoryEvent::Fetched) {
                    this.reread(cx)
                }
            }),
        ];
        let mut this = Self {
            hosting,
            repository,
            checked: summary.head.sha.clone(),
            summary,
            details: Asked::default(),
            checks: Asked::default(),
            conversation: Asked::default(),
            changes: Load::Idle,
            comment,
            posting: None,
            comment_problem: None,
            open: None,
            folded: Folded::default(),
            files_scroll: UniformListScrollHandle::new(),
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        this.reread(cx);
        this
    }

    pub(crate) fn files(&self) -> Option<Arc<[DiffFile]>> {
        self.changes.value().map(|changes| changes.files.clone())
    }

    pub(crate) fn compared(&self) -> Option<(ObjectId, ObjectId)> {
        self.changes.value().map(|changes| (changes.head, changes.merge_base))
    }

    pub(crate) fn mark_open(&mut self, path: Option<SharedString>, cx: &mut Context<Self>) {
        if self.open != path {
            self.open = path;
            cx.notify();
        }
    }

    pub(crate) fn reread(&mut self, cx: &mut Context<Self>) {
        self.read_details(cx);
        self.read_checks(self.checked.clone(), cx);
        self.read_conversation(cx);
    }

    fn fail(&self, error: &CallError, cx: &mut Context<Self>) -> Trouble {
        note_failure(cx, &self.hosting.account, error);
        Trouble::of(error, &self.hosting.account, cx)
    }

    fn read_details(&mut self, cx: &mut Context<Self>) {
        let read = self.hosting.forge.pull(self.summary.number);
        self.details.task = Some(run_async_in_background(cx, read, |this, result, cx| {
            let answer = result.map_err(|error| this.fail(&error, cx));
            if let Ok(details) = &answer {
                this.summary = details.summary.clone();
                if this.checked != details.summary.head.sha {
                    this.read_checks(details.summary.head.sha.clone(), cx);
                }
                let shown_head = this.changes.value().map(|changes| changes.head.to_string());
                if shown_head.as_deref() != Some(details.fetch.sha.as_str()) || this.changes.failure().is_some() {
                    this.read_changes(details, cx);
                }
            }
            this.details.answer(answer);
            cx.notify();
        }));
    }

    fn read_checks(&mut self, sha: String, cx: &mut Context<Self>) {
        self.checked = sha.clone();
        let read = self.hosting.forge.checks(sha);
        self.checks.task = Some(run_async_in_background(cx, read, |this, result, cx| {
            let answer = result.map_err(|error| this.fail(&error, cx));
            this.checks.answer(answer);
            cx.notify();
        }));
    }

    fn read_conversation(&mut self, cx: &mut Context<Self>) {
        let forge = self.hosting.forge.clone();
        let number = self.summary.number;
        let read = async move {
            let mut comments = Vec::new();
            let mut cursor = None;
            for _ in 0..CONVERSATION_PAGES {
                let page = forge.conversation(number, cursor).await?;
                comments.extend(page.items);
                cursor = page.next;
                if cursor.is_none() {
                    break;
                }
            }
            Ok::<_, CallError>(comments)
        };
        self.conversation.task = Some(run_async_in_background(cx, read, |this, result, cx| {
            let answer = result.map_err(|error| this.fail(&error, cx));
            this.conversation.answer(answer);
            cx.notify();
        }));
    }

    fn read_changes(&mut self, details: &PullDetails, cx: &mut Context<Self>) {
        let Some(repo) = self.repository.read(cx).reached_repo() else {
            return;
        };
        let source = self.source(details);
        let task = run_in_background(
            cx,
            move || -> anyhow::Result<PullChanges> {
                let fetched = repo.fetch_pull_head(&source)?;
                let files = parse_patch(&fetched.patch)?;
                Ok(PullChanges {
                    head: fetched.head,
                    merge_base: fetched.merge_base,
                    subjects: fetched.subjects,
                    stats: PatchStats::of(&files),
                    files: files.into(),
                })
            },
            |this, result, cx| {
                this.changes = match result {
                    Ok(changes) => Load::Ready(changes),
                    Err(error) => Load::Failed(Failure::git_reason(&error)),
                };
                cx.notify();
            },
        );
        self.changes.reload(task);
        cx.notify();
    }

    fn source(&self, details: &PullDetails) -> PullSource {
        PullSource {
            remote: self.hosting.remote.clone(),
            url: details.fetch.url.clone(),
            refspec: details.fetch.refspec.clone(),
            number: details.summary.number,
            base_branch: details.summary.base.branch.clone(),
        }
    }

    fn from_this_repository(&self) -> bool {
        self.summary
            .head
            .repo
            .as_ref()
            .is_some_and(|head| same_repository(head, &self.hosting.repo))
    }

    fn checked_out(&self, cx: &App) -> bool {
        let Some(history) = self.repository.read(cx).history() else {
            return false;
        };
        let marks = marks_for(std::slice::from_ref(&self.summary), &history.refs, &self.hosting.remote, &self.hosting.repo);
        history
            .refs
            .iter()
            .any(|label| label.is_head && marks.contains_key(&label.full_name()))
    }

    fn check_out(&mut self, cx: &mut Context<Self>) {
        let how = if self.from_this_repository() {
            PullCheckOut::Tracking {
                remote: self.hosting.remote.clone(),
                branch: self.summary.head.branch.clone(),
            }
        } else {
            let Some(details) = self.details.value.as_ref() else {
                return;
            };
            PullCheckOut::Fork {
                source: self.source(details),
                branch: fork_branch_name(self.summary.number, &self.summary.head.branch),
                owner: self
                    .summary
                    .head
                    .repo
                    .as_ref()
                    .map_or_else(|| "a deleted".to_owned(), |repo| format!("{}'s", repo.owner)),
            }
        };
        self.repository
            .update(cx, |repository, cx| repository.check_out_pull(how, cx));
    }

    fn post_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let body = self.comment.read(cx).value().trim().to_owned();
        if body.is_empty() || self.posting.is_some() {
            return;
        }
        self.comment_problem = None;
        let post = self.hosting.forge.comment(self.summary.number, body);
        self.posting = Some(cx.spawn_in(window, async move |this, cx| {
            let posted = post.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.posting = None;
                match posted {
                    Ok(comment) => {
                        this.conversation.value.get_or_insert_default().push(comment);
                        this.comment.update(cx, |state, cx| state.set_value("", window, cx));
                    }
                    Err(error) => {
                        let trouble = this.fail(&error, cx);
                        this.comment_problem = Some(format!("{} {}", trouble.title, trouble.message).into());
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn open_merge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(details) = self.details.value.clone() else {
            return;
        };
        let subjects = self.changes.value().map(|changes| changes.subjects.clone()).unwrap_or_default();
        let head = readable_head(&self.summary, &self.hosting.repo);
        open_merge_dialog(cx.entity(), self.hosting.forge.clone(), details, head, subjects, window, cx);
    }

    pub(crate) fn merged(&mut self, queued: bool, window: &mut Window, cx: &mut Context<Self>) {
        let summary = &self.summary;
        let mut notification = match queued {
            true => Notification::info(format!(
                "{} finishes the merge on its side. The pull request shows Merged after the next fetch.",
                self.hosting.service
            ))
            .title(format!("Completing #{} into {}", summary.number, summary.base.branch)),
            false => Notification::success(format!("{} isn't needed any more.", summary.head.branch))
                .title(format!("Merged #{} into {}", summary.number, summary.base.branch)),
        };
        if !queued && self.from_this_repository() {
            let view = cx.entity().downgrade();
            notification = notification.action(move |_, _, cx| {
                let view = view.clone();
                let toast = cx.entity().downgrade();
                Button::new("delete-merged-branch")
                    .small()
                    .icon(IconName::Trash)
                    .label(DELETE_BRANCH)
                    .tooltip(DELETE_BRANCH_TOOLTIP)
                    .on_click(move |_, window, cx| {
                        let _ = toast.update(cx, |toast, cx| toast.dismiss(window, cx));
                        let _ = view.update(cx, |view, cx| view.delete_branch(cx));
                    })
            });
        }
        window.push_notification(notification, cx);
        self.repository.update(cx, |repository, cx| repository.fetch(cx));
        self.reread(cx);
    }

    fn delete_branch(&mut self, cx: &mut Context<Self>) {
        let head = self.summary.head.clone();
        let remote = self.hosting.remote.clone();
        let tracking = format!("{remote}/{}", head.branch);
        let local = self.repository.read(cx).history().and_then(|history| {
            history
                .refs
                .iter()
                .find(|label| {
                    label.kind == RefKind::LocalBranch
                        && !label.is_head
                        && label.upstream.as_deref() == Some(tracking.as_str())
                        && label.target.to_string() == head.sha
                })
                .map(|label| label.name.to_string())
        });
        self.repository.update(cx, |repository, cx| {
            repository.delete_pull_branch(remote, head.branch, local, cx)
        });
    }

    pub(crate) fn files_panel(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        let mono = theme.mono_font_family.clone();
        let head = readable_head(&self.summary, &self.hosting.repo);
        let facts = self.changes.value().map(|changes| {
            h_flex()
                .gap_2()
                .xs_text()
                .text_color(muted)
                .child(div().font_family(mono.clone()).child(short_id(changes.head)))
                .child(div().ellipsis().child(format!(
                    "{} since {}, where it branched off",
                    counted(changes.subjects.len(), "commit", "commits"),
                    short_id(changes.merge_base)
                )))
        });
        let stats = self.changes.value().map(|changes| changes.stats).filter(|stats| stats.files > 0);
        let header = v_flex()
            .flex_none()
            .px_4()
            .py_3()
            .gap_1p5()
            .border_b_1()
            .border_color(border)
            .child(div().base_text().font_medium().child(format!("Changes in #{}", self.summary.number)))
            .child(
                h_flex()
                    .gap_1p5()
                    .child(branch_chip(&head, cx))
                    .child(Icon::new(IconName::ArrowRight).xsmall().text_color(muted))
                    .child(branch_chip(&self.summary.base.branch, cx)),
            )
            .children(facts)
            .when_some(stats, |this, stats| {
                this.child(
                    h_flex()
                        .gap_3()
                        .xs_text()
                        .child(div().text_color(muted).child(counted(stats.files, "file", "files")))
                        .child(ui::additions(stats.additions, cx))
                        .child(ui::deletions(stats.deletions, cx))
                        .child(div().flex_1())
                        .child(FileViewToggle),
                )
            });
        let root: Option<Arc<Path>> = self.repository.read(cx).repo().map(|repo| Arc::from(repo.location()));
        let content = match &self.changes {
            Load::Idle | Load::Loading { .. } => ui::loading(FETCHING),
            Load::Failed(failure) => ui::failure(FETCH_FAILED, failure.message(), cx),
            Load::Ready(changes) | Load::Refreshing { value: changes, .. } if changes.files.is_empty() => {
                ui::hint(NO_FILES, cx)
            }
            Load::Ready(changes) | Load::Refreshing { value: changes, .. } => CommitFiles::new(
                changes.files.clone(),
                FileView::current(cx),
                &self.folded,
                self.files_scroll.clone(),
            )
            .open(self.open.clone())
            .root(root)
            .on_open(cx.listener(|this, file: &DiffFile, _, cx| {
                this.open = Some(file.path().clone());
                cx.emit(PullViewEvent::OpenFile(file.clone()));
                cx.notify();
            }))
            .on_fold(cx.listener(|this, folder: &SharedString, _, cx| {
                this.folded.toggle(folder.clone());
                cx.notify();
            }))
            .into_any_element(),
        };
        v_flex()
            .pane()
            .child(header)
            .child(div().fill_height().child(content))
            .into_any_element()
    }

    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        let summary = &self.summary;
        let state = state_mark(summary.state, summary.draft);
        let review = summary
            .review
            .filter(|_| summary.state == PullState::Open)
            .and_then(review_mark);
        let head = readable_head(summary, &self.hosting.repo);
        let open = summary.state == PullState::Open;
        let checked_out = self.checked_out(cx);
        let web = summary.web_url.clone();
        let check_out = (open && self.details.value.is_some()).then(|| {
            if checked_out {
                mark_chip(CHECKED_OUT, IconName::Check, muted)
            } else {
                Button::new("check-out-pull")
                    .small()
                    .icon(IconName::GitBranch)
                    .label(CHECK_OUT)
                    .disabled(self.repository.read(cx).is_writing())
                    .tooltip(if self.from_this_repository() {
                        format!("Check out {} and track it", summary.head.branch)
                    } else {
                        format!(
                            "Check out {}, made from the fork's branch",
                            fork_branch_name(summary.number, &summary.head.branch)
                        )
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.check_out(cx)))
                    .into_any_element()
            }
        });
        v_flex()
            .flex_none()
            .px_5()
            .pt_3p5()
            .pb_3()
            .gap_1p5()
            .border_b_1()
            .border_color(border)
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .flex_wrap()
                            .gap_x_2()
                            .lg_text()
                            .font_semibold()
                            .child(summary.title.clone())
                            .child(div().text_color(muted).child(format!("#{}", summary.number))),
                    )
                    .children(check_out)
                    .child(
                        Button::new("open-pull-on-web")
                            .ghost()
                            .small()
                            .icon(IconName::ExternalLink)
                            .tooltip(format!("Open on {}", self.hosting.service))
                            .on_click(move |_, _, cx| cx.open_url(&web)),
                    ),
            )
            .child(
                h_flex()
                    .gap_1p5()
                    .xs_text()
                    .text_color(muted)
                    .child(mark_chip(state.label, state.icon, state.tint.color(cx)))
                    .children(review.map(|review| mark_chip(review.label, review.icon, review.tint.color(cx))))
                    .child(div().flex_none().child(summary.author.login.clone()))
                    .child(div().flex_none().child(if summary.state == PullState::Merged {
                        "merged"
                    } else {
                        "wants to merge"
                    }))
                    .child(branch_chip(&head, cx))
                    .child(div().flex_none().child("into"))
                    .child(branch_chip(&summary.base.branch, cx)),
            )
            .into_any_element()
    }

    fn description(&self, cx: &App) -> AnyElement {
        let summary = &self.summary;
        let body = self
            .details
            .value
            .as_ref()
            .map(|details| details.body.trim().to_owned())
            .filter(|body| !body.is_empty());
        let muted = cx.theme().muted_foreground;
        comment_block(
            &summary.author.login,
            format!("opened {}", readable_time(&summary.created_at)),
            div()
                .sm_text()
                .when(body.is_none(), |this| this.text_color(muted))
                .child(body.unwrap_or_else(|| NO_DESCRIPTION.to_owned()))
                .into_any_element(),
            cx,
        )
    }

    fn checks_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        let head = |lead: AnyElement, title: &'static str, summary: String| {
            h_flex()
                .min_h(px(40.))
                .px_3()
                .py_2()
                .gap_2()
                .child(lead)
                .child(div().flex_none().sm_text().font_medium().child(title))
                .child(div().flex_1().ellipsis().xs_text().text_color(muted).child(summary))
        };
        let mut rows: Vec<AnyElement> = Vec::new();
        match (&self.checks.value, &self.checks.trouble) {
            (None, None) => rows.push(head(Spinner::new().small().into_any_element(), "Checks", String::new()).into_any_element()),
            (None, Some(trouble)) => rows.push(self.trouble_box(trouble, cx)),
            (Some(checks), _) if checks.is_empty() => rows.push(
                head(
                    Icon::new(IconName::CircleMinus).small().text_color(muted).into_any_element(),
                    "Checks",
                    NO_CHECKS.to_owned(),
                )
                .into_any_element(),
            ),
            (Some(checks), _) => {
                let lead = rollup(checks).map_or_else(|| div().into_any_element(), |state| mark_icon(check_mark(state), cx));
                rows.push(head(lead, "Checks", checks_summary(checks)).into_any_element());
                rows.extend(checks.iter().enumerate().map(|(ix, check)| {
                    let page = check.web_url.clone();
                    h_flex()
                        .h(px(32.))
                        .pl_3()
                        .pr_2()
                        .gap_2()
                        .border_t_1()
                        .border_color(border)
                        .child(mark_icon(check_mark(check.state), cx))
                        .child(div().flex_none().max_w(px(CHECK_NAME_WIDTH)).ellipsis().sm_text().child(check.name.clone()))
                        .child(
                            div()
                                .flex_1()
                                .ellipsis()
                                .xs_text()
                                .text_color(muted)
                                .child(check.description.clone().unwrap_or_default()),
                        )
                        .children(page.map(|page| {
                            Button::new(("check-page", ix))
                                .ghost()
                                .xsmall()
                                .icon(IconName::ExternalLink)
                                .tooltip(format!("Open on {}", self.hosting.service))
                                .on_click(move |_, _, cx| cx.open_url(&page))
                        }))
                        .into_any_element()
                }));
            }
        }
        boxed(rows, cx)
    }

    fn merge_box(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let details = self.details.value.as_ref()?;
        let words = merge_words(details);
        let muted = cx.theme().muted_foreground;
        let writing = self.repository.read(cx).is_writing();
        let action = match details.summary.state {
            PullState::Open if self.hosting.merges => Some(
                Button::new("merge-pull")
                    .small()
                    .icon(IconName::GitMerge)
                    .label(MERGE)
                    .when(words.can_merge, |button| button.primary())
                    .disabled(!words.can_merge)
                    .on_click(cx.listener(|this, _, window, cx| this.open_merge(window, cx)))
                    .into_any_element(),
            ),
            PullState::Merged if self.from_this_repository() && self.head_still_there(cx) => Some(
                Button::new("delete-pull-branch")
                    .small()
                    .icon(IconName::Trash)
                    .label(DELETE_BRANCH)
                    .tooltip(DELETE_BRANCH_TOOLTIP)
                    .disabled(writing)
                    .on_click(cx.listener(|this, _, _, cx| this.delete_branch(cx)))
                    .into_any_element(),
            ),
            _ => None,
        };
        Some(boxed(
            vec![
                h_flex()
                    .min_h(px(56.))
                    .px_3()
                    .py_2()
                    .gap_3()
                    .child(mark_icon(words.mark, cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(div().sm_text().font_medium().child(words.title))
                            .child(div().xs_text().text_color(muted).child(words.about)),
                    )
                    .children(action)
                    .into_any_element(),
            ],
            cx,
        ))
    }

    fn head_still_there(&self, cx: &App) -> bool {
        let tracking = format!("{}/{}", self.hosting.remote, self.summary.head.branch);
        self.repository.read(cx).history().is_some_and(|history| {
            history
                .refs
                .iter()
                .any(|label| label.kind == RefKind::RemoteBranch && label.name.as_str() == tracking)
        })
    }

    fn conversation(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let count = self.conversation.value.as_ref().map(Vec::len);
        let comments: Vec<AnyElement> = match (&self.conversation.value, &self.conversation.trouble) {
            (Some(comments), _) if comments.is_empty() => {
                vec![div().sm_text().text_color(muted).child(NO_COMMENTS).into_any_element()]
            }
            (Some(comments), _) => comments
                .iter()
                .map(|comment| {
                    comment_block(
                        &comment.author.login,
                        readable_time(&comment.created_at),
                        div().sm_text().child(comment.body.clone()).into_any_element(),
                        cx,
                    )
                })
                .collect(),
            (None, Some(trouble)) => vec![self.trouble_box(trouble, cx)],
            (None, None) => vec![Spinner::new().small().into_any_element()],
        };
        v_flex()
            .flex_none()
            .gap_3()
            .child(
                h_flex()
                    .gap_1p5()
                    .sm_text()
                    .font_semibold()
                    .child("Conversation")
                    .children(count.map(|count| div().xs_text().font_normal().text_color(muted).child(count.to_string()))),
            )
            .children(comments)
            .when(self.summary.state == PullState::Open, |this| this.child(self.comment_box(cx)))
            .into_any_element()
    }

    fn comment_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, red) = (theme.muted_foreground, theme.red);
        let posting = self.posting.is_some();
        let written = !self.comment.read(cx).value().trim().is_empty();
        let note = match &self.comment_problem {
            Some(problem) => div().flex_1().xs_text().text_color(red).child(problem.clone()),
            None => div().flex_1().xs_text().text_color(muted).child(PLAIN_TEXT),
        };
        v_flex()
            .flex_none()
            .gap_2()
            .capture_action(cx.listener(|this, enter: &Enter, window, cx| {
                if enter.secondary {
                    cx.stop_propagation();
                    this.post_comment(window, cx);
                }
            }))
            .child(Textarea::new(&self.comment).disabled(posting))
            .child(
                h_flex().gap_2().child(note).child(
                    Button::new("post-comment")
                        .primary()
                        .small()
                        .icon(IconName::MessageSquare)
                        .label(COMMENT)
                        .loading(posting)
                        .disabled(!written || posting)
                        .on_click(cx.listener(|this, _, window, cx| this.post_comment(window, cx))),
                ),
            )
            .into_any_element()
    }

    fn trouble_box(&self, trouble: &Trouble, cx: &mut Context<Self>) -> AnyElement {
        let action = matches!(trouble.fix, Fix::Retry).then(|| {
            Button::new("pull-retry")
                .small()
                .label(RETRY)
                .on_click(cx.listener(|this, _, _, cx| this.reread(cx)))
                .into_any_element()
        });
        trouble.render(action, cx).into_any_element()
    }
}

impl Render for PullView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.header(cx);
        let body = match (&self.details.value, &self.details.trouble) {
            (None, Some(trouble)) => v_flex().p_5().child(self.trouble_box(trouble, cx)).into_any_element(),
            (None, None) if self.details.waits() => ui::loading(READING),
            _ => div()
                .id("pull-overview")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(
                    v_flex()
                        .px_5()
                        .py_4()
                        .gap_4()
                        .child(self.description(cx))
                        .child(self.checks_box(cx))
                        .children(self.merge_box(cx))
                        .child(self.conversation(cx)),
                )
                .vertical_scrollbar(&self.scroll)
                .into_any_element(),
        };
        v_flex().pane().child(header).child(div().fill_height().child(body))
    }
}

fn readable_time(timestamp: &str) -> String {
    let shown: String = timestamp.chars().take(16).map(|character| if character == 'T' { ' ' } else { character }).collect();
    if shown.len() == 16 { shown } else { timestamp.to_owned() }
}

fn comment_block(login: &str, when: String, body: AnyElement, cx: &App) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let login = SharedString::from(login.to_owned());
    h_flex()
        .flex_none()
        .items_start()
        .gap_2p5()
        .child(ui::author_avatar(&login, &login, AVATAR_SIZE))
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    h_flex()
                        .gap_2()
                        .child(div().sm_text().font_medium().child(login.clone()))
                        .child(div().xs_text().text_color(muted).child(when)),
                )
                .child(body),
        )
        .into_any_element()
}

fn boxed(rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    v_flex()
        .flex_none()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(8.))
        .overflow_hidden()
        .children(rows)
        .into_any_element()
}

fn mark_icon(mark: Mark, cx: &App) -> AnyElement {
    let color = mark.tint.color(cx);
    if mark.spins {
        Spinner::new().small().color(color).into_any_element()
    } else {
        Icon::new(mark.icon).small().text_color(color).into_any_element()
    }
}

fn mark_chip(label: &'static str, icon: IconName, color: Hsla) -> AnyElement {
    h_flex()
        .flex_none()
        .h(px(20.))
        .px_1p5()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(color.opacity(0.35))
        .bg(color.opacity(0.1))
        .xs_text()
        .text_color(color)
        .child(Icon::new(icon).xsmall().text_color(color))
        .child(label)
        .into_any_element()
}

fn branch_chip(name: &str, cx: &App) -> AnyElement {
    let theme = cx.theme();
    h_flex()
        .flex_none()
        .max_w(px(220.))
        .h(px(20.))
        .px_1p5()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(theme.border)
        .xs_text()
        .text_color(theme.muted_foreground)
        .child(Icon::new(IconName::GitBranch).xsmall())
        .child(div().ellipsis().child(name.to_owned()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::readable_time;

    #[test]
    fn server_times_read_as_date_and_minute() {
        assert_eq!(readable_time("2026-11-03T18:50:12Z"), "2026-11-03 18:50");
        assert_eq!(readable_time("yesterday"), "yesterday");
    }
}
