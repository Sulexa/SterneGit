use std::{path::Path, sync::Arc};

use gix::{ObjectId, hash::Kind};
use gpui_kit::{
    AnyElement, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, UniformListScrollHandle, Window,
    component::{ActiveTheme as _, StyledExt as _, diff::DiffFile, h_flex, v_flex},
    div,
    prelude::FluentBuilder as _,
    px, uniform_list,
};

use super::{
    commit_files::CommitFiles,
    commit_panel::OpenCommitFile,
    patch_view::{PatchStats, PatchTexts, parse_patch, patch_content},
};
use crate::{
    git::{Commit, Line, Repo, short_id},
    ui::{
        self, FileView, FileViewToggle, Folded, Load, Look as _, commit_subject, counted, listed,
        run_in_background, thousands,
    },
};

const AVATAR_SIZE: Pixels = px(16.);
const COMMIT_ROW_HEIGHT: f32 = 24.;
const COMMIT_ROWS_SHOWN: usize = 7;
const APART: &str = "These commits don't follow one another on a single line, so their changes can't be shown as one. Ctrl+click the commits of one branch, or Shift+click a run of them, to see their changes together.";

const TEXTS: PatchTexts = PatchTexts {
    loading: "Reading their changes…",
    failed: "Couldn't read their changes",
    empty: "Together these commits don't change any files.",
};

pub(crate) struct SelectPickedCommit(pub(crate) ObjectId);

struct Together {
    files: Arc<[DiffFile]>,
    stats: PatchStats,
}

pub(crate) struct PickedPanel {
    commits: Arc<[Commit]>,
    line: Option<Line>,
    root: Option<Arc<Path>>,
    together: Load<Together>,
    open: Option<SharedString>,
    folded: Folded<SharedString>,
    files_scroll: UniformListScrollHandle,
    commits_scroll: UniformListScrollHandle,
}

impl EventEmitter<OpenCommitFile> for PickedPanel {}

impl EventEmitter<SelectPickedCommit> for PickedPanel {}

impl Default for PickedPanel {
    fn default() -> Self {
        Self {
            commits: Arc::from([]),
            line: None,
            root: None,
            together: Load::Idle,
            open: None,
            folded: Folded::default(),
            files_scroll: UniformListScrollHandle::new(),
            commits_scroll: UniformListScrollHandle::new(),
        }
    }
}

impl PickedPanel {
    pub(crate) fn show(&mut self, repo: &Repo, commits: Vec<Commit>, line: Option<Line>, cx: &mut Context<Self>) {
        let same = self.line == line
            && self.commits.len() == commits.len()
            && self.commits.iter().zip(&commits).all(|(shown, commit)| shown.id == commit.id);
        self.commits = commits.into();
        if same && self.together.failure().is_none() {
            cx.notify();
            return;
        }
        if !same {
            self.open = None;
            self.folded = Folded::default();
            self.files_scroll = UniformListScrollHandle::new();
            self.commits_scroll = UniformListScrollHandle::new();
        }
        self.line = line;
        self.root = Some(Arc::from(repo.location()));
        let Some(line) = line else {
            self.together.clear();
            cx.notify();
            return;
        };
        let repo = repo.clone();
        let base = line.base.unwrap_or_else(|| ObjectId::empty_tree(Kind::Sha1));
        let task = run_in_background(
            cx,
            move || parse_patch(&repo.range_patch(base, line.newest)?),
            |this, result: anyhow::Result<Vec<DiffFile>>, cx| {
                this.together.finish(result.map(|files| Together {
                    stats: PatchStats::of(&files),
                    files: files.into(),
                }));
                cx.notify();
            },
        );
        self.together.load(task);
        cx.notify();
    }

    pub(crate) fn files(&self) -> Option<Arc<[DiffFile]>> {
        self.together.value().map(|together| together.files.clone())
    }

    pub(crate) fn compared(&self) -> Option<(ObjectId, ObjectId)> {
        let line = self.line?;
        Some((line.newest, line.base.unwrap_or_else(|| ObjectId::empty_tree(Kind::Sha1))))
    }

    pub(crate) fn count(&self) -> usize {
        self.commits.len()
    }

    pub(crate) fn mark_open(&mut self, path: Option<SharedString>, cx: &mut Context<Self>) {
        if self.open != path {
            self.open = path;
            cx.notify();
        }
    }

    fn commit_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let commits = self.commits.clone();
        let shown = commits.len().min(COMMIT_ROWS_SHOWN);
        let hover = cx.theme().list_hover;
        let (muted, mono) = (cx.theme().muted_foreground, cx.theme().mono_font_family.clone());
        let on_select = cx.listener(|_, id: &ObjectId, _, cx| cx.emit(SelectPickedCommit(*id)));
        let on_select = Arc::new(on_select);
        uniform_list("picked-commits", commits.len(), move |range, _, _| {
            range
                .map(|ix| {
                    let commit = &commits[ix];
                    let id = commit.id;
                    let on_select = on_select.clone();
                    h_flex()
                        .id(ix)
                        .w_full()
                        .h(px(COMMIT_ROW_HEIGHT))
                        .px_2()
                        .gap_2()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .child(ui::author_avatar(&commit.author, &commit.author_email, AVATAR_SIZE))
                        .child(div().flex_1().min_w_0().ellipsis().sm_text().child(commit_subject(commit)))
                        .child(
                            div()
                                .flex_none()
                                .xs_text()
                                .font_family(mono.clone())
                                .text_color(muted)
                                .child(commit.short_id.clone()),
                        )
                        .on_click(move |_, window, cx| on_select(&id, window, cx))
                })
                .collect::<Vec<_>>()
        })
        .track_scroll(&self.commits_scroll)
        .mx(px(-8.))
        .h(px(COMMIT_ROW_HEIGHT * shown as f32))
        .into_any_element()
    }
}

fn authors_and_days(commits: &[Commit]) -> (String, String) {
    let mut authors: Vec<&str> = Vec::new();
    for commit in commits {
        if !authors.contains(&commit.author.as_ref()) {
            authors.push(&commit.author);
        }
    }
    let mut days: Vec<&str> = commits.iter().map(|commit| day_of(&commit.authored)).collect();
    days.sort_unstable();
    let span = match (days.first(), days.last()) {
        (Some(first), Some(last)) if first != last => format!("{first} – {last}"),
        (Some(day), _) => (*day).to_owned(),
        _ => String::new(),
    };
    (listed(&authors), span)
}

fn day_of(date: &str) -> &str {
    date.split_once(' ').map_or(date, |(day, _)| day)
}

fn picked_title(count: usize) -> String {
    format!("{} commits picked", thousands(count))
}

fn together_line(base: Option<ObjectId>, newest: ObjectId) -> String {
    match base {
        Some(base) => format!("Their changes together, from {} to {}", short_id(base), short_id(newest)),
        None => format!("Their changes together, from nothing to {}", short_id(newest)),
    }
}

impl Render for PickedPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let (authors, span) = authors_and_days(&self.commits);
        let list = self.commit_list(cx);
        let stats = self
            .together
            .value()
            .map(|together| together.stats)
            .filter(|stats| stats.files > 0);
        let together = self.line.map(|line| {
            div()
                .xs_text()
                .text_color(muted)
                .child(together_line(line.base, line.newest))
        });
        let on_open =
            cx.listener(|_, file: &DiffFile, _, cx| cx.emit(OpenCommitFile(file.clone())));
        let on_fold = cx.listener(|this, folder: &SharedString, _, cx| {
            this.folded.toggle(folder.clone());
            cx.notify();
        });
        let view = FileView::current(cx);
        let content = match self.line {
            None => ui::hint(APART, cx),
            Some(_) => patch_content(
                &self.together,
                |together| together.stats,
                &TEXTS,
                |together| {
                    CommitFiles::new(
                        together.files.clone(),
                        view,
                        &self.folded,
                        self.files_scroll.clone(),
                    )
                    .open(self.open.clone())
                    .root(self.root.clone())
                    .on_open(on_open)
                    .on_fold(on_fold)
                    .into_any_element()
                },
                cx,
            ),
        };
        v_flex()
            .pane()
            .child(
                v_flex()
                    .flex_none()
                    .px_4()
                    .py_3()
                    .gap_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().base_text().font_medium().child(picked_title(self.commits.len())))
                    .child(
                        h_flex()
                            .gap_2()
                            .xs_text()
                            .text_color(muted)
                            .child(div().min_w_0().ellipsis().child(authors))
                            .child("·")
                            .child(div().flex_none().child(span)),
                    )
                    .child(list)
                    .children(together)
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
                    }),
            )
            .child(div().fill_height().child(content))
    }
}

#[cfg(test)]
mod tests {
    use super::{authors_and_days, day_of, picked_title, together_line};
    use crate::git::{
        Commit,
        fixtures::{commit, id},
        short_id,
    };

    fn by(author: &str, authored: &str) -> Commit {
        let mut made = commit(1, &[], 0);
        made.author = author.to_owned().into();
        made.authored = authored.to_owned().into();
        made
    }

    #[test]
    fn the_header_names_each_author_once_and_the_days_they_span() {
        let commits = [
            by("Sam Lee", "2026-11-03 18:42"),
            by("Alex Martin", "2026-11-04 09:12"),
            by("Sam Lee", "2026-11-01 22:36"),
        ];
        assert_eq!(
            authors_and_days(&commits),
            ("Sam Lee and Alex Martin".to_owned(), "2026-11-01 – 2026-11-04".to_owned())
        );
        let same_day = [by("Sam Lee", "2026-11-03 18:42"), by("Sam Lee", "2026-11-03 09:00")];
        assert_eq!(authors_and_days(&same_day), ("Sam Lee".to_owned(), "2026-11-03".to_owned()));
        assert_eq!(day_of("yesterday"), "yesterday");
    }

    #[test]
    fn the_title_counts_and_the_together_line_names_both_ends() {
        assert_eq!(picked_title(1_204), "1,204 commits picked");
        assert_eq!(
            together_line(Some(id(1)), id(2)),
            format!("Their changes together, from {} to {}", short_id(id(1)), short_id(id(2)))
        );
        assert_eq!(
            together_line(None, id(2)),
            format!("Their changes together, from nothing to {}", short_id(id(2)))
        );
    }
}
