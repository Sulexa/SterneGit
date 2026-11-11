use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gix::ObjectId;
use gpui_kit::{Context, EventEmitter, SharedString};

use super::{
    operation::{self, Finish, Operated, Operation, Outcome},
    stash_menu::StashAction,
};
use crate::{
    accounts,
    git::{
        self, Attendance, Behind, CheckedOut, FileChange, History, PartAction, PullMode,
        PullSource, RefLabel, RemoteBranch, Repo, ResetMode, Scope, Side, Stash, Underway,
        Version, WorkingChanges, WorktreeChanges, WorktreeStart, changes_elsewhere, short_id,
    },
    ui::{Failure, Job, Load, run_in_background},
};

pub(crate) enum RepositoryEvent {
    HistoryRead,
    ChangesRead,
    Fetched,
    Committed,
    Unpublished,
    Operated(Operated),
    Reworded(Reworded),
    Squashed(Squashed),
    PullModeNotSaved(Failure),
}

pub(crate) struct Reworded {
    pub(crate) id: ObjectId,
    pub(crate) outcome: Result<ObjectId, Failure>,
}

pub(crate) struct Squashed {
    pub(crate) count: usize,
    pub(crate) outcome: Result<ObjectId, Failure>,
}

pub(crate) enum PullCheckOut {
    Tracking {
        remote: String,
        branch: String,
    },
    Fork {
        source: PullSource,
        branch: String,
        owner: String,
    },
}

#[derive(Clone)]
struct Opened {
    repo: Repo,
    history: Arc<History>,
    root: PathBuf,
    pull_mode: PullMode,
    has_stash: bool,
}

impl Opened {
    fn read(path: &Path, reading: &Reading) -> anyhow::Result<Self> {
        let (repo, history) = match reading {
            Reading::WholeRepository => {
                let (repo, history) = Repo::open(path)?;
                (repo, Arc::new(history))
            }
            Reading::ChangesOnly(history) => (Repo::open_worktree(path)?, history.clone()),
        };
        let root = resolve(repo.location());
        let pull_mode = repo.pull_mode();
        let has_stash = repo.has_stash();
        Ok(Self {
            repo,
            history,
            root,
            pull_mode,
            has_stash,
        })
    }
}

#[derive(Clone)]
enum Reading {
    WholeRepository,
    ChangesOnly(Arc<History>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusCheck {
    Forced,
    UnlessBusy,
}

pub(crate) struct Repository {
    path: PathBuf,
    linked_worktree: bool,
    reading: Reading,
    opened: Load<Opened>,
    changes: Option<Arc<WorkingChanges>>,
    elsewhere: Vec<WorktreeChanges>,
    worktree_check: Job,
    status_read: Job,
    ref_check: Job,
    fetch: Job,
    last_fetch: Option<Instant>,
    fetch_failure: Option<Failure>,
    write: Job,
    committing: bool,
    running: Option<Operation>,
    finishing: Option<Finish>,
    write_failure: Option<Failure>,
    chosen_pull_mode: Option<PullMode>,
    pull_mode_save: Job,
    push_once_read: bool,
}

impl EventEmitter<RepositoryEvent> for Repository {}

impl Repository {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self::reading(path, Reading::WholeRepository)
    }

    pub(crate) fn changes_of_worktree(location: PathBuf, history: Arc<History>) -> Self {
        Self::reading(location, Reading::ChangesOnly(history))
    }

    fn reading(path: PathBuf, reading: Reading) -> Self {
        Self {
            linked_worktree: git::is_linked_worktree(&path),
            path,
            reading,
            opened: Load::Idle,
            changes: None,
            elsewhere: Vec::new(),
            worktree_check: Job::default(),
            status_read: Job::default(),
            ref_check: Job::default(),
            fetch: Job::default(),
            last_fetch: None,
            fetch_failure: None,
            write: Job::default(),
            committing: false,
            running: None,
            finishing: None,
            write_failure: None,
            chosen_pull_mode: None,
            pull_mode_save: Job::default(),
            push_once_read: false,
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn is_linked_worktree(&self) -> bool {
        self.linked_worktree
    }

    pub(crate) fn name(&self) -> SharedString {
        match self.repo() {
            Some(repo) => repo.name(),
            None => git::folder_name(&self.path),
        }
    }

    pub(crate) fn repo(&self) -> Option<&Repo> {
        self.opened.value().map(|opened| &opened.repo)
    }

    pub(crate) fn history(&self) -> Option<&Arc<History>> {
        self.opened.value().map(|opened| &opened.history)
    }

    pub(crate) fn head_label(&self) -> Option<SharedString> {
        self.history()?.head_label.clone()
    }

    pub(crate) fn is_reading_history(&self) -> bool {
        self.opened.is_loading()
    }

    pub(crate) fn failure(&self) -> Option<&Failure> {
        self.opened.failure()
    }

    pub(crate) fn shows(&self, path: &Path) -> bool {
        self.opened
            .value()
            .is_some_and(|opened| opened.root == path)
            || resolve(&self.path) == path
    }

    pub(crate) fn changes(&self) -> Option<&Arc<WorkingChanges>> {
        self.changes.as_ref()
    }

    pub(crate) fn changes_elsewhere(&self, location: &Path) -> Option<&Arc<WorkingChanges>> {
        self.elsewhere
            .iter()
            .find(|read| *read.location == *location)
            .map(|read| &read.changes)
    }

    pub(crate) fn has_changes(&self) -> bool {
        self.changes
            .as_ref()
            .is_some_and(|changes| !changes.is_empty())
    }

    pub(crate) fn is_writing(&self) -> bool {
        self.write.is_running()
    }

    pub(crate) fn is_committing(&self) -> bool {
        self.committing
    }

    pub(crate) fn running(&self) -> Option<Operation> {
        self.running
    }

    pub(crate) fn finishing(&self) -> Option<Finish> {
        self.finishing
    }

    pub(crate) fn underway(&self) -> Option<&Underway> {
        self.changes.as_ref()?.underway.as_ref()
    }

    pub(crate) fn has_stash(&self) -> bool {
        self.opened.value().is_some_and(|opened| opened.has_stash)
    }

    pub(crate) fn pull_mode(&self) -> PullMode {
        self.chosen_pull_mode
            .or_else(|| self.opened.value().map(|opened| opened.pull_mode))
            .unwrap_or_default()
    }

    pub(crate) fn staged_count(&self) -> usize {
        self.changes
            .as_ref()
            .map_or(0, |changes| changes.staged.len())
    }

    pub(crate) fn write_failure(&self) -> Option<&Failure> {
        self.write_failure.as_ref()
    }

    pub(crate) fn is_fetching(&self) -> bool {
        self.fetch.is_running()
    }

    pub(crate) fn fetch_failure(&self) -> Option<&Failure> {
        self.fetch_failure.as_ref()
    }

    pub(crate) fn read_if_never_read(&mut self, cx: &mut Context<Self>) {
        if self.opened.is_idle() {
            self.refresh(cx);
        }
    }

    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let reading = self.reading.clone();
        let task = run_in_background(
            cx,
            move || Opened::read(&path, &reading),
            |this, result, cx| {
                this.opened.finish(result);
                if let Some(opened) = this.opened.value() {
                    if matches!(this.reading, Reading::WholeRepository) {
                        this.elsewhere = opened.history.worktree_changes.clone();
                    }
                    this.read_status(StatusCheck::Forced, cx);
                    if std::mem::take(&mut this.push_once_read) {
                        this.push(cx);
                    }
                } else {
                    this.status_read.cancel();
                    this.changes = None;
                }
                cx.emit(RepositoryEvent::HistoryRead);
                cx.notify();
            },
        );
        self.opened.reload(task);
        cx.notify();
    }

    pub(crate) fn check_for_changes(&mut self, cx: &mut Context<Self>) {
        self.read_status(StatusCheck::UnlessBusy, cx);
        if matches!(self.reading, Reading::ChangesOnly(_)) {
            return;
        }
        self.check_worktrees(cx);
        if self.opened.is_loading() || self.ref_check.is_running() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        let task = run_in_background(
            cx,
            move || repo.ref_digest(),
            |this, result, cx| {
                this.ref_check.finish();
                let shown = this.history().map(|history| history.digest);
                if let Ok(digest) = result
                    && Some(digest) != shown
                    && !this.opened.is_loading()
                {
                    this.refresh(cx);
                }
            },
        );
        self.ref_check.run(task);
    }

    fn check_worktrees(&mut self, cx: &mut Context<Self>) {
        if self.worktree_check.is_running() || self.write.is_running() || self.opened.is_loading() {
            return;
        }
        let Some(history) = self.history().cloned() else {
            return;
        };
        if history.worktrees.iter().all(|worktree| worktree.is_here) {
            return;
        }
        let worktrees = history.worktrees.clone();
        let task = run_in_background(
            cx,
            move || changes_elsewhere(&worktrees),
            |this, read, cx| {
                this.worktree_check.finish();
                let moved = this
                    .history()
                    .is_some_and(|history| changes_rows_moved(history, &read));
                this.elsewhere = read;
                if moved && !this.opened.is_loading() {
                    this.refresh(cx);
                }
                cx.notify();
            },
        );
        self.worktree_check.run(task);
    }

    pub(crate) fn fetch_if_older_than(&mut self, age: Duration, cx: &mut Context<Self>) {
        if self
            .last_fetch
            .is_none_or(|started| started.elapsed() >= age)
        {
            self.fetch_unattended(cx);
        }
    }

    pub(crate) fn fetch_unattended(&mut self, cx: &mut Context<Self>) {
        self.fetch_as(Attendance::Unattended, cx);
    }

    pub(crate) fn fetch(&mut self, cx: &mut Context<Self>) {
        self.fetch_as(Attendance::Attended, cx);
    }

    fn fetch_as(&mut self, attendance: Attendance, cx: &mut Context<Self>) {
        if self.fetch.is_running() || self.running.is_some() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.last_fetch = Some(Instant::now());
        let task = run_in_background(
            cx,
            move || reaching(repo, attendance).fetch(),
            |this, result, cx| {
                this.fetch.finish();
                match result {
                    Ok(_) => {
                        this.fetch_failure = None;
                        this.check_for_changes(cx);
                        cx.emit(RepositoryEvent::Fetched);
                    }
                    Err(error) => this.fetch_failure = Some(Failure::git_reason(&error)),
                }
                cx.notify();
            },
        );
        self.fetch.run(task);
        cx.notify();
    }

    pub(crate) fn move_across(&mut self, from: Side, scope: Scope, cx: &mut Context<Self>) {
        self.write_to(
            move |repo| match from {
                Side::Unstaged => repo.stage(&scope),
                Side::Staged => repo.unstage(&scope),
            },
            cx,
        );
    }

    pub(crate) fn apply_part(
        &mut self,
        action: PartAction,
        patch: Vec<u8>,
        cx: &mut Context<Self>,
    ) {
        self.write_to(move |repo| repo.apply_part(action, &patch), cx);
    }

    pub(crate) fn rewrite_guid(&mut self, change: crate::unity::GuidChange, keep_old: bool, cx: &mut Context<Self>) {
        self.write_to(
            move |repo| {
                if keep_old {
                    crate::unity::keep_old_guid(repo, &change)
                } else {
                    crate::unity::point_users_to_new_guid(repo, &change)
                }
            },
            cx,
        );
    }

    pub(crate) fn discard(&mut self, changes: Vec<FileChange>, cx: &mut Context<Self>) {
        self.write_to(move |repo| repo.discard(&changes), cx);
    }

    fn write_to(
        &mut self,
        work: impl FnOnce(&Repo) -> anyhow::Result<()> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.write.is_running() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.write_failure = None;
        self.status_read.cancel();
        let task = run_in_background(
            cx,
            move || work(&repo),
            |this, result, cx| {
                this.write.finish();
                if let Err(error) = result {
                    this.write_failure = Some(Failure::git_reason(&error));
                }
                this.read_status(StatusCheck::Forced, cx);
                cx.notify();
            },
        );
        self.write.run(task);
        cx.notify();
    }

    pub(crate) fn commit(&mut self, message: String, cx: &mut Context<Self>) {
        if self.write.is_running() || self.staged_count() == 0 {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.write_failure = None;
        self.committing = true;
        self.status_read.cancel();
        let task = run_in_background(
            cx,
            move || repo.commit(&message),
            |this, result, cx| {
                this.write.finish();
                this.committing = false;
                match result {
                    Ok(()) => {
                        cx.emit(RepositoryEvent::Committed);
                        this.refresh(cx);
                    }
                    Err(error) => {
                        this.write_failure = Some(Failure::git_reason(&error));
                        this.read_status(StatusCheck::Forced, cx);
                    }
                }
                cx.notify();
            },
        );
        self.write.run(task);
        cx.notify();
    }

    pub(crate) fn reword(&mut self, id: ObjectId, message: String, cx: &mut Context<Self>) {
        if self.write.is_running() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.status_read.cancel();
        let task = run_in_background(
            cx,
            move || repo.reword(id, &message),
            move |this, result, cx| {
                this.write.finish();
                let outcome = result.map_err(|error| Failure::git_reason(&error));
                cx.emit(RepositoryEvent::Reworded(Reworded { id, outcome }));
                this.refresh(cx);
                cx.notify();
            },
        );
        self.write.run(task);
        cx.notify();
    }

    pub(crate) fn squash(
        &mut self,
        newest: ObjectId,
        oldest: ObjectId,
        count: usize,
        message: String,
        cx: &mut Context<Self>,
    ) {
        if self.write.is_running() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.status_read.cancel();
        let task = run_in_background(
            cx,
            move || repo.squash(newest, oldest, &message),
            move |this, result, cx| {
                this.write.finish();
                let outcome = result.map_err(|error| Failure::git_reason(&error));
                cx.emit(RepositoryEvent::Squashed(Squashed { count, outcome }));
                this.refresh(cx);
                cx.notify();
            },
        );
        self.write.run(task);
        cx.notify();
    }

    pub(crate) fn pull(&mut self, cx: &mut Context<Self>) {
        let mode = self.pull_mode();
        self.operate(
            Operation::Pull,
            move |repo| repo.pull(mode).map(operation::pulled_back),
            cx,
        );
    }

    pub(crate) fn push_once_reread(&mut self, cx: &mut Context<Self>) {
        self.push_once_read = true;
        self.refresh(cx);
    }

    pub(crate) fn push(&mut self, cx: &mut Context<Self>) {
        if self.repo().is_some_and(|repo| !repo.has_remotes()) {
            cx.emit(RepositoryEvent::Unpublished);
            return;
        }
        self.operate(
            Operation::Push,
            |repo| repo.push().map(operation::pushed),
            cx,
        );
    }

    pub(crate) fn pull_then_push(&mut self, behind: Behind, cx: &mut Context<Self>) {
        self.write_and_report(
            Some(Operation::Push),
            operation::PULL_AND_PUSH_FAILED.into(),
            move |repo| repo.pull_then_push(&behind).map(operation::caught_up),
            cx,
        );
    }

    pub(crate) fn force_push(&mut self, behind: Behind, cx: &mut Context<Self>) {
        self.operate(
            Operation::Push,
            move |repo| repo.force_push(&behind).map(operation::pushed),
            cx,
        );
    }

    pub(crate) fn create_branch(
        &mut self,
        name: String,
        start: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.operate(
            Operation::Branch,
            move |repo| {
                repo.create_branch(&name, start.as_deref()).map(|carried| {
                    operation::switched(operation::branch_created(&name), &name, carried)
                })
            },
            cx,
        );
    }

    pub(crate) fn check_out(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        let failed = operation::check_out_failed(label.local_name());
        self.write_and_report(
            None,
            failed,
            move |repo| {
                let (checked_out, carried) = match label.on_remote() {
                    Some(remote) => repo.check_out_remote(&label.name, remote.branch)?,
                    None => (
                        CheckedOut::Existing(label.name.to_string()),
                        repo.check_out(&label.name)?,
                    ),
                };
                let done = operation::checked_out(&checked_out);
                Ok(operation::switched(done, label.local_name(), carried))
            },
            cx,
        );
    }

    pub(crate) fn pull_branch(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        if label.is_head {
            return self.pull(cx);
        }
        self.operate(
            Operation::Pull,
            move |repo| {
                repo.pull_branch(&label.name)
                    .map(|pulled| operation::pulled(&pulled))
            },
            cx,
        );
    }

    pub(crate) fn push_branch(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        if label.is_head {
            return self.push(cx);
        }
        self.operate(
            Operation::Push,
            move |repo| repo.push_branch(&label.name).map(operation::pushed),
            cx,
        );
    }

    pub(crate) fn merge(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        let Some((into, revision)) = self.checked_out_and_revision(&label) else {
            return;
        };
        let from = label.name;
        self.write_and_report(
            None,
            operation::merge_failed(&from, &into),
            move |repo| {
                repo.merge(&revision)
                    .map(|merged| operation::merged(&from, &into, merged))
            },
            cx,
        );
    }

    pub(crate) fn rebase_onto(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        let Some((branch, revision)) = self.checked_out_and_revision(&label) else {
            return;
        };
        let onto = label.name;
        self.write_and_report(
            None,
            operation::rebase_failed(&branch, &onto),
            move |repo| {
                repo.rebase_onto(&revision)
                    .map(|rebased| operation::rebased(&branch, &onto, rebased))
            },
            cx,
        );
    }

    pub(crate) fn check_out_commit(&mut self, id: ObjectId, cx: &mut Context<Self>) {
        let short = short_id(id).to_string();
        self.write_and_report(
            None,
            operation::check_out_failed(&short),
            move |repo| {
                let carried = repo.check_out_commit(id)?;
                Ok(operation::switched(operation::checked_out_commit(&short), &short, carried))
            },
            cx,
        );
    }

    pub(crate) fn cherry_pick(&mut self, id: ObjectId, cx: &mut Context<Self>) {
        let short = short_id(id).to_string();
        let branch = self.checked_out_name();
        self.write_and_report(
            None,
            operation::cherry_pick_failed(&short, &branch),
            move |repo| {
                repo.cherry_pick(id)
                    .map(|picked| operation::cherry_picked(&short, &branch, picked))
            },
            cx,
        );
    }

    pub(crate) fn revert(&mut self, id: ObjectId, cx: &mut Context<Self>) {
        let short = short_id(id).to_string();
        let branch = self.checked_out_name();
        let merge = self.history().is_some_and(|history| {
            history
                .row_of(&id)
                .is_some_and(|row| history.commits[row].parents.len() > 1)
        });
        self.write_and_report(
            None,
            operation::revert_failed(&short),
            move |repo| {
                repo.revert(id, merge)
                    .map(|picked| operation::reverted(&short, &branch, picked))
            },
            cx,
        );
    }

    pub(crate) fn cherry_pick_several(&mut self, newest_first: Vec<ObjectId>, cx: &mut Context<Self>) {
        let count = newest_first.len();
        let branch = self.checked_out_name();
        let oldest_first: Vec<ObjectId> = newest_first.into_iter().rev().collect();
        self.write_and_report(
            None,
            operation::cherry_pick_several_failed(count, &branch),
            move |repo| {
                repo.cherry_pick_several(&oldest_first)
                    .map(|picked| operation::cherry_picked_several(count, &branch, picked))
            },
            cx,
        );
    }

    pub(crate) fn revert_several(&mut self, newest_first: Vec<ObjectId>, cx: &mut Context<Self>) {
        let count = newest_first.len();
        let branch = self.checked_out_name();
        let merges = self.history().is_some_and(|history| {
            newest_first.iter().any(|id| {
                history
                    .row_of(id)
                    .is_some_and(|row| history.commits[row].parents.len() > 1)
            })
        });
        self.write_and_report(
            None,
            operation::revert_several_failed(count),
            move |repo| {
                repo.revert_several(&newest_first, merges)
                    .map(|picked| operation::reverted_several(count, &branch, picked))
            },
            cx,
        );
    }

    pub(crate) fn reset(&mut self, id: ObjectId, mode: ResetMode, cx: &mut Context<Self>) {
        let short = short_id(id).to_string();
        let branch = self.checked_out_name();
        self.write_and_report(
            None,
            operation::reset_failed(&branch, &short),
            move |repo| {
                repo.reset(id, mode)
                    .map(|()| operation::reset_done(&branch, &short, mode))
            },
            cx,
        );
    }

    pub(crate) fn create_tag(
        &mut self,
        name: String,
        id: ObjectId,
        message: Option<String>,
        push_to: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let short = short_id(id).to_string();
        self.write_and_report(
            None,
            operation::tag_failed(&name),
            move |repo| {
                repo.create_tag(&name, id, message.as_deref())?;
                let pushed = push_to
                    .as_deref()
                    .map(|remote| (remote, repo.push_tag(remote, &name).map(drop)));
                Ok(operation::tag_created(&name, &short, pushed))
            },
            cx,
        );
    }

    pub(crate) fn push_remote(&self) -> Option<String> {
        self.repo().and_then(|repo| repo.push_remote())
    }

    pub(crate) fn check_out_tag(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::check_out_failed(&label.name),
            move |repo| {
                let carried = repo.check_out_commit(label.target)?;
                let done = operation::checked_out_commit(&label.name);
                Ok(operation::switched(done, &label.name, carried))
            },
            cx,
        );
    }

    pub(crate) fn push_tag(&mut self, label: RefLabel, remote: String, cx: &mut Context<Self>) {
        self.write_and_report(
            Some(Operation::Push),
            operation::tag_push_failed(&label.name),
            move |repo| {
                repo.push_tag(&remote, &label.name)
                    .map(|pushed| operation::tag_pushed(&label.name, &remote, pushed))
            },
            cx,
        );
    }

    pub(crate) fn delete_tag(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::delete_failed(&label),
            move |repo| {
                repo.delete_tag(&label.name)?;
                Ok(operation::tag_deleted(&label.name))
            },
            cx,
        );
    }

    pub(crate) fn delete_tag_there(&mut self, label: RefLabel, remote: String, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::tag_delete_there_failed(&label.name, &remote),
            move |repo| {
                repo.delete_remote_tag(&remote, &label.name)?;
                Ok(operation::tag_deleted_there(&label.name, &remote))
            },
            cx,
        );
    }

    pub(crate) fn add_worktree(
        &mut self,
        folder: PathBuf,
        start: WorktreeStart,
        open: bool,
        cx: &mut Context<Self>,
    ) {
        let name = git::folder_name(&folder).to_string();
        self.write_and_report(
            None,
            operation::worktree_add_failed(&name),
            move |repo| {
                repo.add_worktree(&folder, &start)?;
                let checked_out = match &start {
                    WorktreeStart::Branch(branch) | WorktreeStart::Tracking { branch, .. } => branch,
                    WorktreeStart::NewBranch { name, .. } => name,
                };
                Ok(Outcome::WorktreeAdded {
                    done: operation::worktree_added(&name, checked_out),
                    open: open.then_some(folder),
                })
            },
            cx,
        );
    }

    pub(crate) fn remove_worktrees(
        &mut self,
        removed: Vec<(PathBuf, SharedString)>,
        delete_branch: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let names: Vec<SharedString> = removed.iter().map(|(_, name)| name.clone()).collect();
        self.write_and_report(
            None,
            operation::worktree_remove_failed(&names),
            move |repo| {
                let mut folders_left = Vec::new();
                for (location, _) in &removed {
                    folders_left.extend(repo.remove_worktree(location)?);
                }
                let branch = delete_branch.map(|branch| {
                    let deleted = repo.delete_branch(&branch);
                    (branch, deleted)
                });
                Ok(operation::worktrees_removed(&names, &folders_left, branch))
            },
            cx,
        );
    }

    pub(crate) fn lock_worktree(
        &mut self,
        location: PathBuf,
        name: SharedString,
        reason: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let locked = reason.is_some();
        self.write_and_report(
            None,
            operation::worktree_lock_failed(&name, locked),
            move |repo| {
                match &reason {
                    Some(reason) => repo.lock_worktree(&location, reason)?,
                    None => repo.unlock_worktree(&location)?,
                }
                Ok(operation::worktree_locked(&name, locked))
            },
            cx,
        );
    }

    pub(crate) fn forget_missing_worktrees(&mut self, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::FORGET_MISSING_FAILED.into(),
            |repo| {
                repo.forget_missing_worktrees()
                    .map(|()| SharedString::from(operation::FORGOT_MISSING_WORKTREES))
            },
            cx,
        );
    }

    fn checked_out_name(&self) -> String {
        self.history()
            .and_then(|history| history.branch.as_deref())
            .unwrap_or("HEAD")
            .to_owned()
    }

    pub(crate) fn reached_repo(&self) -> Option<Repo> {
        self.repo()
            .cloned()
            .map(|repo| reaching(repo, Attendance::Attended))
    }

    pub(crate) fn check_out_pull(&mut self, how: PullCheckOut, cx: &mut Context<Self>) {
        let failed: SharedString = "Couldn't check out the pull request".into();
        self.write_and_report(
            None,
            failed,
            move |repo| match how {
                PullCheckOut::Tracking { remote, branch } => {
                    let (checked_out, carried) = repo.check_out_pull_branch(&remote, &branch)?;
                    let done = operation::checked_out(&checked_out);
                    Ok(operation::switched(done, &branch, carried))
                }
                PullCheckOut::Fork { source, branch, owner } => {
                    let carried = repo.check_out_fork_pull(&source, &branch)?;
                    let done = operation::checked_out_fork(&branch, &owner);
                    Ok(operation::switched(done, &branch, carried))
                }
            },
            cx,
        );
    }

    pub(crate) fn delete_pull_branch(
        &mut self,
        remote: String,
        branch: String,
        local: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let failed: SharedString = format!("Couldn't delete {branch}").into();
        self.write_and_report(
            None,
            failed,
            move |repo| {
                repo.delete_remote_branch(&remote, &branch)?;
                if let Some(local) = &local {
                    repo.delete_branch(local)?;
                }
                Ok(operation::pull_branch_deleted(&branch, &remote, local.as_deref()))
            },
            cx,
        );
    }

    pub(crate) fn delete_branch(&mut self, label: RefLabel, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::delete_failed(&label),
            move |repo| {
                match label.on_remote() {
                    Some(RemoteBranch { remote, branch }) => {
                        repo.delete_remote_branch(remote, branch)?
                    }
                    None => repo.delete_branch(&label.name)?,
                }
                Ok(operation::deleted(&label))
            },
            cx,
        );
    }

    pub(crate) fn finish_underway(&mut self, finish: Finish, cx: &mut Context<Self>) {
        let Some(underway) = self.underway().cloned() else {
            return;
        };
        if self.write.is_running() {
            return;
        }
        self.finishing = Some(finish);
        self.write_and_report(
            None,
            operation::finish_failed(&underway, finish),
            move |repo| match finish {
                Finish::Continue => repo
                    .continue_underway()
                    .map(|continued| operation::continued(&underway, continued)),
                Finish::Abort => repo
                    .abort_underway()
                    .map(|carried| operation::aborted(&underway, carried)),
            },
            cx,
        );
    }

    pub(crate) fn resolve(&mut self, change: FileChange, text: Vec<u8>, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::resolve_failed(&change.path),
            move |repo| repo.resolve(&change, &text).map(|()| Outcome::Quiet),
            cx,
        );
    }

    pub(crate) fn keep(&mut self, change: FileChange, version: Version, cx: &mut Context<Self>) {
        self.write_and_report(
            None,
            operation::resolve_failed(&change.path),
            move |repo| repo.keep(&change, version).map(|()| Outcome::Quiet),
            cx,
        );
    }

    fn checked_out_and_revision(&self, label: &RefLabel) -> Option<(SharedString, String)> {
        let history = self.history()?;
        let checked_out = history.branch.clone()?;
        Some((checked_out, label.revision(&history.refs)))
    }

    pub(crate) fn stash(&mut self, cx: &mut Context<Self>) {
        self.operate(
            Operation::Stash,
            |repo| {
                repo.stash()
                    .map(|()| SharedString::from(operation::STASHED))
            },
            cx,
        );
    }

    pub(crate) fn pop_stash(&mut self, cx: &mut Context<Self>) {
        self.operate(
            Operation::Pop,
            |repo| {
                repo.pop_stash()
                    .map(|()| SharedString::from(operation::POPPED))
            },
            cx,
        );
    }

    pub(crate) fn act_on_stash(
        &mut self,
        stash: Stash,
        action: StashAction,
        cx: &mut Context<Self>,
    ) {
        self.write_and_report(
            None,
            action.failed(&stash).into(),
            move |repo| {
                match action {
                    StashAction::Apply => repo.apply_stash(stash.id)?,
                    StashAction::Pop => repo.pop_stash_entry(stash.id)?,
                    StashAction::Delete => repo.drop_stash(stash.id)?,
                }
                Ok(SharedString::from(action.done(&stash)))
            },
            cx,
        );
    }

    pub(crate) fn choose_pull_mode(&mut self, mode: PullMode, cx: &mut Context<Self>) {
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.chosen_pull_mode = Some(mode);
        let task = run_in_background(
            cx,
            move || repo.save_pull_mode(mode),
            |this, result, cx| {
                this.pull_mode_save.finish();
                if let Err(error) = result {
                    cx.emit(RepositoryEvent::PullModeNotSaved(Failure::git_reason(
                        &error,
                    )));
                }
            },
        );
        self.pull_mode_save.run(task);
        cx.notify();
    }

    fn operate<R: Into<Outcome> + Send + 'static>(
        &mut self,
        operation: Operation,
        work: impl FnOnce(&Repo) -> anyhow::Result<R> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        self.write_and_report(Some(operation), operation.failed().into(), work, cx);
    }

    fn write_and_report<R: Into<Outcome> + Send + 'static>(
        &mut self,
        header_button: Option<Operation>,
        failed: SharedString,
        work: impl FnOnce(&Repo) -> anyhow::Result<R> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.write.is_running() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.running = header_button;
        self.status_read.cancel();
        let task = run_in_background(
            cx,
            move || work(&reaching(repo, Attendance::Attended)),
            move |this, result, cx| {
                this.write.finish();
                this.running = None;
                this.finishing = None;
                let outcome = result
                    .map(Into::into)
                    .map_err(|error| operation::failure(&error));
                cx.emit(RepositoryEvent::Operated(Operated { failed, outcome }));
                this.refresh(cx);
                cx.notify();
            },
        );
        self.write.run(task);
        cx.notify();
    }

    fn read_status(&mut self, check: StatusCheck, cx: &mut Context<Self>) {
        let Some(repo) = self.repo() else { return };
        let busy = self.status_read.is_running() || self.write.is_running();
        if !repo.has_work_tree() || (check == StatusCheck::UnlessBusy && busy) {
            return;
        }
        let repo = repo.clone();
        let task = run_in_background(
            cx,
            move || repo.status(),
            |this, result, cx| {
                this.status_read.finish();
                if let Ok(changes) = result {
                    this.install_changes(changes, cx);
                }
            },
        );
        self.status_read.run(task);
    }

    fn install_changes(&mut self, changes: WorkingChanges, cx: &mut Context<Self>) {
        if self
            .changes
            .as_ref()
            .is_some_and(|shown| shown.digest == changes.digest)
        {
            return;
        }
        self.changes = Some(Arc::new(changes));
        cx.emit(RepositoryEvent::ChangesRead);
        cx.notify();
    }
}

fn changes_rows_moved(history: &History, read: &[WorktreeChanges]) -> bool {
    let shown_in_history = |read: &&WorktreeChanges| {
        !read.changes.is_empty()
            && read
                .changes
                .head
                .is_some_and(|head| history.row_of(&head).is_some())
    };
    let changed: Vec<&WorktreeChanges> = read.iter().filter(shown_in_history).collect();
    changed.len() != history.locations_with_changes_rows().count()
        || changed
            .iter()
            .any(|read| !history.shows_changes_of(&read.location, read.changes.head))
}

fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

fn reaching(repo: Repo, attendance: Attendance) -> Repo {
    let access = accounts::access_for_repo(&repo, attendance);
    repo.with_access(access)
}
