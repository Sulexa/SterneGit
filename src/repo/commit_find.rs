use std::rc::Rc;

use gix::ObjectId;
use gpui_kit::{
    App, AppContext as _, Context, Entity, ScrollStrategy, SharedString, Subscription, Window,
};

use super::{
    RepoView,
    commit_search::{CommitFinder, Descriptions, find_commits},
    history_list::Finding,
    navigation::Target,
};
use crate::{
    git::HISTORY_LIMIT,
    ui::{FindBar, FindEvent, Found, Job, ROW_HEIGHT, run_in_background, thousands},
};

const FIND_COMMITS: &str = "Find commits by message, author, id, branch or tag";

#[derive(Default)]
pub(super) struct CommitFind {
    bar: Option<Entity<FindBar>>,
    open: bool,
    finding: Option<Finding>,
    descriptions: Descriptions,
    reading: Job,
    events: Option<Subscription>,
}

impl CommitFind {
    pub(super) fn bar(&self) -> Option<Entity<FindBar>> {
        self.bar.clone().filter(|_| self.open)
    }

    pub(super) fn finding(&self) -> Option<Finding> {
        self.finding.clone().filter(|_| self.open)
    }
}

impl RepoView {
    pub(crate) fn find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.target.has_open_file() {
            self.file_diff.update(cx, |panel, cx| panel.open_find(window, cx));
        } else if self.shows_graph() {
            self.open_commit_find(window, cx);
        }
    }

    pub(crate) fn find_again(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.target.has_open_file() {
            self.file_diff.update(cx, |panel, cx| panel.step_found(forward, window, cx));
            return;
        }
        if !self.shows_graph() {
            return;
        }
        if self.commit_find.open {
            self.step_commit_found(forward, cx);
        } else {
            self.open_commit_find(window, cx);
        }
    }

    pub(super) fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.target.has_open_file() {
            let finding = self.file_diff.read(cx).is_finding();
            if finding {
                self.file_diff.update(cx, |panel, cx| panel.close_find(window, cx));
            }
            return finding;
        }
        let open = self.commit_find.open;
        if open {
            self.close_commit_find(window, cx);
        }
        open
    }

    pub(super) fn is_finding(&self, cx: &App) -> bool {
        if self.target.has_open_file() {
            self.file_diff.read(cx).is_finding()
        } else {
            self.commit_find.open && self.shows_graph()
        }
    }

    pub(super) fn find_commits_again(&mut self, cx: &mut Context<Self>) {
        if self.commit_find.open {
            self.search_commits(cx);
            self.read_descriptions(cx);
        }
    }

    fn shows_graph(&self) -> bool {
        !matches!((&self.target, &self.pull_pane), (Target::Pull { .. }, Some(_)))
    }

    fn open_commit_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let bar = match &self.commit_find.bar {
            Some(bar) => bar.clone(),
            None => {
                let view = cx.entity().downgrade();
                let bar = cx.new(|cx| {
                    FindBar::new(
                        FIND_COMMITS,
                        move |cx| {
                            view.upgrade()
                                .map_or_else(Found::default, |view| view.read(cx).commit_found(cx))
                        },
                        window,
                        cx,
                    )
                });
                self.commit_find.events = Some(cx.subscribe_in(&bar, window, |this, _, event, window, cx| {
                    match event {
                        FindEvent::Searched => {
                            this.search_commits(cx);
                            this.scroll_to_first_commit_found(cx);
                        }
                        FindEvent::Next => this.step_commit_found(true, cx),
                        FindEvent::Previous => this.step_commit_found(false, cx),
                        FindEvent::Closed => this.close_commit_find(window, cx),
                    }
                }));
                self.commit_find.bar = Some(bar.clone());
                bar
            }
        };
        let reopened = !self.commit_find.open;
        self.commit_find.open = true;
        bar.update(cx, |bar, cx| bar.focus(window, cx));
        if reopened {
            self.search_commits(cx);
        }
        self.read_descriptions(cx);
        cx.notify();
    }

    fn close_commit_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_find.open = false;
        self.commit_find.finding = None;
        self.middle_focus.focus(window, cx);
        cx.notify();
    }

    fn search_commits(&mut self, cx: &mut Context<Self>) {
        let Some(bar) = self.commit_find.bar.clone().filter(|_| self.commit_find.open) else {
            return;
        };
        let Some(history) = self.repository.read(cx).history().cloned() else {
            return;
        };
        let note = history
            .truncated
            .then(|| SharedString::from(format!("in the newest {} commits", thousands(HISTORY_LIMIT))));
        bar.update(cx, |bar, cx| bar.set_note(note, cx));
        let search = bar.read(cx).search(cx);
        self.commit_find.finding = CommitFinder::new(&search).map(|finder| {
            let found = find_commits(&history, &finder, &self.commit_find.descriptions);
            Finding {
                finder: Rc::new(finder),
                found: Rc::new(found),
            }
        });
        cx.notify();
    }

    fn selected_row(&self, cx: &App) -> Option<usize> {
        let id = self.target.commit()?;
        self.repository.read(cx).history()?.row_of(&id)
    }

    fn commit_found(&self, cx: &App) -> Found {
        let Some(finding) = &self.commit_find.finding else {
            return Found::default();
        };
        Found {
            count: finding.found.rows.len(),
            current: self.selected_row(cx).and_then(|row| finding.found.index_of(row)),
        }
    }

    fn reading_row(&self, cx: &App) -> usize {
        self.selected_row(cx).unwrap_or_else(|| {
            let scrolled = -self.history_scroll.0.borrow().base_handle.offset().y;
            (scrolled / ROW_HEIGHT).max(0.) as usize
        })
    }

    fn scroll_to_first_commit_found(&mut self, cx: &mut Context<Self>) {
        let Some(finding) = self.commit_find.finding.clone() else {
            return;
        };
        if let Some(at) = finding.found.first_from(self.reading_row(cx)) {
            self.history_scroll
                .scroll_to_item(finding.found.rows[at], ScrollStrategy::Nearest);
            cx.notify();
        }
    }

    fn step_commit_found(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(finding) = self.commit_find.finding.clone() else {
            return;
        };
        if let Some(at) = finding.found.step_from(self.reading_row(cx), forward) {
            self.show_found_commit(finding.found.rows[at], cx);
        }
    }

    fn show_found_commit(&mut self, row: usize, cx: &mut Context<Self>) {
        let id = self
            .repository
            .read(cx)
            .history()
            .and_then(|history| history.commits.get(row))
            .map(|commit| commit.id);
        if let Some(id) = id {
            self.show_commit(id, cx);
        }
    }

    fn read_descriptions(&mut self, cx: &mut Context<Self>) {
        if self.commit_find.reading.is_running() {
            return;
        }
        let repository = self.repository.read(cx);
        let (Some(repo), Some(history)) = (repository.repo().cloned(), repository.history().cloned())
        else {
            return;
        };
        let known = &self.commit_find.descriptions;
        let unread: Vec<ObjectId> = history
            .commits
            .iter()
            .enumerate()
            .filter(|(row, commit)| history.worktree_on_row(*row).is_none() && !known.contains_key(&commit.id))
            .map(|(_, commit)| commit.id)
            .collect();
        if unread.is_empty() {
            return;
        }
        self.commit_find.reading.run(run_in_background(
            cx,
            move || repo.descriptions(&unread),
            |this, read, cx| {
                this.commit_find.reading.finish();
                this.commit_find.descriptions.extend(read);
                this.search_commits(cx);
            },
        ));
    }
}
