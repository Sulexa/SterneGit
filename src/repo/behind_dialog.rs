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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Primary {
    Pull(&'static str),
    ForcePush,
}

#[derive(Debug, PartialEq, Eq)]
struct BehindQuestion {
    title: String,
    description: String,
    primary: Option<Primary>,
}

fn behind_question(behind: &Behind) -> BehindQuestion {
    if behind.rewritten {
        return rewritten_question(behind);
    }
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
        primary: behind
            .can_pull()
            .then_some(Primary::Pull(if ours == 0 { PULL } else { PULL_AND_PUSH })),
    }
}

fn rewritten_question(behind: &Behind) -> BehindQuestion {
    let Tracking { branch, upstream } = &behind.tracking;
    let (versions, them) = match behind.theirs {
        1 => (
            format!(
                "The commit {upstream} has and {branch} doesn't makes the same changes as one of {branch}'s: it was rewritten after it was pushed, with a new message or a rebase."
            ),
            "it",
        ),
        theirs => (
            format!(
                "The {theirs} commits {upstream} has and {branch} doesn't make the same changes as commits of {branch}: they were rewritten after they were pushed, with a new message or a rebase."
            ),
            "them",
        ),
    };
    BehindQuestion {
        title: format!("{upstream} has older versions of {branch}'s commits"),
        description: format!(
            "{versions} Force push replaces {them} on {upstream} with {branch}'s versions, so no change is lost. Pulling would bring the old versions back beside the new ones."
        ),
        primary: Some(Primary::ForcePush),
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
        let chosen = (repository.clone(), behind.clone());
        let primary = question.primary;
        let primary_button = primary.map(|primary| {
            let label = match primary {
                Primary::Pull(label) => label,
                Primary::ForcePush => FORCE_PUSH,
            };
            DialogAction::new().child(Button::new("behind-primary").primary().label(label))
        });
        let force_button = (primary != Some(Primary::ForcePush)).then(|| {
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
            )
        });
        alert
            .title(question.title.clone())
            .description(question.description.clone())
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label(CANCEL)))
                    .children(force_button)
                    .children(primary_button),
            )
            .when_some(primary, |alert, primary| {
                alert.on_ok(move |_, _, cx| {
                    let (repository, behind) = &chosen;
                    repository.update(cx, |repository, cx| match primary {
                        Primary::Pull(_) => repository.pull_then_push(behind.clone(), cx),
                        Primary::ForcePush => repository.force_push(behind.clone(), cx),
                    });
                    true
                })
            })
    });
}

#[cfg(test)]
mod tests {
    use super::{BehindQuestion, PULL, PULL_AND_PUSH, Primary, behind_question};
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
            rewritten: false,
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
                primary: Some(Primary::Pull(PULL_AND_PUSH)),
            }
        );
    }

    #[test]
    fn commits_rewritten_since_they_were_pushed_offer_only_force_push() {
        let rewritten = |theirs| Behind {
            rewritten: true,
            ..behind("main", theirs, theirs, true)
        };
        assert_eq!(
            behind_question(&rewritten(2)),
            BehindQuestion {
                title: "origin/main has older versions of main's commits".into(),
                description: "The 2 commits origin/main has and main doesn't make the same changes as commits of main: \
                              they were rewritten after they were pushed, with a new message or a rebase. \
                              Force push replaces them on origin/main with main's versions, so no change is lost. \
                              Pulling would bring the old versions back beside the new ones."
                    .into(),
                primary: Some(Primary::ForcePush),
            }
        );
        assert_eq!(
            behind_question(&rewritten(1)).description,
            "The commit origin/main has and main doesn't makes the same changes as one of main's: \
             it was rewritten after it was pushed, with a new message or a rebase. \
             Force push replaces it on origin/main with main's versions, so no change is lost. \
             Pulling would bring the old versions back beside the new ones."
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
        assert_eq!(question.primary, Some(Primary::Pull(PULL)));
        assert_eq!(
            behind_question(&behind("feature/blame", 3, 0, false)).primary,
            Some(Primary::Pull(PULL))
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
        assert_eq!(question.primary, None);
    }
}
