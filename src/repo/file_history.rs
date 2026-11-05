use std::{ffi::OsString, sync::Arc};

use gix::ObjectId;
use gpui_kit::{
    AnyElement, App, FocusHandle, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, ScrollStrategy, SharedString, Styled as _, Task,
    UniformListScrollHandle, Window,
    component::{
        ActiveTheme as _, Sizable as _, h_flex, scroll::Scrollbar, spinner::Spinner, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};

use super::navigation::Step;
use crate::{
    actions::HISTORY_CONTEXT,
    git::{FileCommit, LineCounts},
    ui::{
        self, Failure, Handler, Load, Look as _, SectionHeader, commit_subject, handler, list_row,
    },
};

const TWO_LINE_ROW: Pixels = px(48.);
const AVATAR_SIZE: Pixels = px(14.);

#[derive(Clone)]
pub(crate) struct HistorySource {
    pub(crate) path: OsString,
    pub(crate) from: Option<ObjectId>,
}

pub(crate) struct FileHistory {
    path: SharedString,
    source: HistorySource,
    commits: Load<Arc<[FileCommit]>>,
    selected: Option<ObjectId>,
    scroll: UniformListScrollHandle,
}

pub(crate) struct HistoryPlace {
    pub(crate) row: usize,
    pub(crate) count: usize,
    pub(crate) name: SharedString,
}

impl FileHistory {
    pub(crate) fn reading(path: SharedString, source: HistorySource, task: Task<()>) -> Self {
        let mut commits = Load::Idle;
        commits.load(task);
        Self {
            path,
            selected: source.from,
            source,
            commits,
            scroll: UniformListScrollHandle::new(),
        }
    }

    pub(crate) fn source(&self) -> &HistorySource {
        &self.source
    }

    pub(crate) fn reread(&mut self, task: Task<()>) {
        self.commits.reload(task);
    }

    pub(crate) fn finish(&mut self, read: anyhow::Result<Vec<FileCommit>>) -> Option<FileCommit> {
        self.commits.finish(read.map(Arc::from));
        let kept = self.selected.and_then(|id| self.row_of(id));
        let row = kept.or_else(|| self.commits().first().map(|_| 0))?;
        self.select(row)
    }

    pub(crate) fn select_id(&mut self, id: ObjectId) -> Option<FileCommit> {
        let row = self.row_of(id)?;
        self.select(row)
    }

    pub(crate) fn step(&mut self, step: Step) -> Option<FileCommit> {
        let last = self.commits().len().checked_sub(1)?;
        let row = self.selected.and_then(|id| self.row_of(id)).unwrap_or(0);
        let row = match step {
            Step::Newer => row.saturating_sub(1),
            Step::Older => (row + 1).min(last),
        };
        self.select(row)
    }

    pub(crate) fn shows(&self, id: ObjectId, path: &str) -> bool {
        if self.selected != Some(id) {
            return false;
        }
        match self.row_of(id) {
            Some(row) => &*self.commits()[row].path == path,
            None => self.commits.value().is_none() && &*self.path == path,
        }
    }

    pub(crate) fn place(&self) -> Option<HistoryPlace> {
        let row = self.row_of(self.selected?)?;
        Some(HistoryPlace {
            row,
            count: self.commits().len(),
            name: file_name(&self.path),
        })
    }

    pub(crate) fn list(&self, focus: FocusHandle) -> FileHistoryList {
        FileHistoryList {
            commits: self.commits.value().cloned(),
            reading: self.commits.is_loading(),
            failure: self.commits.failure().cloned(),
            selected: self.selected.and_then(|id| self.row_of(id)),
            focus,
            scroll: self.scroll.clone(),
            on_select: None,
        }
    }

    fn select(&mut self, row: usize) -> Option<FileCommit> {
        let chosen = self.commits.value()?.get(row)?.clone();
        self.selected = Some(chosen.commit.id);
        self.scroll.scroll_to_item(row, ScrollStrategy::Nearest);
        Some(chosen)
    }

    fn commits(&self) -> &[FileCommit] {
        self.commits.value().map_or(&[], |commits| commits)
    }

    fn row_of(&self, id: ObjectId) -> Option<usize> {
        self.commits()
            .iter()
            .position(|change| change.commit.id == id)
    }
}

fn file_name(path: &str) -> SharedString {
    path.rsplit('/').next().unwrap_or(path).to_owned().into()
}

#[derive(IntoElement)]
pub(crate) struct FileHistoryList {
    commits: Option<Arc<[FileCommit]>>,
    reading: bool,
    failure: Option<Failure>,
    selected: Option<usize>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    on_select: Option<Handler<ObjectId>>,
}

impl FileHistoryList {
    pub(crate) fn on_select(
        mut self,
        callback: impl Fn(&ObjectId, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(handler(callback));
        self
    }
}

impl RenderOnce for FileHistoryList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            commits,
            reading,
            failure,
            selected,
            focus,
            scroll,
            on_select,
        } = self;
        let header = SectionHeader::new("File history")
            .when_some(commits.as_ref(), |header, commits| {
                header.count(commits.len())
            })
            .when(reading, |header| header.action(Spinner::new().xsmall()));
        let body = match (commits, failure) {
            (_, Some(failure)) => {
                ui::failure("Couldn't read this file's history", failure.message(), cx)
            }
            (None, None) => div().into_any_element(),
            (Some(commits), None) if commits.is_empty() => {
                ui::hint("No commit changed this file.", cx)
            }
            (Some(commits), None) => div()
                .id("file-history")
                .relative()
                .size_full()
                .child(
                    uniform_list("file-commits", commits.len(), move |range, _, cx| {
                        range
                            .map(|row| {
                                file_commit_row(
                                    &commits[row],
                                    selected == Some(row),
                                    on_select.clone(),
                                    cx,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .size_full()
                    .track_scroll(&scroll),
                )
                .child(Scrollbar::vertical(&scroll))
                .into_any_element(),
        };
        let focus_on_click = focus.clone();
        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .track_focus(&focus)
            .key_context(HISTORY_CONTEXT)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus_on_click.focus(window, cx)
            })
            .child(header)
            .child(div().fill_height().child(body))
    }
}

fn file_commit_row(
    change: &FileCommit,
    selected: bool,
    on_select: Option<Handler<ObjectId>>,
    cx: &App,
) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    let commit = &change.commit;
    let id = commit.id;
    let detail = match &change.original_path {
        Some(original) => format!("Renamed from {original}"),
        None => format!("{} · {}", commit.author, day(&commit.committed)),
    };
    let counts = match change.lines {
        Some(LineCounts { added, removed }) => h_flex()
            .flex_none()
            .gap_1p5()
            .child(ui::additions(added, cx))
            .child(ui::deletions(removed, cx)),
        None => h_flex().flex_none().text_color(muted).child("binary"),
    };
    list_row(SharedString::from(format!("file-commit/{id}")), selected)
        .h(TWO_LINE_ROW)
        .pl_3()
        .pr_2()
        .child(
            h_flex()
                .w_full()
                .min_w_0()
                .gap_2()
                .child(ui::change_badge(change.kind, cx))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0p5()
                        .child(
                            div()
                                .ellipsis()
                                .sm_text()
                                .when(commit.subject.is_empty(), |this| this.text_color(muted))
                                .child(commit_subject(commit)),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .xs_text()
                                .when(change.original_path.is_none(), |this| {
                                    this.child(ui::author_avatar(
                                        &commit.author,
                                        &commit.author_email,
                                        AVATAR_SIZE,
                                    ))
                                })
                                .child(div().flex_1().ellipsis().text_color(muted).child(detail))
                                .child(counts),
                        ),
                ),
        )
        .when_some(on_select, |row, on_select| {
            row.on_click(move |_, window, cx| on_select(&id, window, cx))
        })
        .into_any_element()
}

fn day(time: &str) -> &str {
    time.split_once(' ').map_or(time, |(day, _)| day)
}

#[cfg(test)]
mod tests {
    use gix::ObjectId;
    use gpui_kit::Task;

    use super::{FileHistory, HistorySource, Step};
    use crate::git::{ChangeKind, FileCommit, fixtures};

    fn changed(byte: u8, path: &str) -> FileCommit {
        FileCommit {
            commit: fixtures::commit(byte, &[], 0),
            kind: ChangeKind::Modified,
            path: path.to_owned().into(),
            original_path: None,
            lines: None,
        }
    }

    fn read(selected: Option<u8>, commits: &[(u8, &str)]) -> FileHistory {
        let source = HistorySource {
            path: "src/b.rs".into(),
            from: selected.map(fixtures::id),
        };
        let mut history = FileHistory::reading("src/b.rs".into(), source, Task::ready(()));
        history.finish(Ok(commits
            .iter()
            .map(|&(byte, path)| changed(byte, path))
            .collect()));
        history
    }

    fn selected(history: &FileHistory) -> Option<ObjectId> {
        history.selected
    }

    #[test]
    fn opened_from_a_commit_the_history_keeps_that_commit_selected() {
        let history = read(Some(2), &[(3, "src/b.rs"), (2, "src/b.rs"), (1, "a.rs")]);
        assert_eq!(selected(&history), Some(fixtures::id(2)));
        assert_eq!(
            history.place().map(|place| (place.row, place.count)),
            Some((1, 3))
        );
        assert_eq!(
            history.place().map(|place| place.name).as_deref(),
            Some("b.rs")
        );
    }

    #[test]
    fn without_a_commit_in_the_list_the_newest_one_is_selected() {
        assert_eq!(
            selected(&read(None, &[(3, "b.rs"), (2, "b.rs")])),
            Some(fixtures::id(3))
        );
        assert_eq!(
            selected(&read(Some(9), &[(3, "b.rs"), (2, "b.rs")])),
            Some(fixtures::id(3))
        );
        let empty = read(None, &[]);
        assert_eq!(selected(&empty), None);
        assert!(empty.place().is_none());
    }

    #[test]
    fn steps_move_through_the_commits_and_stop_at_both_ends() {
        let mut history = read(None, &[(3, "b.rs"), (2, "b.rs"), (1, "a.rs")]);
        let ids = |change: Option<FileCommit>| change.map(|change| change.commit.id.as_bytes()[0]);
        assert_eq!(ids(history.step(Step::Newer)), Some(3));
        assert_eq!(ids(history.step(Step::Older)), Some(2));
        assert_eq!(ids(history.step(Step::Older)), Some(1));
        assert_eq!(ids(history.step(Step::Older)), Some(1));
        assert_eq!(ids(history.step(Step::Newer)), Some(2));
    }

    #[test]
    fn while_reading_the_history_shows_the_commit_it_was_opened_from() {
        let source = HistorySource {
            path: "src/b.rs".into(),
            from: Some(fixtures::id(2)),
        };
        let history = FileHistory::reading("src/b.rs".into(), source, Task::ready(()));
        assert!(history.shows(fixtures::id(2), "src/b.rs"));
        assert!(!history.shows(fixtures::id(2), "src/a.rs"));
        assert!(!history.shows(fixtures::id(3), "src/b.rs"));
    }

    #[test]
    fn the_history_shows_a_commit_only_under_the_name_the_file_had_in_it() {
        let history = read(Some(1), &[(2, "b.rs"), (1, "a.rs")]);
        assert!(history.shows(fixtures::id(1), "a.rs"));
        assert!(!history.shows(fixtures::id(1), "b.rs"));
        assert!(!history.shows(fixtures::id(2), "b.rs"));
    }

    #[test]
    fn a_reread_keeps_the_selected_commit_while_it_is_still_listed() {
        let mut history = read(Some(2), &[(2, "b.rs"), (1, "b.rs")]);
        history.reread(Task::ready(()));
        history.finish(Ok(vec![
            changed(4, "b.rs"),
            changed(2, "b.rs"),
            changed(1, "b.rs"),
        ]));
        assert_eq!(selected(&history), Some(fixtures::id(2)));
        history.reread(Task::ready(()));
        history.finish(Ok(vec![changed(5, "b.rs")]));
        assert_eq!(selected(&history), Some(fixtures::id(5)));
    }
}
