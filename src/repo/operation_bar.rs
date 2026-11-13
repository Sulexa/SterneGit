use gpui_kit::{
    App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::operation::Finish;
use crate::{
    git::{Picking, Rebasing, Underway},
    ui::{Handler, Look as _, counted, handler},
};

const HEIGHT: f32 = 40.;
const TINT: f32 = 0.08;

pub(crate) struct Wording {
    pub(crate) title: SharedString,
    pub(crate) step: Option<Step>,
    pub(crate) abort: &'static str,
    pub(crate) abort_tooltip: &'static str,
    pub(crate) finish: &'static str,
    pub(crate) finish_tooltip: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) position: Option<SharedString>,
    pub(crate) id: SharedString,
    pub(crate) subject: SharedString,
}

pub(crate) fn wording(underway: &Underway) -> Wording {
    match underway {
        Underway::Rebase(rebasing) => Wording {
            title: format!("Rebasing {} onto {}", rebasing.branch, rebasing.onto).into(),
            step: step(rebasing),
            abort: "Abort rebase",
            abort_tooltip: "Put the branch back where it was before the rebase",
            finish: "Continue",
            finish_tooltip: "Commit the resolved changes and replay the next commits",
        },
        Underway::Merge(merging) => Wording {
            title: format!("Merging {} into {}", merging.from, merging.into).into(),
            step: None,
            abort: "Abort merge",
            abort_tooltip: "Put the branch back where it was before the merge",
            finish: "Commit merge",
            finish_tooltip: "Commit the merge with its usual message",
        },
        Underway::Squash(merging) => Wording {
            title: match merging.from.is_empty() {
                true => format!("Squashing into {}", merging.into),
                false => format!("Squashing {} into {}", merging.from, merging.into),
            }
            .into(),
            step: None,
            abort: "Abort squash",
            abort_tooltip: "Put the branch back where it was before the squash",
            finish: "Commit squash",
            finish_tooltip: "Commit the squash with the subjects of the branch's commits as its message",
        },
        Underway::Replay(replay) => Wording {
            title: format!("Rebasing {} onto {}", replay.from, replay.picking.branch).into(),
            step: Some(picked(&replay.picking)),
            abort: "Abort rebase",
            abort_tooltip: "Put the branch back where it was before the rebase",
            finish: "Continue",
            finish_tooltip: "Commit the resolved changes and replay the next commits",
        },
        Underway::CherryPick(picking) => Wording {
            title: format!("Cherry-picking onto {}", picking.branch).into(),
            step: Some(picked(picking)),
            abort: "Abort cherry-pick",
            abort_tooltip: "Put the branch back where it was before the cherry-pick",
            finish: "Commit",
            finish_tooltip: "Commit the resolved changes with the picked commit's message and author",
        },
        Underway::Revert(picking) => Wording {
            title: format!("Reverting on {}", picking.branch).into(),
            step: Some(picked(picking)),
            abort: "Abort revert",
            abort_tooltip: "Put the branch back where it was before the revert",
            finish: "Commit revert",
            finish_tooltip: "Commit the resolved changes with the usual Revert message",
        },
    }
}

fn picked(picking: &Picking) -> Step {
    Step {
        position: picking
            .place
            .map(|place| format!("Commit {} of {}", place.at, place.of).into()),
        id: picking.commit.id.clone().into(),
        subject: picking.commit.subject.clone().into(),
    }
}

fn step(rebasing: &Rebasing) -> Option<Step> {
    let replaying = rebasing.replaying.as_ref()?;
    Some(Step {
        position: Some(format!("Commit {} of {}", rebasing.step, rebasing.total).into()),
        id: replaying.id.clone().into(),
        subject: replaying.subject.clone().into(),
    })
}

pub(crate) fn waiting_for(conflicts: usize) -> String {
    format!(
        "Resolve the {} first",
        counted(conflicts, "conflicted file", "conflicted files")
    )
}

#[derive(IntoElement)]
pub(crate) struct OperationBar {
    underway: Underway,
    conflicts: usize,
    finishing: Option<Finish>,
    writing: bool,
    on_finish: Option<Handler<Finish>>,
}

impl OperationBar {
    pub(crate) fn new(underway: Underway, conflicts: usize) -> Self {
        Self {
            underway,
            conflicts,
            finishing: None,
            writing: false,
            on_finish: None,
        }
    }

    pub(crate) fn finishing(mut self, finishing: Option<Finish>, writing: bool) -> Self {
        self.finishing = finishing;
        self.writing = writing;
        self
    }

    pub(crate) fn on_finish(
        mut self,
        callback: impl Fn(&Finish, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_finish = Some(handler(callback));
        self
    }

    fn button(&self, finish: Finish, label: &'static str, tooltip: SharedString) -> Button {
        let on_finish = self.on_finish.clone();
        let blocked = finish == Finish::Continue && self.conflicts > 0;
        Button::new(match finish {
            Finish::Continue => "operation-continue",
            Finish::Abort => "operation-abort",
        })
        .small()
        .label(label)
        .tooltip(tooltip)
        .loading(self.finishing == Some(finish))
        .disabled(self.writing || blocked)
        .when_some(on_finish, |button, on_finish| {
            button.on_click(move |_, window, cx| on_finish(&finish, window, cx))
        })
    }
}

impl RenderOnce for OperationBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let wording = wording(&self.underway);
        let stuck = self.conflicts > 0;
        let tint = if stuck { theme.yellow } else { theme.green };
        let muted = theme.muted_foreground;
        let step = wording.step.as_ref().map(|step| {
            h_flex()
                .gap_2()
                .min_w_0()
                .children(
                    step.position
                        .clone()
                        .map(|position| div().flex_none().text_color(muted).child(position)),
                )
                .child(
                    div()
                        .flex_none()
                        .text_color(muted)
                        .font_family(theme.mono_font_family.clone())
                        .child(step.id.clone()),
                )
                .child(div().min_w_0().ellipsis().child(step.subject.clone()))
        });
        let state = if stuck {
            div().text_color(theme.magenta).child(counted(
                self.conflicts,
                "conflicted file",
                "conflicted files",
            ))
        } else {
            div().text_color(theme.green).child("No conflicts left")
        };
        let finish_tooltip = if stuck {
            SharedString::from(waiting_for(self.conflicts))
        } else {
            wording.finish_tooltip.into()
        };
        let abort = self
            .button(Finish::Abort, wording.abort, wording.abort_tooltip.into())
            .ghost()
            .icon(IconName::Undo2);
        let finish = self
            .button(Finish::Continue, wording.finish, finish_tooltip)
            .primary();
        h_flex()
            .flex_none()
            .h(px(HEIGHT))
            .px_3()
            .gap(px(10.))
            .sm_text()
            .bg(tint.opacity(TINT))
            .border_b_1()
            .border_color(theme.border)
            .child(
                Icon::new(if stuck {
                    IconName::TriangleAlert
                } else {
                    IconName::CircleCheck
                })
                .text_color(tint),
            )
            .child(div().flex_none().font_semibold().child(wording.title))
            .child(h_flex().flex_1().min_w_0().children(step))
            .child(div().flex_none().xs_text().child(state))
            .child(abort)
            .child(finish)
    }
}

#[cfg(test)]
mod tests {
    use super::{Step, waiting_for, wording};
    use crate::git::{Merging, Picking, Rebasing, Replay, Replaying, Underway};

    #[test]
    fn a_rebase_says_which_commit_of_how_many_stopped() {
        let rebase = Underway::Rebase(Rebasing {
            branch: "main".into(),
            onto: "origin/main".into(),
            step: 2,
            total: 3,
            replaying: Some(Replaying {
                id: "3f2a91c".into(),
                subject: "Show conflicted files in the changes list".into(),
            }),
        });
        let words = wording(&rebase);
        assert_eq!(words.title, "Rebasing main onto origin/main");
        assert_eq!(
            words.step,
            Some(Step {
                position: Some("Commit 2 of 3".into()),
                id: "3f2a91c".into(),
                subject: "Show conflicted files in the changes list".into(),
            })
        );
        assert_eq!((words.abort, words.finish), ("Abort rebase", "Continue"));
    }

    #[test]
    fn a_merge_is_finished_with_a_commit() {
        let merge = Underway::Merge(Merging {
            from: "feature/blame".into(),
            into: "main".into(),
        });
        let words = wording(&merge);
        assert_eq!(words.title, "Merging feature/blame into main");
        assert_eq!(words.step, None);
        assert_eq!((words.abort, words.finish), ("Abort merge", "Commit merge"));
        assert_eq!(waiting_for(1), "Resolve the 1 conflicted file first");
    }

    #[test]
    fn a_cherry_pick_and_a_revert_show_the_commit_without_a_position() {
        let commit = Replaying {
            id: "8ff3473".into(),
            subject: "Render side-by-side diffs".into(),
        };
        let picking = Picking {
            branch: "main".into(),
            commit,
            place: None,
        };
        let words = wording(&Underway::CherryPick(picking.clone()));
        assert_eq!(words.title, "Cherry-picking onto main");
        assert_eq!(
            words.step,
            Some(Step {
                position: None,
                id: "8ff3473".into(),
                subject: "Render side-by-side diffs".into(),
            })
        );
        assert_eq!((words.abort, words.finish), ("Abort cherry-pick", "Commit"));
        let words = wording(&Underway::Revert(picking.clone()));
        assert_eq!(words.title, "Reverting on main");
        assert_eq!((words.abort, words.finish), ("Abort revert", "Commit revert"));
        let several = Picking {
            place: Some(crate::git::Place { at: 2, of: 3 }),
            ..picking
        };
        let position = wording(&Underway::CherryPick(several)).step.and_then(|step| step.position);
        assert_eq!(position.as_deref(), Some("Commit 2 of 3"));
    }

    #[test]
    fn a_squash_and_a_rebase_then_fast_forward_read_as_the_merge_mode_chosen() {
        let squash = |from: &str| {
            wording(&Underway::Squash(Merging {
                from: from.into(),
                into: "main".into(),
            }))
        };
        let words = squash("feature/blame");
        assert_eq!(words.title, "Squashing feature/blame into main");
        assert_eq!((words.abort, words.finish), ("Abort squash", "Commit squash"));
        assert_eq!(squash("").title, "Squashing into main");
        let replay = Underway::Replay(Replay {
            from: "feature/blame".into(),
            picking: Picking {
                branch: "main".into(),
                commit: Replaying {
                    id: "e3779b1".into(),
                    subject: "Parse git diff output into hunks".into(),
                },
                place: None,
            },
        });
        let words = wording(&replay);
        assert_eq!(words.title, "Rebasing feature/blame onto main");
        assert_eq!(words.step.map(|step| step.id), Some("e3779b1".into()));
        assert_eq!((words.abort, words.finish), ("Abort rebase", "Continue"));
    }
}
