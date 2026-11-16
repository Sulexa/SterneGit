use std::collections::HashMap;

use gpui_kit::{
    App, Entity, ParentElement as _, Styled as _, Window,
    component::{
        WindowExt as _,
        button::{Button, ButtonVariant, ButtonVariants as _},
        dialog::{DialogAction, DialogClose, DialogFooter},
    },
    div,
};

use super::{
    changes_list::{DiscardQuestion, discard_question, last_commit_question},
    repository::Repository,
};
use crate::{
    git::{ChangeKind, FileChange, WorkingChanges},
    ui::counted,
};

const STASH_INSTEAD: &str = "Stash instead";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Discarded {
    Unstaged,
    ToLastCommit,
}

pub(crate) fn confirm_discard(
    repository: Entity<Repository>,
    changes: Vec<FileChange>,
    discarded: Discarded,
    window: &mut Window,
    cx: &mut App,
) {
    let question = match discarded {
        Discarded::Unstaged => discard_question(&changes),
        Discarded::ToLastCommit => last_commit_question(&changes),
    };
    window.open_alert_dialog(cx, move |alert, _, _| {
        let repository = repository.clone();
        let changes = changes.clone();
        alert
            .confirm()
            .title(question.title.clone())
            .description(question.description.clone())
            .ok_text(question.confirm)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, _, cx| {
                let changes = changes.clone();
                repository.update(cx, |repository, cx| match discarded {
                    Discarded::Unstaged => repository.discard(changes, cx),
                    Discarded::ToLastCommit => repository.discard_to_last_commit(changes, cx),
                });
                true
            })
    });
}

fn every_change(changes: &WorkingChanges) -> impl Iterator<Item = &FileChange> {
    changes
        .conflicted
        .iter()
        .chain(&changes.staged)
        .chain(&changes.unstaged)
}

fn everything_question(changes: &WorkingChanges) -> DiscardQuestion {
    let mut new_by_path = HashMap::<&str, bool>::new();
    for change in every_change(changes) {
        let new = matches!(change.kind, ChangeKind::Added | ChangeKind::Untracked);
        *new_by_path.entry(&change.path).or_default() |= new;
    }
    let total = new_by_path.len();
    let created = new_by_path.values().filter(|new| **new).count();
    let back = total - created;
    let what = match (back, created) {
        (_, 0) => format!("{}, staged changes included.", gone_back(back)),
        (0, _) => format!(
            "{} deleted, staged ones included.",
            counted(created, "new file is", "new files are")
        ),
        _ => format!(
            "{}, staged changes included, and {} deleted.",
            gone_back(back),
            counted(created, "new file is", "new files are"),
        ),
    };
    let title = match total {
        1 => "Discard the one changed file?".to_owned(),
        _ => format!("Discard all {total} changed files?"),
    };
    DiscardQuestion {
        title,
        description: format!(
            "{what} Git keeps no copy of them, so this can't be undone. Stash instead puts everything aside, where Pop brings it back."
        ),
        confirm: "Discard",
    }
}

fn gone_back(files: usize) -> String {
    counted(
        files,
        "file goes back to how it was in the last commit",
        "files go back to how they were in the last commit",
    )
}

pub(crate) fn confirm_discard_all(
    repository: Entity<Repository>,
    changes: &WorkingChanges,
    window: &mut Window,
    cx: &mut App,
) {
    let question = everything_question(changes);
    let everything: Vec<FileChange> = every_change(changes).cloned().collect();
    window.open_alert_dialog(cx, move |alert, _, _| {
        let stashed = repository.clone();
        let discarded = (repository.clone(), everything.clone());
        alert
            .title(question.title.clone())
            .description(question.description.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(
                        div().size_full().child(
                            Button::new("discard-all-stash")
                                .label(STASH_INSTEAD)
                                .on_click(move |_, window, cx| {
                                    stashed.update(cx, |repository, cx| repository.stash(cx));
                                    window.close_dialog(cx);
                                }),
                        ),
                    )
                    .child(
                        DialogAction::new()
                            .child(Button::new("discard-all").danger().label(question.confirm)),
                    ),
            )
            .on_ok(move |_, _, cx| {
                let (repository, everything) = &discarded;
                let everything = everything.clone();
                repository.update(cx, |repository, cx| {
                    repository.discard_to_last_commit(everything, cx)
                });
                true
            })
    });
}

#[cfg(test)]
mod tests {
    use super::everything_question;
    use crate::git::{ChangeKind, FileChange, WorkingChanges};

    fn change(kind: ChangeKind, path: &str) -> FileChange {
        FileChange::new(path.as_bytes(), None, kind)
    }

    fn changes(staged: Vec<FileChange>, unstaged: Vec<FileChange>) -> WorkingChanges {
        WorkingChanges {
            staged,
            unstaged,
            ..WorkingChanges::default()
        }
    }

    #[test]
    fn discarding_everything_counts_files_that_go_back_and_new_files_deleted() {
        let question = everything_question(&changes(
            vec![change(ChangeKind::Added, "src/ui/toast.rs"), change(ChangeKind::Modified, "src/git/patch.rs")],
            vec![
                change(ChangeKind::Modified, "src/git/patch.rs"),
                change(ChangeKind::Modified, "src/git/status.rs"),
                change(ChangeKind::Untracked, "docs/notes.md"),
            ],
        ));
        assert_eq!(question.title, "Discard all 4 changed files?");
        assert_eq!(
            question.description,
            "2 files go back to how they were in the last commit, staged changes included, and 2 new files are deleted. \
             Git keeps no copy of them, so this can't be undone. Stash instead puts everything aside, where Pop brings it back."
        );
    }

    #[test]
    fn a_staged_new_file_changed_again_is_still_new() {
        let question = everything_question(&changes(
            vec![change(ChangeKind::Added, "a.txt")],
            vec![change(ChangeKind::Modified, "a.txt")],
        ));
        assert_eq!(question.title, "Discard the one changed file?");
        assert!(question.description.starts_with("1 new file is deleted, staged ones included."));
    }

    #[test]
    fn only_tracked_changes_say_nothing_is_deleted() {
        let question = everything_question(&changes(vec![], vec![change(ChangeKind::Modified, "a.txt")]));
        assert!(question.description.starts_with("1 file goes back to how it was in the last commit, staged changes included."));
    }
}
