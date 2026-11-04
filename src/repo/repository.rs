use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gpui_kit::{Context, EventEmitter, SharedString};

use crate::{
    git::{self, History, Repo, Scope, Side, WorkingChanges},
    ui::{Failure, Job, Load, run_in_background},
};

pub(crate) enum RepositoryEvent {
    HistoryRead,
    ChangesRead,
}

#[derive(Clone)]
struct Opened {
    repo: Repo,
    history: Arc<History>,
    root: PathBuf,
}

impl Opened {
    fn read(path: &Path) -> anyhow::Result<Self> {
        let (repo, history) = Repo::open(path)?;
        let root = resolve(repo.location());
        Ok(Self {
            repo,
            history: Arc::new(history),
            root,
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
    write_failure: Option<Failure>,
}

impl EventEmitter<RepositoryEvent> for Repository {}

impl Repository {
    pub(crate) fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            path,
            opened: Load::Idle,
            changes: None,
            status_read: Job::default(),
            ref_check: Job::default(),
            fetch: Job::default(),
            last_fetch: None,
            fetch_failure: None,
            write: Job::default(),
            write_failure: None,
        };
        this.refresh(cx);
        this
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

    pub(crate) fn write_failure(&self) -> Option<&Failure> {
        self.write_failure.as_ref()
    }

    pub(crate) fn is_fetching(&self) -> bool {
        self.fetch.is_running()
    }

    pub(crate) fn fetch_failure(&self) -> Option<&Failure> {
        self.fetch_failure.as_ref()
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
            self.fetch(cx);
        }
    }

    pub(crate) fn fetch(&mut self, cx: &mut Context<Self>) {
        if self.fetch.is_running() {
            return;
        }
        let Some(repo) = self.repo().cloned() else {
            return;
        };
        self.last_fetch = Some(Instant::now());
        let task = run_in_background(
            cx,
            move || repo.fetch(),
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
            move || match from {
                Side::Unstaged => repo.stage(&scope),
                Side::Staged => repo.unstage(&scope),
            },
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
