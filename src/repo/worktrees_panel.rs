use std::{collections::BTreeSet, path::Path, sync::Arc};

use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    repository::Repository, worktree_dialog::open_worktree_dialog,
    worktree_menu::merged_into_head,
};
use crate::{
    git::{History, Worktree},
    hosting::{boxed, chip},
    ui::{Handler, Look as _, counted},
};

const WIDTH: f32 = 800.;
const ABOUT: &str = "Every folder that checks out this repository. Removing one deletes its folder; its commits stay on its branch.";

struct Row {
    worktree: Worktree,
    changed: usize,
    merged: bool,
}

impl Row {
    fn can_be_picked(&self) -> bool {
        self.worktree.can_be_removed()
    }
}

struct WorktreesPanel {
    repository: Entity<Repository>,
    picked: BTreeSet<Arc<Path>>,
    into: String,
    on_open: Handler<Arc<Path>>,
    _repository_changes: Subscription,
}

impl WorktreesPanel {
    fn rows(&self, history: &History, cx: &App) -> Vec<Row> {
        let repository = self.repository.read(cx);
        history
            .worktrees
            .iter()
            .map(|worktree| {
                let changes = match worktree.is_here {
                    true => repository.changes(),
                    false => repository.changes_elsewhere(&worktree.location),
                };
                Row {
                    changed: changes.map_or(0, |changes| changes.file_count()),
                    merged: !worktree.is_here && merged_into_head(history, worktree),
                    worktree: worktree.clone(),
                }
            })
            .collect()
    }

    fn toggle(&mut self, location: Arc<Path>, cx: &mut Context<Self>) {
        if !self.picked.remove(&location) {
            self.picked.insert(location);
        }
        cx.notify();
    }

    fn select_merged(&mut self, cx: &mut Context<Self>) {
        let Some(history) = self.repository.read(cx).history().cloned() else {
            return;
        };
        let merged: Vec<Arc<Path>> = self
            .rows(&history, cx)
            .into_iter()
            .filter(|row| row.can_be_picked() && row.merged)
            .map(|row| row.worktree.location)
            .collect();
        self.picked.extend(merged);
        cx.notify();
    }

    fn remove_picked(&mut self, cx: &mut Context<Self>) {
        let Some(history) = self.repository.read(cx).history().cloned() else {
            return;
        };
        let removed: Vec<_> = history
            .worktrees
            .iter()
            .filter(|worktree| worktree.can_be_removed() && self.picked.contains(&worktree.location))
            .map(|worktree| (worktree.location.to_path_buf(), worktree.name.clone()))
            .collect();
        if removed.is_empty() {
            return;
        }
        self.picked.clear();
        self.repository
            .update(cx, |repository, cx| repository.remove_worktrees(removed, None, cx));
        cx.notify();
    }

    fn row(&self, row: &Row, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (muted, blue, red, yellow, magenta) =
            (theme.muted_foreground, theme.blue, theme.red, theme.yellow, theme.magenta);
        let worktree = &row.worktree;
        let merged_into = format!("Merged into {}", self.into);
        let picked = self.picked.contains(&worktree.location);
        let location = worktree.location.clone();
        let chips = [
            worktree.is_main.then(|| chip("Main worktree", None, cx)),
            worktree.is_here.then(|| chip("This tab", Some(blue), cx)),
            worktree.is_missing.then(|| chip("Folder missing", Some(red), cx)),
            (row.changed > 0).then(|| chip(counted(row.changed, "changed file", "changed files"), Some(yellow), cx)),
            row.merged.then(|| chip(merged_into.clone(), Some(magenta), cx)),
            worktree.is_locked().then(|| chip("Locked", None, cx)),
        ];
        let open = (!worktree.is_here && !worktree.is_missing).then(|| {
            let on_open = self.on_open.clone();
            let location = location.clone();
            Button::new(gpui_kit::SharedString::from(format!("open/{}", worktree.location.display())))
                .ghost()
                .xsmall()
                .icon(IconName::ExternalLink)
                .tooltip("Open this worktree in a new tab")
                .on_click(move |_, window, cx| on_open(&location, window, cx))
        });
        h_flex()
            .min_h(px(52.))
            .px_3()
            .gap_2p5()
            .when(picked, |this| this.bg(theme.list_active))
            .child(
                Checkbox::new(gpui_kit::SharedString::from(format!("pick/{}", worktree.location.display())))
                    .checked(picked)
                    .disabled(!row.can_be_picked())
                    .on_click(cx.listener(move |this, _: &bool, _, cx| this.toggle(location.clone(), cx))),
            )
            .child(Icon::new(IconName::FolderGit).text_color(muted))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_flex()
                            .gap_2()
                            .min_w_0()
                            .child(div().flex_none().sm_text().font_medium().child(worktree.name.clone()))
                            .child(div().min_w_0().truncate().xs_text().text_color(muted).child(worktree.checked_out())),
                    )
                    .child(
                        div()
                            .truncate()
                            .xs_text()
                            .text_color(muted)
                            .child(worktree.location.display().to_string()),
                    ),
            )
            .child(h_flex().flex_none().gap_1p5().children(chips.into_iter().flatten()))
            .child(div().flex_none().w(px(24.)).children(open))
    }
}

impl Render for WorktreesPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(history) = self.repository.read(cx).history().cloned() else {
            return div().into_any_element();
        };
        let writing = self.repository.read(cx).is_writing();
        self.into = history.branch.as_deref().unwrap_or("HEAD").to_owned();
        let rows = self.rows(&history, cx);
        let live: BTreeSet<Arc<Path>> = rows
            .iter()
            .filter(|row| row.can_be_picked())
            .map(|row| row.worktree.location.clone())
            .collect();
        self.picked.retain(|location| live.contains(location));
        let losing = rows
            .iter()
            .filter(|row| self.picked.contains(&row.worktree.location) && row.changed > 0)
            .count();
        let picked = self.picked.len();
        let any_missing = rows.iter().any(|row| row.worktree.is_missing);
        let theme = cx.theme();
        let (muted, yellow) = (theme.muted_foreground, theme.yellow);
        let list: Vec<_> = rows
            .iter()
            .map(|row| self.row(row, cx).into_any_element())
            .collect();
        let repository = self.repository.clone();
        v_flex()
            .gap_2()
            .child(div().sm_text().text_color(muted).child(ABOUT))
            .child(
                h_flex()
                    .mt_2()
                    .gap_1()
                    .child(div().flex_1().xs_text().text_color(muted).child(counted(rows.len(), "worktree", "worktrees")))
                    .child(
                        Button::new("select-merged")
                            .ghost()
                            .small()
                            .label("Select merged")
                            .on_click(cx.listener(|this, _, _, cx| this.select_merged(cx))),
                    )
                    .child(
                        Button::new("forget-missing")
                            .ghost()
                            .small()
                            .label("Forget missing folders")
                            .disabled(!any_missing || writing)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.repository
                                    .update(cx, |repository, cx| repository.forget_missing_worktrees(cx))
                            })),
                    ),
            )
            .child(boxed(list, cx))
            .when(losing > 0, |this| {
                this.child(
                    h_flex()
                        .gap_1p5()
                        .xs_text()
                        .text_color(yellow)
                        .child(Icon::new(IconName::TriangleAlert).xsmall())
                        .child(format!(
                            "{} uncommitted changes, which are lost.",
                            counted(losing, "worktree has", "worktrees have")
                        )),
                )
            })
            .child(
                h_flex()
                    .mt_2()
                    .gap_2()
                    .child(
                        Button::new("panel-new-worktree")
                            .icon(IconName::FolderPlus)
                            .label("New worktree…")
                            .on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                                open_worktree_dialog(repository.clone(), None, window, cx);
                            }),
                    )
                    .child(div().flex_1())
                    .child(Button::new("panel-close").label("Close").on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("panel-remove")
                            .danger()
                            .icon(IconName::Trash)
                            .label(match picked {
                                0 => "Remove".to_owned(),
                                _ => format!("Remove {}", counted(picked, "worktree", "worktrees")),
                            })
                            .disabled(picked == 0 || writing)
                            .on_click(cx.listener(|this, _, _, cx| this.remove_picked(cx))),
                    ),
            )
            .into_any_element()
    }
}

pub(crate) fn open_worktrees_panel(
    repository: Entity<Repository>,
    on_open: Handler<Arc<Path>>,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(name) = repository
        .read(cx)
        .history()
        .and_then(|history| history.main_worktree().map(|main| main.name.clone()))
        .or_else(|| Some(repository.read(cx).name()))
    else {
        return;
    };
    let panel = cx.new(|cx| WorktreesPanel {
        _repository_changes: cx.observe(&repository, |_, _, cx| cx.notify()),
        repository,
        picked: BTreeSet::new(),
        into: String::new(),
        on_open,
    });
    let title = format!("Worktrees of {name}");
    window.open_dialog(cx, move |dialog, _, _| {
        dialog.title(title.clone()).w(px(WIDTH)).child(panel.clone())
    });
}
