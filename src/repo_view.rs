//! One open repository: refs on the left, the commit graph in the middle, and
//! the selected commit's message and diff on the right.
//!
//! Uncommitted changes sit in a row above the graph. Selecting it lists the
//! unstaged and staged files on the right, and opening one of them compares it
//! in the middle, side by side or unified.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use gix::ObjectId;
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _, StyledExt as _,
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    diff::{Diff, DiffMode, DiffState},
    h_flex,
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    scroll::Scrollbar,
    spinner::Spinner,
    status_bar::StatusBar,
    v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, ClipboardItem, ContentMask, Context, Entity,
    FocusHandle, Hsla, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    PathBuilder, Pixels, Render, ScrollStrategy, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Task, UniformListScrollHandle, WeakEntity, Window, canvas, div,
    fill, point, prelude::FluentBuilder as _, px, size, transparent_black, uniform_list,
};

use crate::{
    app::{CloseDiff, CopyHash, Fetch, NewerCommit, OlderCommit, Refresh},
    graph::{Edge, Row},
    repository::{
        self, ChangeKind, CommitDetails, FileChange, HISTORY_LIMIT, History, RefKind, RefLabel,
        Side, WorkingChanges,
    },
};

/// The height of every list row. The graph is painted row by row, so its lines
/// only join up when all rows are exactly this tall.
const ROW_HEIGHT: Pixels = px(28.);
const LANE_WIDTH: Pixels = px(12.);
/// Lanes beyond this are clipped rather than pushing the messages off screen.
const MAX_VISIBLE_LANES: usize = 32;
const NODE_RADIUS: Pixels = px(4.5);
const LANE_COLORS: usize = 6;
const MAX_BADGES: usize = 3;
const AUTHOR_WIDTH: Pixels = px(120.);
const DATE_WIDTH: Pixels = px(104.);
const ID_WIDTH: Pixels = px(56.);

#[derive(Clone, Copy)]
enum SidebarRow {
    Header(RefKind, usize),
    Ref(usize),
}

#[derive(Clone, Copy)]
enum ChangeRow {
    Header(Side, usize),
    File(Side, usize),
}

/// The changed file compared in the middle pane.
#[derive(Clone, PartialEq, Eq)]
struct OpenChange {
    side: Side,
    path: SharedString,
}

enum ChangeDiff {
    Loading,
    Ready { files: usize, additions: usize, deletions: usize },
    Failed(SharedString),
}

enum Details {
    None,
    Loading,
    Ready {
        body: SharedString,
        files: usize,
        additions: usize,
        deletions: usize,
    },
    Failed(SharedString),
}

pub(crate) struct RepoView {
    path: PathBuf,
    /// Where the repository was found, resolved, once it has been read: two
    /// paths inside one repository open the same tab.
    root: Option<PathBuf>,
    history: Option<Arc<History>>,
    sidebar: Arc<Vec<SidebarRow>>,
    loading: bool,
    error: Option<SharedString>,
    selected: Option<usize>,
    details: Details,
    diff: Entity<DiffState>,
    /// Bumped by every load, so a slow result can't replace a newer one.
    history_revision: usize,
    details_revision: usize,
    /// Whether a check for moved refs is running.
    checking: bool,
    fetching: bool,
    /// When the last fetch started, so a failing one isn't retried at once.
    last_fetch: Option<Instant>,
    fetch_error: Option<SharedString>,
    history_focus: FocusHandle,
    history_scroll: UniformListScrollHandle,
    refs_scroll: UniformListScrollHandle,
    _history_task: Task<()>,
    _details_task: Task<()>,
    _check_task: Task<()>,
    _fetch_task: Task<()>,
    /// The uncommitted changes, once read.
    changes: Option<Arc<WorkingChanges>>,
    change_rows: Arc<Vec<ChangeRow>>,
    /// Whether the uncommitted changes are selected instead of a commit.
    showing_changes: bool,
    open_change: Option<OpenChange>,
    change_diff: ChangeDiff,
    change_diff_state: Entity<DiffState>,
    change_diff_revision: usize,
    status_revision: usize,
    checking_status: bool,
    /// Whether a stage or unstage is running.
    staging: bool,
    staging_error: Option<SharedString>,
    changes_scroll: UniformListScrollHandle,
    _status_task: Task<()>,
    _change_diff_task: Task<()>,
    _staging_task: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl RepoView {
    pub(crate) fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let diff = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Unified));
        // Side by side by default: a working change is read as a comparison.
        let change_diff_state = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Split));
        let history_focus = cx.focus_handle().tab_stop(true);
        let subscriptions = vec![
            cx.observe(&diff, |_, _, cx| cx.notify()),
            cx.observe(&change_diff_state, |_, _, cx| cx.notify()),
        ];
        let mut this = Self {
            path,
            root: None,
            history: None,
            sidebar: Arc::default(),
            loading: false,
            error: None,
            selected: None,
            details: Details::None,
            diff,
            history_revision: 0,
            details_revision: 0,
            checking: false,
            fetching: false,
            last_fetch: None,
            fetch_error: None,
            history_focus,
            history_scroll: UniformListScrollHandle::new(),
            refs_scroll: UniformListScrollHandle::new(),
            _history_task: Task::ready(()),
            _details_task: Task::ready(()),
            _check_task: Task::ready(()),
            _fetch_task: Task::ready(()),
            changes: None,
            change_rows: Arc::default(),
            showing_changes: false,
            open_change: None,
            change_diff: ChangeDiff::Loading,
            change_diff_state,
            change_diff_revision: 0,
            status_revision: 0,
            checking_status: false,
            staging: false,
            staging_error: None,
            changes_scroll: UniformListScrollHandle::new(),
            _status_task: Task::ready(()),
            _change_diff_task: Task::ready(()),
            _staging_task: Task::ready(()),
            _subscriptions: subscriptions,
        };
        this.refresh(cx);
        this
    }

    /// The repository's folder name, or the opened path's until it has been read.
    pub(crate) fn name(&self) -> SharedString {
        match &self.history {
            Some(history) => history.name.clone(),
            None => self.path.file_name().map_or_else(
                || self.path.display().to_string().into(),
                |name| name.to_string_lossy().into_owned().into(),
            ),
        }
    }

    pub(crate) fn head(&self) -> Option<SharedString> {
        self.history.as_ref()?.head.clone()
    }

    pub(crate) fn is_loading(&self) -> bool {
        self.loading
    }

    pub(crate) fn has_failed(&self) -> bool {
        self.error.is_some()
    }

    /// Whether this tab shows the repository at `path`, already resolved.
    pub(crate) fn shows(&self, path: &Path) -> bool {
        self.root.as_deref() == Some(path) || resolve(&self.path) == path
    }

    pub(crate) fn focus_handle(&self) -> FocusHandle {
        self.history_focus.clone()
    }

    fn selected_commit(&self) -> Option<&repository::Commit> {
        if self.showing_changes {
            return None;
        }
        self.history.as_ref()?.commits.get(self.selected?)
    }

    fn has_changes(&self) -> bool {
        self.changes.as_ref().is_some_and(|changes| !changes.is_empty())
    }

    /// Reads the uncommitted changes again. Unless `force`d, a read already
    /// running or a stage in progress makes this one unnecessary.
    fn check_status(&mut self, force: bool, cx: &mut Context<Self>) {
        let Some(history) = &self.history else { return };
        if !history.has_work_tree() || (!force && (self.checking_status || self.staging)) {
            return;
        }
        let job = history.status_job();
        self.status_revision += 1;
        let revision = self.status_revision;
        self.checking_status = true;
        self._status_task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                if this.status_revision != revision {
                    return;
                }
                this.checking_status = false;
                // A failed read changes nothing; the next one tries again.
                if let Ok(changes) = result {
                    this.install_status(changes, cx);
                }
            });
        });
    }

    fn install_status(&mut self, changes: WorkingChanges, cx: &mut Context<Self>) {
        if self.changes.as_ref().is_some_and(|shown| shown.digest == changes.digest) {
            return;
        }
        let mut rows = Vec::new();
        for side in [Side::Unstaged, Side::Staged] {
            let files = changes.files(side);
            rows.push(ChangeRow::Header(side, files.len()));
            rows.extend((0..files.len()).map(|ix| ChangeRow::File(side, ix)));
        }
        self.change_rows = Arc::new(rows);
        // With nothing else to show, show the changes.
        if self.selected.is_none() && !changes.is_empty() {
            self.showing_changes = true;
        }
        let changes = Arc::new(changes);
        self.changes = Some(changes.clone());
        // The open file follows its change: staged in full, it moves sides.
        if let Some(open) = self.open_change.clone() {
            let other = match open.side {
                Side::Unstaged => Side::Staged,
                Side::Staged => Side::Unstaged,
            };
            let found = [open.side, other].into_iter().find_map(|side| {
                changes.find(side, &open.path).map(|change| (side, change.clone()))
            });
            match found {
                Some((side, change)) => self.load_change(side, change, cx),
                None => self.close_change(cx),
            }
        }
        cx.notify();
    }

    fn select_changes(&mut self, cx: &mut Context<Self>) {
        if !self.showing_changes {
            self.showing_changes = true;
            cx.notify();
        }
    }

    /// Compares a changed file in the middle pane.
    fn open_change(&mut self, side: Side, change: FileChange, cx: &mut Context<Self>) {
        self.showing_changes = true;
        self.load_change(side, change, cx);
    }

    fn load_change(&mut self, side: Side, change: FileChange, cx: &mut Context<Self>) {
        let Some(history) = &self.history else { return };
        let open = OpenChange { side, path: change.path.clone() };
        // Reading the same file again keeps the old patch up until the new one
        // is ready, so an edit doesn't flash a spinner.
        let job = history.change_diff_job(change, side);
        if self.open_change.as_ref() != Some(&open) {
            self.change_diff = ChangeDiff::Loading;
            self.change_diff_state.update(cx, |state, cx| state.set_files([], cx));
        }
        self.open_change = Some(open);
        self.change_diff_revision += 1;
        let revision = self.change_diff_revision;
        self._change_diff_task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                if this.change_diff_revision != revision {
                    return;
                }
                this.change_diff = match result {
                    Ok(files) => {
                        let (additions, deletions) = files.iter().fold((0, 0), |(a, d), file| {
                            (a + file.additions(), d + file.deletions())
                        });
                        let count = files.len();
                        this.change_diff_state.update(cx, |state, cx| state.set_files(files, cx));
                        ChangeDiff::Ready { files: count, additions, deletions }
                    }
                    Err(error) => ChangeDiff::Failed(format!("{error:#}").into()),
                };
                cx.notify();
            });
        });
        cx.notify();
    }

    fn close_change(&mut self, cx: &mut Context<Self>) {
        if self.open_change.take().is_some() {
            self.change_diff_revision += 1;
            self._change_diff_task = Task::ready(());
            self.change_diff_state.update(cx, |state, cx| state.set_files([], cx));
            cx.notify();
        }
    }

    fn set_change_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
        self.change_diff_state.update(cx, |state, cx| state.set_mode(mode, cx));
    }

    /// Stages (or, from the staged side, unstages) `changes`, or every change
    /// on that side when it is empty.
    fn stage(&mut self, side: Side, changes: Vec<FileChange>, cx: &mut Context<Self>) {
        let Some(history) = &self.history else { return };
        if self.staging {
            return;
        }
        let job: Box<dyn FnOnce() -> anyhow::Result<()> + Send> = match side {
            Side::Unstaged => Box::new(history.stage_job(changes)),
            Side::Staged => Box::new(history.unstage_job(changes)),
        };
        self.staging = true;
        self.staging_error = None;
        // A status read meanwhile could show the files half moved.
        self.status_revision += 1;
        self.checking_status = false;
        self._staging_task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                this.staging = false;
                if let Err(error) = result {
                    let message = format!("{error:#}");
                    let reason = message.lines().rfind(|line| !line.trim().is_empty());
                    this.staging_error =
                        Some(reason.unwrap_or("unknown error").trim().to_owned().into());
                }
                this.check_status(true, cx);
                cx.notify();
            });
        });
        cx.notify();
    }

    /// Stages or unstages the file open in the middle pane.
    fn stage_open_change(&mut self, cx: &mut Context<Self>) {
        let Some(open) = self.open_change.clone() else { return };
        let change = self.changes.as_ref().and_then(|changes| changes.find(open.side, &open.path));
        if let Some(change) = change.cloned() {
            self.stage(open.side, vec![change], cx);
        }
    }

    /// Reads the history again. The current one stays on screen until the new
    /// one is ready, and the selection survives when its commit still exists.
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        let path = self.path.clone();
        let previous = self
            .history
            .as_ref()
            .zip(self.selected)
            .and_then(|(history, row)| history.commits.get(row))
            .map(|commit| commit.id);
        self.history_revision += 1;
        let revision = self.history_revision;
        self.loading = true;
        self.error = None;
        self._history_task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { repository::load(&path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.history_revision != revision {
                    return;
                }
                this.loading = false;
                match result {
                    Ok(history) => {
                        this.install_history(history, previous, cx);
                        this.check_status(true, cx);
                    }
                    Err(error) => {
                        this.history = None;
                        this.root = None;
                        this.sidebar = Arc::default();
                        this.selected = None;
                        this.clear_details(cx);
                        this.error = Some(format!("{error:#}").into());
                    }
                }
                cx.notify();
            });
        });
        cx.notify();
    }

    /// Reads the history again only if HEAD or a ref moved since it was read:
    /// a commit, checkout or fetch made outside FastGit.
    pub(crate) fn check_for_changes(&mut self, cx: &mut Context<Self>) {
        self.check_status(false, cx);
        if self.loading || self.checking {
            return;
        }
        let Some(history) = &self.history else { return };
        let job = history.state_job();
        self.checking = true;
        self._check_task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                this.checking = false;
                let shown = this.history.as_ref().map(|history| history.state);
                // A failed check changes nothing; the next one tries again.
                if let Ok(state) = result
                    && Some(state) != shown
                    && !this.loading
                {
                    this.refresh(cx);
                }
            });
        });
    }

    /// Fetches unless a fetch started less than `age` ago.
    pub(crate) fn fetch_if_older_than(&mut self, age: Duration, cx: &mut Context<Self>) {
        if self.last_fetch.is_none_or(|started| started.elapsed() >= age) {
            self.fetch(cx);
        }
    }

    /// Fetches every remote in the background, then shows what moved.
    pub(crate) fn fetch(&mut self, cx: &mut Context<Self>) {
        if self.fetching {
            return;
        }
        let Some(history) = &self.history else { return };
        let job = history.fetch_job();
        self.fetching = true;
        self.last_fetch = Some(Instant::now());
        self._fetch_task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| {
                this.fetching = false;
                match result {
                    Ok(_) => {
                        this.fetch_error = None;
                        this.check_for_changes(cx);
                    }
                    // Git's last line says why, e.g. "fatal: Authentication failed".
                    Err(error) => {
                        let message = format!("{error:#}");
                        let reason = message.lines().rfind(|line| !line.trim().is_empty());
                        this.fetch_error =
                            Some(reason.unwrap_or("unknown error").trim().to_owned().into());
                    }
                }
                cx.notify();
            });
        });
        cx.notify();
    }

    fn install_history(&mut self, history: History, previous: Option<ObjectId>, cx: &mut Context<Self>) {
        let row = previous
            .or(history.head_id)
            .and_then(|id| history.rows.get(&id).copied())
            .or((!history.commits.is_empty()).then_some(0));
        self.root = Some(resolve(history.location()));
        self.sidebar = Arc::new(sidebar_rows(&history));
        self.history = Some(Arc::new(history));
        self.selected = None;
        match row {
            Some(row) => self.select(row, cx),
            None => self.clear_details(cx),
        }
    }

    fn select(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(history) = self.history.clone() else { return };
        let Some(commit) = history.commits.get(row) else { return };
        let was_showing_changes = std::mem::take(&mut self.showing_changes);
        if self.selected == Some(row) {
            if was_showing_changes {
                self.history_scroll.scroll_to_item(row, ScrollStrategy::Nearest);
                cx.notify();
            }
            return;
        }
        self.selected = Some(row);
        self.history_scroll.scroll_to_item(row, ScrollStrategy::Nearest);
        self.details_revision += 1;
        let revision = self.details_revision;
        self.details = Details::Loading;
        // Clear the old patch at once so nothing acts on the wrong commit.
        self.diff.update(cx, |state, cx| state.set_files([], cx));
        let job = history.details_job(commit.id);
        self._details_task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { job() }).await;
            let _ = this.update(cx, |this, cx| this.install_details(revision, result, cx));
        });
        cx.notify();
    }

    fn install_details(
        &mut self,
        revision: usize,
        result: anyhow::Result<CommitDetails>,
        cx: &mut Context<Self>,
    ) {
        if self.details_revision != revision {
            return;
        }
        self.details = match result {
            Ok(details) => {
                let (additions, deletions) = details
                    .files
                    .iter()
                    .fold((0, 0), |(added, deleted), file| {
                        (added + file.additions(), deleted + file.deletions())
                    });
                let files = details.files.len();
                self.diff.update(cx, |state, cx| state.set_files(details.files, cx));
                Details::Ready {
                    body: details.body,
                    files,
                    additions,
                    deletions,
                }
            }
            Err(error) => Details::Failed(format!("{error:#}").into()),
        };
        cx.notify();
    }

    fn clear_details(&mut self, cx: &mut Context<Self>) {
        self.details_revision += 1;
        self.details = Details::None;
        self._details_task = Task::ready(());
        self.diff.update(cx, |state, cx| state.set_files([], cx));
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(count) = self.history.as_ref().map(|history| history.commits.len()) else {
            return;
        };
        if self.showing_changes {
            if delta > 0 && count > 0 {
                self.select(0, cx);
            }
            return;
        }
        if count == 0 {
            return;
        }
        // Above the newest commit are the uncommitted changes.
        if delta < 0 && self.selected == Some(0) && self.has_changes() {
            self.select_changes(cx);
            return;
        }
        let row = self
            .selected
            .unwrap_or(0)
            .saturating_add_signed(delta)
            .min(count - 1);
        self.select(row, cx);
    }

    fn reveal_ref(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(history) = self.history.clone() else { return };
        let Some(label) = history.refs.get(ix) else { return };
        match history.rows.get(&label.target) {
            Some(&row) => {
                self.select(row, cx);
                self.history_scroll.scroll_to_item(row, ScrollStrategy::Center);
            }
            None => window.push_notification(
                format!(
                    "{} points to a commit older than the newest {} shown.",
                    label.name,
                    thousands(HISTORY_LIMIT)
                ),
                cx,
            ),
        }
    }

    pub(crate) fn copy_hash(&self, window: &mut Window, cx: &mut Context<Self>) {
        // With the changes selected there's no commit to copy.
        if let Some(commit) = self.selected_commit() {
            cx.write_to_clipboard(ClipboardItem::new_string(commit.id.to_string()));
            window.push_notification("Commit hash copied", cx);
        }
    }

    fn render_body(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(history) = self.history.clone() else {
            return if self.loading {
                loading_state("Reading history…")
            } else {
                self.render_failure(cx)
            };
        };
        let rem = window.rem_size();
        h_resizable("fastgit-panes")
            .child(
                resizable_panel()
                    .size(rem * 14.)
                    .size_range(rem * 10. ..rem * 30.)
                    .child(self.render_refs(&history, cx)),
            )
            .child(
                resizable_panel()
                    .size_range(rem * 30. ..rem * 400.)
                    .child(if self.open_change.is_some() {
                        self.render_change_diff(cx)
                    } else {
                        self.render_history(&history, cx)
                    }),
            )
            .child(
                resizable_panel()
                    .size(rem * 30.)
                    .size_range(rem * 22. ..rem * 90.)
                    .child(if self.showing_changes {
                        self.render_changes(cx)
                    } else {
                        self.render_details(&history, cx)
                    }),
            )
            .into_any_element()
    }

    fn render_failure(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .p_6()
            .child(Icon::new(IconName::GitMerge).large().text_color(muted))
            .child(div().text_lg().font_medium().child("Couldn't open this repository"))
            .child(
                div()
                    .max_w(px(520.))
                    .text_sm()
                    .text_center()
                    .text_color(muted)
                    .child(self.error.clone().unwrap_or_default()),
            )
            .child(
                Button::new("retry")
                    .label("Retry")
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
            .into_any_element()
    }

    fn render_refs(&self, history: &Arc<History>, cx: &mut Context<Self>) -> AnyElement {
        let history = history.clone();
        let rows = self.sidebar.clone();
        let selected = self.selected_commit().map(|commit| commit.id);
        let owner = cx.entity().downgrade();
        div()
            .id("refs")
            .relative()
            .size_full()
            .min_h_0()
            .bg(cx.theme().sidebar)
            .child(
                uniform_list("ref-list", rows.len(), move |range, _, cx| {
                    range
                        .map(|ix| match rows[ix] {
                            SidebarRow::Header(kind, count) => section_header(kind, count, cx),
                            SidebarRow::Ref(ix) => {
                                ref_row(&history.refs[ix], ix, selected, owner.clone(), cx)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&self.refs_scroll),
            )
            .child(Scrollbar::vertical(&self.refs_scroll))
            .into_any_element()
    }

    fn render_history(&self, history: &Arc<History>, cx: &mut Context<Self>) -> AnyElement {
        let history = history.clone();
        let selected = self.selected.filter(|_| !self.showing_changes);
        let owner = cx.entity().downgrade();
        let style = RowStyle::new(cx);
        let wip = self.changes.as_ref().filter(|changes| !changes.is_empty()).map(|changes| {
            wip_row(changes, self.showing_changes, &style, owner.clone(), cx)
        });
        let list = div()
            .id("history")
            .relative()
            .flex_1()
            .w_full()
            .min_h_0()
            .track_focus(&self.history_focus)
            .key_context("History")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.history_focus.focus(window, cx)),
            )
            .child(
                uniform_list("commits", history.commits.len(), move |range, _, cx| {
                    // The graph is only as wide as the rows on screen need, so one
                    // wide stretch of history doesn't squeeze every message.
                    let lanes = history.graph.rows[range.clone()]
                        .iter()
                        .map(|row| row.width as usize)
                        .max()
                        .unwrap_or(1)
                        .clamp(1, MAX_VISIBLE_LANES);
                    let graph_width = LANE_WIDTH * lanes as f32 + px(6.);
                    range
                        .map(|row| {
                            commit_row(
                                &history,
                                row,
                                selected == Some(row),
                                graph_width,
                                &style,
                                owner.clone(),
                                cx,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&self.history_scroll),
            )
            .child(Scrollbar::vertical(&self.history_scroll));
        v_flex()
            .size_full()
            .min_h_0()
            .children(wip)
            .child(list)
            .into_any_element()
    }

    /// The unstaged and staged files, each with a button to move it across.
    fn render_changes(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(changes) = self.changes.clone() else {
            return loading_state("Reading changes…");
        };
        let muted = cx.theme().muted_foreground;
        let header = v_flex()
            .flex_none()
            .px_4()
            .py_3()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().text_base().font_medium().child("Uncommitted changes"))
            .child(div().text_xs().text_color(muted).child(format!(
                "{} unstaged · {} staged",
                thousands(changes.unstaged.len()),
                thousands(changes.staged.len())
            )))
            .when_some(self.staging_error.clone(), |this, error| {
                this.child(div().text_xs().text_color(cx.theme().red).child(error))
            });
        if changes.is_empty() {
            return v_flex()
                .size_full()
                .child(header)
                .child(centered_message("No uncommitted changes.", cx))
                .into_any_element();
        }
        let rows = self.change_rows.clone();
        let open = self.open_change.clone();
        let staging = self.staging;
        let owner = cx.entity().downgrade();
        let list = div()
            .id("changes")
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                uniform_list("change-list", rows.len(), move |range, _, cx| {
                    range
                        .map(|ix| match rows[ix] {
                            ChangeRow::Header(side, count) => {
                                change_header(side, count, staging, owner.clone(), cx)
                            }
                            ChangeRow::File(side, ix) => {
                                let change = &changes.files(side)[ix];
                                let selected = open
                                    .as_ref()
                                    .is_some_and(|open| open.side == side && open.path == change.path);
                                change_row(change, side, selected, staging, owner.clone(), cx)
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .size_full()
                .track_scroll(&self.changes_scroll),
            )
            .child(Scrollbar::vertical(&self.changes_scroll));
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(header)
            .child(list)
            .into_any_element()
    }

    /// One changed file compared with what it was, in the middle pane.
    fn render_change_diff(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(open) = self.open_change.clone() else {
            return div().into_any_element();
        };
        let muted = cx.theme().muted_foreground;
        let change = self.changes.as_ref().and_then(|changes| changes.find(open.side, &open.path));
        let kind = change.map_or(ChangeKind::Modified, |change| change.kind);
        let original = change.and_then(|change| change.original.clone());
        let mode = self.change_diff_state.read(cx).mode();
        let stats = match &self.change_diff {
            ChangeDiff::Ready { additions, deletions, .. } => Some((*additions, *deletions)),
            _ => None,
        };
        let (action_label, action_icon, action_tip) = match open.side {
            Side::Unstaged => ("Stage", IconName::Plus, "Stage this file"),
            Side::Staged => ("Unstage", IconName::Minus, "Unstage this file"),
        };
        let toolbar = h_flex()
            .flex_none()
            .w_full()
            .min_w_0()
            .h(px(40.))
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(kind_badge(kind, cx))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(match &original {
                        Some(original) => SharedString::from(format!("{original} → {}", open.path)),
                        None => open.path.clone(),
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .text_xs()
                    .text_color(muted)
                    .child(match open.side {
                        Side::Unstaged => "unstaged",
                        Side::Staged => "staged",
                    }),
            )
            .when_some(stats, |this, (additions, deletions)| {
                this.child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(cx.theme().green)
                        .child(format!("+{}", thousands(additions))),
                )
                .child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(cx.theme().red)
                        .child(format!("−{}", thousands(deletions))),
                )
            })
            .child(div().flex_1())
            .child(
                Button::new("diff-split")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Columns2)
                    .label("Split")
                    .selected(mode == DiffMode::Split)
                    .tooltip("Compare side by side")
                    .on_click(cx.listener(|this, _, _, cx| this.set_change_mode(DiffMode::Split, cx))),
            )
            .child(
                Button::new("diff-unified")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Rows2)
                    .label("Unified")
                    .selected(mode == DiffMode::Unified)
                    .tooltip("Show changes in one column")
                    .on_click(
                        cx.listener(|this, _, _, cx| this.set_change_mode(DiffMode::Unified, cx)),
                    ),
            )
            .child(
                Button::new("diff-stage")
                    .xsmall()
                    .icon(action_icon)
                    .label(action_label)
                    .tooltip(action_tip)
                    .disabled(self.staging || change.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.stage_open_change(cx))),
            )
            .child(
                Button::new("diff-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip_with_action("Back to the graph", &CloseDiff, Some("FastGit"))
                    .on_click(cx.listener(|this, _, _, cx| this.close_change(cx))),
            );
        let content = match &self.change_diff {
            ChangeDiff::Loading => loading_state("Reading changes…"),
            ChangeDiff::Failed(error) => v_flex()
                .size_full()
                .p_4()
                .gap_2()
                .child(div().font_medium().child("Couldn't compare this file"))
                .child(div().text_sm().text_color(muted).child(error.clone()))
                .into_any_element(),
            ChangeDiff::Ready { files: 0, .. } => centered_message(
                "No changes to show: only line endings or file mode differ.",
                cx,
            ),
            ChangeDiff::Ready { .. } => Diff::new(&self.change_diff_state)
                .line_number(true)
                .soft_wrap(true)
                .header_visible(false)
                .into_any_element(),
        };
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(toolbar)
            .child(div().flex_1().min_h_0().child(content))
            .into_any_element()
    }

    fn render_details(&self, history: &History, cx: &mut Context<Self>) -> AnyElement {
        let Some(commit) = self.selected.and_then(|row| history.commits.get(row)) else {
            return centered_message("Select a commit to see its changes.", cx);
        };
        let muted = cx.theme().muted_foreground;
        let content = match &self.details {
            Details::None => div().into_any_element(),
            Details::Loading => loading_state("Reading commit…"),
            Details::Failed(error) => v_flex()
                .size_full()
                .p_4()
                .gap_2()
                .child(div().font_medium().child("Couldn't read this commit"))
                .child(div().text_sm().text_color(muted).child(error.clone()))
                .into_any_element(),
            Details::Ready { files: 0, .. } => {
                centered_message("This commit doesn't change any files.", cx)
            }
            Details::Ready { .. } => Diff::new(&self.diff)
                .line_number(true)
                .soft_wrap(true)
                .into_any_element(),
        };
        let body = match &self.details {
            Details::Ready { body, .. } if !body.is_empty() => Some(body.clone()),
            _ => None,
        };
        let stats = match &self.details {
            Details::Ready {
                files,
                additions,
                deletions,
                ..
            } if *files > 0 => Some((*files, *additions, *deletions)),
            _ => None,
        };
        let parents = commit
            .parents
            .iter()
            .map(|id| id.to_hex_with_len(7).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(
                v_flex()
                    .flex_none()
                    .px_4()
                    .py_3()
                    .gap_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().text_base().font_medium().child(if commit.subject.is_empty() {
                        "(no message)".into()
                    } else {
                        commit.subject.clone()
                    }))
                    .when_some(body, |this, body| {
                        this.child(
                            div()
                                .id("message-body")
                                .max_h(px(140.))
                                .overflow_y_scroll()
                                .text_sm()
                                .text_color(muted)
                                .child(body),
                        )
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .text_color(muted)
                            .child(commit.author.clone())
                            .child("·")
                            .child(commit.authored.clone()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .text_xs()
                            .text_color(muted)
                            .child(
                                div()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child(commit.short_id.clone()),
                            )
                            .when(!parents.is_empty(), |this| {
                                this.child(format!(
                                    "{} {parents}",
                                    if commit.parents.len() > 1 { "merges" } else { "parent" }
                                ))
                            })
                            .child(div().flex_1())
                            .child(
                                Button::new("copy-hash")
                                    .ghost()
                                    .xsmall()
                                    .label("Copy hash")
                                    .tooltip_with_action(
                                        "Copy the full hash",
                                        &CopyHash,
                                        Some("FastGit"),
                                    )
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.copy_hash(window, cx)),
                                    ),
                            ),
                    )
                    .when_some(stats, |this, (files, additions, deletions)| {
                        this.child(
                            h_flex()
                                .gap_3()
                                .text_xs()
                                .child(div().text_color(muted).child(format!(
                                    "{} {}",
                                    thousands(files),
                                    if files == 1 { "file" } else { "files" }
                                )))
                                .child(
                                    div()
                                        .text_color(cx.theme().green)
                                        .child(format!("+{}", thousands(additions))),
                                )
                                .child(
                                    div()
                                        .text_color(cx.theme().red)
                                        .child(format!("−{}", thousands(deletions))),
                                ),
                        )
                    }),
            )
            .child(div().flex_1().min_h_0().child(content))
            .into_any_element()
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let summary = if self.loading {
            "Reading history…".to_string()
        } else if let Some(history) = &self.history {
            let mut summary = format!(
                "{} commits · {} refs · read in {} ms",
                thousands(history.commits.len()),
                thousands(history.refs.len()),
                history.elapsed.as_millis()
            );
            if history.truncated {
                summary.push_str(&format!(" · newest {} shown", thousands(HISTORY_LIMIT)));
            }
            summary
        } else {
            String::new()
        };
        let position = match (&self.history, self.selected) {
            (Some(history), Some(row)) => format!(
                "Commit {} of {}",
                thousands(row + 1),
                thousands(history.commits.len())
            ),
            _ => String::new(),
        };
        let fetch = if self.fetching {
            Some(div().text_color(muted).child("Fetching…"))
        } else {
            self.fetch_error.clone().map(|error| {
                div()
                    .max_w(px(480.))
                    .truncate()
                    .text_color(cx.theme().red)
                    .child(SharedString::from(format!("Fetch failed: {error}")))
            })
        };
        StatusBar::new()
            .left(div().text_xs().text_color(muted).child(summary))
            .right(
                h_flex()
                    .gap_4()
                    .text_xs()
                    .children(fetch)
                    .child(div().text_color(muted).child(position)),
            )
    }
}

impl Render for RepoView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .on_action(cx.listener(|this, _: &OlderCommit, _, cx| this.move_selection(1, cx)))
            .on_action(cx.listener(|this, _: &NewerCommit, _, cx| this.move_selection(-1, cx)))
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_action(cx.listener(|this, _: &Fetch, _, cx| this.fetch(cx)))
            .on_action(cx.listener(|this, _: &CopyHash, window, cx| this.copy_hash(window, cx)))
            .on_action(cx.listener(|this, _: &CloseDiff, _, cx| this.close_change(cx)))
            .child(div().flex_1().min_h_0().child(self.render_body(window, cx)))
            .child(self.render_status_bar(cx))
    }
}

/// Resolves a path so two spellings of one folder compare equal.
fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

fn sidebar_rows(history: &History) -> Vec<SidebarRow> {
    let mut rows = Vec::new();
    for kind in [RefKind::LocalBranch, RefKind::RemoteBranch, RefKind::Tag] {
        let refs: Vec<usize> = (0..history.refs.len())
            .filter(|&ix| history.refs[ix].kind == kind)
            .collect();
        if !refs.is_empty() {
            rows.push(SidebarRow::Header(kind, refs.len()));
            rows.extend(refs.into_iter().map(SidebarRow::Ref));
        }
    }
    rows
}

fn ref_icon(kind: RefKind) -> IconName {
    match kind {
        RefKind::LocalBranch => IconName::GitBranch,
        RefKind::RemoteBranch => IconName::Cloud,
        RefKind::Tag => IconName::Tag,
    }
}

fn section_header(kind: RefKind, count: usize, cx: &App) -> AnyElement {
    let label = match kind {
        RefKind::LocalBranch => "Branches",
        RefKind::RemoteBranch => "Remotes",
        RefKind::Tag => "Tags",
    };
    h_flex()
        .w_full()
        .h(ROW_HEIGHT)
        .px_3()
        .gap_2()
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(label)
        .child(div().flex_1())
        .child(thousands(count))
        .into_any_element()
}

fn ref_row(
    label: &RefLabel,
    ix: usize,
    selected: Option<ObjectId>,
    owner: WeakEntity<RepoView>,
    cx: &App,
) -> AnyElement {
    ListItem::new(SharedString::from(format!("{:?}/{}", label.kind, label.name)))
        .selected(selected == Some(label.target))
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
        .px_3()
        .child(
            h_flex()
                .gap_2()
                .min_w_0()
                .child(
                    Icon::new(ref_icon(label.kind))
                        .small()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .when(label.is_head, |this| this.font_semibold())
                        .child(label.name.clone()),
                ),
        )
        .on_click(move |_, window, cx| {
            let _ = owner.update(cx, |this, cx| this.reveal_ref(ix, window, cx));
        })
        .into_any_element()
}

/// Colors and fonts shared by every visible history row in one frame.
#[derive(Clone)]
struct RowStyle {
    palette: [Hsla; LANE_COLORS],
    muted: Hsla,
    mono: SharedString,
}

impl RowStyle {
    fn new(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            palette: [
                theme.blue,
                theme.magenta,
                theme.green,
                theme.yellow,
                theme.cyan,
                theme.red,
            ],
            muted: theme.muted_foreground,
            mono: theme.mono_font_family.clone(),
        }
    }

    fn lane(&self, lane: u16) -> Hsla {
        self.palette[lane as usize % LANE_COLORS]
    }
}

fn commit_row(
    history: &History,
    row: usize,
    selected: bool,
    graph_width: Pixels,
    style: &RowStyle,
    owner: WeakEntity<RepoView>,
    cx: &App,
) -> AnyElement {
    let commit = &history.commits[row];
    let graph_row = history.graph.rows[row].clone();
    let is_head = history.head_id == Some(commit.id);
    let labels = history
        .refs_by_commit
        .get(&commit.id)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let color = style.lane(graph_row.lane);
    let palette = style.palette;
    ListItem::new(SharedString::from(commit.id.to_string()))
        .selected(selected)
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
        .pl_0()
        .pr_3()
        .child(
            h_flex()
                .h(ROW_HEIGHT)
                .w_full()
                .min_w_0()
                .gap_3()
                .child(
                    div().flex_none().w(graph_width).h(ROW_HEIGHT).child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, (), window, _| {
                                paint_graph_row(&graph_row, is_head, &palette, bounds, window)
                            },
                        )
                        .size_full(),
                    ),
                )
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .gap_1p5()
                        .children(
                            labels
                                .iter()
                                .take(MAX_BADGES)
                                .map(|&ix| ref_badge(&history.refs[ix], color, cx)),
                        )
                        .when(labels.len() > MAX_BADGES, |this| {
                            this.child(
                                div()
                                    .flex_none()
                                    .text_xs()
                                    .text_color(style.muted)
                                    .child(format!("+{}", labels.len() - MAX_BADGES)),
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_sm()
                                .when(commit.subject.is_empty(), |this| this.text_color(style.muted))
                                .child(if commit.subject.is_empty() {
                                    "(no message)".into()
                                } else {
                                    commit.subject.clone()
                                }),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .w(AUTHOR_WIDTH)
                        .truncate()
                        .text_xs()
                        .text_color(style.muted)
                        .child(commit.author.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(DATE_WIDTH)
                        .text_xs()
                        .text_color(style.muted)
                        .child(commit.committed.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .w(ID_WIDTH)
                        .text_xs()
                        .font_family(style.mono.clone())
                        .text_color(style.muted)
                        .child(commit.short_id.clone()),
                ),
        )
        .on_click(move |_, _, cx| {
            let _ = owner.update(cx, |this, cx| this.select(row, cx));
        })
        .into_any_element()
}

/// The row above the graph that stands for the uncommitted changes.
fn wip_row(
    changes: &WorkingChanges,
    selected: bool,
    style: &RowStyle,
    owner: WeakEntity<RepoView>,
    cx: &App,
) -> AnyElement {
    let color = style.muted;
    let mut summary = Vec::new();
    if !changes.unstaged.is_empty() {
        summary.push(format!("{} unstaged", thousands(changes.unstaged.len())));
    }
    if !changes.staged.is_empty() {
        summary.push(format!("{} staged", thousands(changes.staged.len())));
    }
    ListItem::new("wip")
        .selected(selected)
        .flex_none()
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
        .pl_0()
        .pr_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            h_flex()
                .h(ROW_HEIGHT)
                .w_full()
                .min_w_0()
                .gap_3()
                .child(
                    div().flex_none().w(LANE_WIDTH + px(6.)).h(ROW_HEIGHT).child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, (), window, _| {
                                // A hollow node: nothing is committed yet.
                                let center =
                                    point(bounds.left() + LANE_WIDTH * 0.5, bounds.center().y);
                                window.paint_quad(
                                    fill(
                                        Bounds::centered_at(
                                            center,
                                            size(NODE_RADIUS * 2., NODE_RADIUS * 2.),
                                        ),
                                        transparent_black(),
                                    )
                                    .corner_radii(NODE_RADIUS)
                                    .border_widths(px(1.5))
                                    .border_color(color),
                                );
                            },
                        )
                        .size_full(),
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .italic()
                        .child("Uncommitted changes"),
                )
                .child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(style.muted)
                        .child(summary.join(" · ")),
                ),
        )
        .on_click(move |_, _, cx| {
            let _ = owner.update(cx, |this, cx| this.select_changes(cx));
        })
        .into_any_element()
}

fn kind_color(kind: ChangeKind, cx: &App) -> Hsla {
    let theme = cx.theme();
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => theme.green,
        ChangeKind::Deleted => theme.red,
        ChangeKind::Modified => theme.yellow,
        ChangeKind::Renamed | ChangeKind::Copied => theme.blue,
        ChangeKind::Conflicted => theme.magenta,
    }
}

fn kind_badge(kind: ChangeKind, cx: &App) -> AnyElement {
    let color = kind_color(kind, cx);
    div()
        .flex_none()
        .w(px(18.))
        .h(px(18.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(color.opacity(0.18))
        .text_xs()
        .font_semibold()
        .font_family(cx.theme().mono_font_family.clone())
        .text_color(color)
        .child(kind.letter())
        .into_any_element()
}

fn change_header(
    side: Side,
    count: usize,
    staging: bool,
    owner: WeakEntity<RepoView>,
    cx: &App,
) -> AnyElement {
    let (label, button, icon) = match side {
        Side::Unstaged => ("Unstaged files", "Stage all", IconName::Plus),
        Side::Staged => ("Staged files", "Unstage all", IconName::Minus),
    };
    h_flex()
        .w_full()
        .h(ROW_HEIGHT)
        .px_3()
        .gap_2()
        .text_xs()
        .font_medium()
        .text_color(cx.theme().muted_foreground)
        .child(format!("{label} ({})", thousands(count)))
        .child(div().flex_1())
        .child(
            Button::new(SharedString::from(format!("all-{side:?}")))
                .ghost()
                .xsmall()
                .icon(icon)
                .label(button)
                .disabled(staging || count == 0)
                .on_click(move |_, _, cx| {
                    let _ = owner.update(cx, |this, cx| this.stage(side, Vec::new(), cx));
                }),
        )
        .into_any_element()
}

fn change_row(
    change: &FileChange,
    side: Side,
    selected: bool,
    staging: bool,
    owner: WeakEntity<RepoView>,
    cx: &App,
) -> AnyElement {
    let (name, folder) = match change.path.rsplit_once('/') {
        Some((folder, name)) => (name.to_owned(), Some(folder.to_owned())),
        None => (change.path.to_string(), None),
    };
    let (icon, tip) = match side {
        Side::Unstaged => (IconName::Plus, "Stage file"),
        Side::Staged => (IconName::Minus, "Unstage file"),
    };
    let id = format!("{side:?}/{}", change.path);
    let open_owner = owner.clone();
    let open_change = change.clone();
    let stage_change = change.clone();
    ListItem::new(SharedString::from(id.clone()))
        .selected(selected)
        .w_full()
        .h(ROW_HEIGHT)
        .py_0()
        .pl_3()
        .pr_1()
        .child(
            h_flex()
                .w_full()
                .min_w_0()
                .gap_2()
                .child(kind_badge(change.kind, cx))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_1p5()
                        .overflow_hidden()
                        .child(div().flex_none().max_w_full().truncate().text_sm().child(name))
                        .when_some(folder, |this, folder| {
                            this.child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(folder),
                            )
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("stage/{id}")))
                        .ghost()
                        .xsmall()
                        .icon(icon)
                        .tooltip(tip)
                        .disabled(staging)
                        .on_click(move |_, _, cx| {
                            // The row underneath would open the file too.
                            cx.stop_propagation();
                            let change = stage_change.clone();
                            let _ = owner.update(cx, |this, cx| this.stage(side, vec![change], cx));
                        }),
                ),
        )
        .on_click(move |_, _, cx| {
            let change = open_change.clone();
            let _ = open_owner.update(cx, |this, cx| this.open_change(side, change, cx));
        })
        .into_any_element()
}

fn ref_badge(label: &RefLabel, color: Hsla, cx: &App) -> AnyElement {
    // Badges shrink (the flex default) before they ever spill into the author column.
    h_flex()
        .min_w(px(32.))
        .max_w(px(160.))
        .h(px(20.))
        .px_1p5()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(color.opacity(0.6))
        .bg(color.opacity(if label.is_head { 0.35 } else { 0.14 }))
        .text_xs()
        .text_color(cx.theme().foreground)
        .child(Icon::new(ref_icon(label.kind)).xsmall().text_color(color))
        .child(
            div()
                .min_w_0()
                .truncate()
                .when(label.is_head, |this| this.font_semibold())
                .child(label.name.clone()),
        )
        .into_any_element()
}

fn paint_graph_row(
    row: &Row,
    is_head: bool,
    palette: &[Hsla; LANE_COLORS],
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let x = |lane: u16| bounds.left() + LANE_WIDTH * (lane as f32 + 0.5);
    let color = |lane: u16| palette[lane as usize % LANE_COLORS];
    let (top, middle, bottom) = (bounds.top(), bounds.center().y, bounds.bottom());
    let node = row.lane;
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        for edge in &row.edges {
            let mut path = PathBuilder::stroke(px(2.));
            let lane = match *edge {
                Edge::Pass(lane) => {
                    path.move_to(point(x(lane), top));
                    path.line_to(point(x(lane), bottom));
                    lane
                }
                Edge::In { from } => {
                    path.move_to(point(x(from), top));
                    if from == node {
                        path.line_to(point(x(node), middle));
                    } else {
                        // Run down the child's lane, then bend into the commit.
                        let bend = point(x(from), middle);
                        path.cubic_bezier_to(point(x(node), middle), bend, bend);
                    }
                    from
                }
                Edge::Out { to } => {
                    path.move_to(point(x(node), middle));
                    if to == node {
                        path.line_to(point(x(node), bottom));
                    } else {
                        // Leave the commit sideways, then bend down into the parent's lane.
                        let bend = point(x(to), middle);
                        path.cubic_bezier_to(point(x(to), bottom), bend, bend);
                    }
                    to
                }
            };
            if let Ok(path) = path.build() {
                window.paint_path(path, color(lane));
            }
        }

        let center = point(x(node), middle);
        // Merges are smaller dots, so the commits that carry work stand out.
        let radius = if row.merge { NODE_RADIUS * 0.7 } else { NODE_RADIUS };
        window.paint_quad(
            fill(Bounds::centered_at(center, size(radius * 2., radius * 2.)), color(node))
                .corner_radii(radius),
        );
        if is_head {
            let ring = NODE_RADIUS + px(3.);
            window.paint_quad(
                fill(Bounds::centered_at(center, size(ring * 2., ring * 2.)), transparent_black())
                    .corner_radii(ring)
                    .border_widths(px(1.5))
                    .border_color(color(node)),
            );
        }
    });
}

fn loading_state(message: &'static str) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(Spinner::new())
        .child(div().text_sm().child(message))
        .into_any_element()
}

fn centered_message(message: &'static str, cx: &App) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(message)
        .into_any_element()
}

fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (ix, digit) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::thousands;

    #[test]
    fn thousands_are_grouped() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }
}
