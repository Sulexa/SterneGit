use gpui_kit::{
    App, Entity, ParentElement as _, Styled as _, Window,
    component::{
        WindowExt as _,
        button::{Button, ButtonVariants as _},
        dialog::{DialogAction, DialogClose, DialogFooter},
    },
    div,
    prelude::FluentBuilder as _,
};

use super::repository::Repository;
use crate::{
    git::{Behind, Tracking},
    ui::counted,
};

const CANCEL: &str = "Cancel";
const FORCE_PUSH: &str = "Force push";
const PULL_AND_PUSH: &str = "Pull and push";
const PULL: &str = "Pull";

#[derive(Debug, PartialEq, Eq)]
struct BehindQuestion {
    title: String,
    description: String,
    pull: Option<&'static str>,
}

fn behind_question(behind: &Behind) -> BehindQuestion {
    let Behind {
        tracking: Tracking { branch, upstream },
        theirs,
        ours,
        ..
    } = behind;
    let (theirs, ours) = (*theirs, *ours);
    let own = match ours {
        0 => "none of its own to push".to_owned(),
        _ => format!("{ours} that {upstream} doesn't"),
    };
    let counts = format!(
        "{upstream} has {}, and {branch} has {own}.",
        counted(theirs, "new commit", "new commits")
    );
    let pull = match (ours, behind.checked_out) {
        (0, _) => format!("Pull fast-forwards {branch} to {upstream}."),
        (_, true) => format!(
            "Pull and push brings them into {branch}, with a merge commit if {branch} can't fast-forward, then pushes."
        ),
        (_, false) => format!(
            "Check out {branch} to pull them in: a branch that isn't checked out can only fast-forward."
        ),
    };
    let dropped = match theirs {
        1 => "the new commit".to_owned(),
        _ => format!("the {theirs} new commits"),
    };
    let force = format!("Force push replaces {upstream} with {branch}, dropping {dropped} for everyone.");
    BehindQuestion {
        title: format!("{upstream} has commits that {branch} doesn't have"),
        description: format!("{counts} {pull} {force}"),
        pull: behind
            .can_pull()
            .then_some(if ours == 0 { PULL } else { PULL_AND_PUSH }),
    }
}

pub(crate) fn open_behind_dialog(
    repository: Entity<Repository>,
    behind: Behind,
    window: &mut Window,
    cx: &mut App,
) {
    let question = behind_question(&behind);
    window.open_alert_dialog(cx, move |alert, _, _| {
        let forced = (repository.clone(), behind.clone());
        let pulled = (repository.clone(), behind.clone());
        let pull_button = question.pull.map(|label| {
            DialogAction::new().child(Button::new("behind-pull").primary().label(label))
        });
        alert
            .title(question.title.clone())
            .description(question.description.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label(CANCEL)))
                    .child(
                        div().size_full().child(
                            Button::new("behind-force-push")
                                .danger()
                                .label(FORCE_PUSH)
                                .on_click(move |_, window, cx| {
                                    let (repository, behind) = &forced;
                                    repository.update(cx, |repository, cx| {
                                        repository.force_push(behind.clone(), cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                    )
                    .children(pull_button),
            )
            .when(question.pull.is_some(), |alert| {
                alert.on_ok(move |_, _, cx| {
                    let (repository, behind) = &pulled;
                    repository.update(cx, |repository, cx| {
                        repository.pull_then_push(behind.clone(), cx)
                    });
                    true
                })
            })
    });
}

#[cfg(test)]
mod tests {
    use super::{BehindQuestion, PULL, PULL_AND_PUSH, behind_question};
    use crate::git::{Behind, Tracking, fixtures::id};

    fn behind(branch: &str, theirs: usize, ours: usize, checked_out: bool) -> Behind {
        Behind {
            tracking: Tracking {
                branch: branch.into(),
                upstream: format!("origin/{branch}"),
            },
            theirs,
            ours,
            checked_out,
            upstream_at: id(1),
        }
    }

    #[test]
    fn both_sides_with_commits_offer_to_pull_then_push() {
        assert_eq!(
            behind_question(&behind("main", 2, 1, true)),
            BehindQuestion {
                title: "origin/main has commits that main doesn't have".into(),
                description: "origin/main has 2 new commits, and main has 1 that origin/main doesn't. \
                              Pull and push brings them into main, with a merge commit if main can't fast-forward, then pushes. \
                              Force push replaces origin/main with main, dropping the 2 new commits for everyone."
                    .into(),
                pull: Some(PULL_AND_PUSH),
            }
        );
    }

    #[test]
    fn nothing_to_push_offers_a_plain_pull() {
        let question = behind_question(&behind("main", 1, 0, true));
        assert_eq!(
            question.description,
            "origin/main has 1 new commit, and main has none of its own to push. \
             Pull fast-forwards main to origin/main. \
             Force push replaces origin/main with main, dropping the new commit for everyone."
        );
        assert_eq!(question.pull, Some(PULL));
        assert_eq!(
            behind_question(&behind("feature/blame", 3, 0, false)).pull,
            Some(PULL)
        );
    }

    #[test]
    fn a_branch_that_isnt_checked_out_cant_merge_so_it_gets_no_pull() {
        let question = behind_question(&behind("feature/blame", 3, 2, false));
        assert_eq!(
            question.description,
            "origin/feature/blame has 3 new commits, and feature/blame has 2 that origin/feature/blame doesn't. \
             Check out feature/blame to pull them in: a branch that isn't checked out can only fast-forward. \
             Force push replaces origin/feature/blame with feature/blame, dropping the 3 new commits for everyone."
        );
        assert_eq!(question.pull, None);
    }
}
