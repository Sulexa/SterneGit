use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{Context, EventEmitter, SharedString};

use super::{
    operation::{self, Finish, Operated, Operation, Outcome},
    stash_menu::StashAction,
};
use crate::{
    accounts,
    git::{
        self, Attendance, Behind, CheckedOut, FileChange, History, PartAction, PullMode,
        RefLabel, RemoteBranch, Repo, Scope, Side, Stash, Underway, Version, WorkingChanges,
    },
    ui::{Failure, Job, Load, run_in_background},
};

pub(crate) enum RepositoryEvent {
    HistoryRead,
    ChangesRead,
    Committed,
    Unpublished,
    Operated(Operated),
    PullModeNotSaved(Failure),
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
    fn read(path: &Path) -> anyhow::Result<Self> {
        let (repo, history) = Repo::open(path)?;
        let root = resolve(repo.location());
        let pull_mode = repo.pull_mode();
        let has_stash = repo.has_stash();
        Ok(Self {
            repo,
            history: Arc::new(history),
            root,
            pull_mode,
            has_stash,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusCheck {
    Forced,
    UnlessBusy,
}

pub(crate) struct Repository {
    path: PathBuf,
    opened: Load<Opened>,
    changes: Option<Arc<WorkingChanges>>,
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
        Self {
            path,
            opened: Load::Idle,
            changes: None,
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
        let task = run_in_background(
            cx,
            move || Opened::read(&path),
            |this, result, cx| {
                this.opened.finish(result);
                if this.opened.value().is_some() {
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

    pub(crate) fn fetch_if_older_than(&mut self, age: Duration, cx: &mut Context<Self>) {
        if self
            .last_fetch
            .is_none_or(|started| started.elapsed() >= age)
        {
            self.fetch_as(Attendance::Unattended, cx);
        }
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

    pub(crate) fn discard(&mut self, change: FileChange, cx: &mut Context<Self>) {
        self.write_to(move |repo| repo.discard(&change), cx);
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

fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

fn reaching(repo: Repo, attendance: Attendance) -> Repo {
    let access = accounts::access_for_repo(&repo, attendance);
    repo.with_access(access)
}
