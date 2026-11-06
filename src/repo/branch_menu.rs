use gpui_kit::{
    App, Entity, SharedString, Window,
    component::{
        WindowExt as _,
        button::ButtonVariant,
        menu::{PopupMenu, PopupMenuItem},
    },
};

use std::sync::Arc;

use super::{pulls::PullMarks, repository::Repository, toolbar::mode_label};
use crate::{
    git::{HISTORY_LIMIT, History, PullMode, RefKind, RefLabel, RemoteBranch},
    ui::{Handler, handler, thousands},
};

const PULL_FAST_FORWARD_ONLY: &str = "Pull (fast-forward only)";
const PUSH: &str = "Push";
const BRANCH_HERE: &str = "Create branch here…";
const DELETE: &str = "Delete";
const CREATE_PULL: &str = "Create pull request…";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BranchAction {
    CheckOut,
    Pull,
    Push,
    Merge,
    Rebase,
    BranchHere,
    NewWorktree,
    OpenWorktree,
    Delete,
    ShowPull(u64),
    CreatePull,
}

#[derive(Clone, Default)]
pub(crate) struct PullOffers {
    pub(crate) marks: Arc<PullMarks>,
    pub(crate) creates: bool,
    pub(crate) default_branch: Option<String>,
}

impl PullOffers {
    fn offered(&self, label: &RefLabel) -> Option<BranchAction> {
        if let Some(mark) = self.marks.get(&label.full_name()) {
            return Some(BranchAction::ShowPull(mark.number));
        }
        let default = self.default_branch.as_deref() == Some(label.local_name());
        (self.creates && !default).then_some(BranchAction::CreatePull)
    }
}

#[derive(Clone)]
pub(crate) struct BranchMenu {
    checked_out: Option<SharedString>,
    pull_mode: PullMode,
    writing: bool,
    pulls: Option<PullOffers>,
    on_action: Option<Handler<(RefLabel, BranchAction)>>,
}

impl BranchMenu {
    pub(crate) fn new(
        checked_out: Option<SharedString>,
        pull_mode: PullMode,
        writing: bool,
    ) -> Self {
        Self {
            checked_out,
            pull_mode,
            writing,
            pulls: None,
            on_action: None,
        }
    }

    pub(crate) fn pulls(mut self, offers: Option<PullOffers>) -> Self {
        self.pulls = offers;
        self
    }

    pub(crate) fn on_action(
        mut self,
        callback: impl Fn(&(RefLabel, BranchAction), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(handler(callback));
        self
    }

    pub(crate) fn build(&self, menu: PopupMenu, label: &RefLabel) -> PopupMenu {
        self.offered(label)
            .into_iter()
            .enumerate()
            .fold(menu, |menu, (ix, group)| {
                let menu = if ix > 0 { menu.separator() } else { menu };
                group
                    .into_iter()
                    .fold(menu, |menu, action| menu.item(self.item(action, label)))
            })
    }

    fn offered(&self, label: &RefLabel) -> Vec<Vec<BranchAction>> {
        use BranchAction::*;
        let checked_out = label.is_head;
        let local = label.kind == RefKind::LocalBranch;
        let another_branch_is_checked_out = self.checked_out.is_some() && !checked_out;
        let elsewhere = label.worktree.is_some();
        let worktree = match (checked_out, elsewhere) {
            (false, false) => vec![BranchHere, NewWorktree],
            _ => vec![BranchHere],
        };
        [
            label.can_be_checked_out().then(|| vec![CheckOut]),
            elsewhere.then(|| vec![OpenWorktree]),
            local.then(|| vec![Pull, Push]),
            self.pulls
                .as_ref()
                .and_then(|offers| offers.offered(label))
                .map(|action| vec![action]),
            another_branch_is_checked_out.then(|| vec![Merge, Rebase]),
            Some(worktree),
            (!checked_out && !elsewhere).then(|| vec![Delete]),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn text(&self, action: BranchAction, label: &RefLabel) -> String {
        let checked_out = self.checked_out.as_deref().unwrap_or_default();
        match action {
            BranchAction::CheckOut => format!("Check out {}", label.local_name()),
            BranchAction::Pull if label.is_head => {
                format!("Pull ({})", mode_label(self.pull_mode).to_lowercase())
            }
            BranchAction::Pull => PULL_FAST_FORWARD_ONLY.to_owned(),
            BranchAction::Push => PUSH.to_owned(),
            BranchAction::Merge => format!("Merge {} into {checked_out}", label.name),
            BranchAction::Rebase => format!("Rebase {checked_out} onto {}", label.name),
            BranchAction::BranchHere => BRANCH_HERE.to_owned(),
            BranchAction::NewWorktree => format!("New worktree for {}…", label.local_name()),
            BranchAction::OpenWorktree => format!(
                "Open the worktree {}",
                label.worktree.as_deref().unwrap_or_default()
            ),
            BranchAction::ShowPull(number) => format!("Show pull request #{number}"),
            BranchAction::CreatePull => CREATE_PULL.to_owned(),
            BranchAction::Delete => match label.on_remote() {
                Some(RemoteBranch { remote, branch }) => format!("Delete {branch} from {remote}…"),
                None => format!("Delete {}…", label.name),
            },
        }
    }

    fn item(&self, action: BranchAction, label: &RefLabel) -> PopupMenuItem {
        let on_action = self.on_action.clone();
        let chosen = (label.clone(), action);
        PopupMenuItem::new(self.text(action, label))
            .disabled(self.writing || on_action.is_none())
            .on_click(move |_, window, cx| {
                if let Some(on_action) = &on_action {
                    on_action(&chosen, window, cx)
                }
            })
    }
}

pub(crate) struct DeleteQuestion {
    pub(crate) title: String,
    pub(crate) description: String,
}

pub(crate) fn delete_question(label: &RefLabel, history: &History) -> DeleteQuestion {
    match label.on_remote() {
        Some(RemoteBranch { remote, branch }) => DeleteQuestion {
            title: format!("Delete {branch} from {remote}?"),
            description: format!(
                "The branch is removed from {remote} for everyone who uses it. \
                 A local {branch}, if you have one, stays."
            ),
        },
        None => DeleteQuestion {
            title: format!("Delete {}?", label.name),
            description: what_deleting_loses(&label.name, history.commits_only_on(label)),
        },
    }
}

fn what_deleting_loses(branch: &str, commits_only_on_it: Option<usize>) -> String {
    match commits_only_on_it {
        Some(0) => {
            format!("Every commit of {branch} is on another branch or tag too, so nothing is lost.")
        }
        Some(1) => format!("1 commit is on no other branch or tag. Deleting {branch} loses it."),
        Some(count) => format!(
            "{} commits are on no other branch or tag. Deleting {branch} loses them.",
            thousands(count)
        ),
        None => format!(
            "{branch} points to a commit older than the newest {} shown, \
             so SterneGit can't tell whether deleting it loses any commits.",
            thousands(HISTORY_LIMIT)
        ),
    }
}

pub(crate) fn confirm_delete(
    repository: Entity<Repository>,
    label: RefLabel,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(history) = repository.read(cx).history().cloned() else {
        return;
    };
    let question = delete_question(&label, &history);
    window.open_alert_dialog(cx, move |alert, _, _| {
        let repository = repository.clone();
        let label = label.clone();
        alert
            .confirm()
            .title(question.title.clone())
            .description(question.description.clone())
            .ok_text(DELETE)
            .ok_variant(ButtonVariant::Danger)
            .on_ok(move |_, _, cx| {
                repository.update(cx, |repository, cx| {
                    repository.delete_branch(label.clone(), cx)
                });
                true
            })
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{BranchAction::*, BranchMenu, PullOffers, what_deleting_loses};
    use crate::{
        git::{PullMode, RefKind, RefLabel, fixtures::id},
        repo::pulls::PullMarks,
        ui::PullMark,
    };

    fn label(name: &str, kind: RefKind, is_head: bool) -> RefLabel {
        RefLabel {
            name: name.to_owned().into(),
            kind,
            target: id(1),
            is_head,
            upstream: None,
            from_upstream: None,
            worktree: None,
        }
    }

    fn on_main() -> BranchMenu {
        BranchMenu::new(Some("main".into()), PullMode::FastForwardIfPossible, false)
    }

    fn texts(menu: &BranchMenu, label: &RefLabel) -> Vec<Vec<String>> {
        menu.offered(label)
            .into_iter()
            .map(|group| {
                group
                    .into_iter()
                    .map(|action| menu.text(action, label))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn another_local_branch_offers_every_action_with_both_names() {
        let blame = label("feature/blame", RefKind::LocalBranch, false);
        assert_eq!(
            texts(&on_main(), &blame),
            [
                vec!["Check out feature/blame"],
                vec!["Pull (fast-forward only)", "Push"],
                vec![
                    "Merge feature/blame into main",
                    "Rebase main onto feature/blame"
                ],
                vec!["Create branch here…", "New worktree for feature/blame…"],
                vec!["Delete feature/blame…"],
            ]
        );
    }

    #[test]
    fn with_a_connector_a_branch_offers_its_pull_request_or_to_create_one() {
        let marks: PullMarks = [("refs/heads/feature/diff-view".to_owned(), PullMark { number: 12, draft: false })].into();
        let offers = PullOffers {
            marks: Arc::new(marks),
            creates: true,
            default_branch: Some("main".into()),
        };
        let menu = on_main().pulls(Some(offers));
        let diff_view = label("feature/diff-view", RefKind::LocalBranch, false);
        assert_eq!(texts(&menu, &diff_view)[2], ["Show pull request #12"]);
        let tabs = label("topic/tabs", RefKind::RemoteBranch, false);
        assert!(texts(&menu, &tabs).contains(&vec!["Create pull request…".to_owned()]));
        let main = label("origin/main", RefKind::RemoteBranch, false);
        assert!(!menu.offered(&main).iter().flatten().any(|action| *action == CreatePull));
        assert!(!on_main().offered(&tabs).iter().flatten().any(|action| *action == CreatePull));
    }

    #[test]
    fn the_checked_out_branch_pulls_with_the_chosen_mode_and_cannot_be_deleted() {
        let main = label("main", RefKind::LocalBranch, true);
        assert_eq!(
            on_main().offered(&main),
            [vec![Pull, Push], vec![BranchHere]]
        );
        let rebasing = BranchMenu::new(Some("main".into()), PullMode::Rebase, false);
        assert_eq!(rebasing.text(Pull, &main), "Pull (rebase)");
    }

    #[test]
    fn a_remote_branch_checks_out_under_its_own_name_and_is_deleted_on_its_remote() {
        let remote = label("origin/feature/diff-view", RefKind::RemoteBranch, false);
        assert_eq!(
            texts(&on_main(), &remote),
            [
                vec!["Check out feature/diff-view"],
                vec![
                    "Merge origin/feature/diff-view into main",
                    "Rebase main onto origin/feature/diff-view"
                ],
                vec!["Create branch here…", "New worktree for feature/diff-view…"],
                vec!["Delete feature/diff-view from origin…"],
            ]
        );
    }

    #[test]
    fn a_detached_head_has_nothing_to_merge_into_or_rebase() {
        let detached = BranchMenu::new(None, PullMode::FastForwardIfPossible, false);
        let blame = label("feature/blame", RefKind::LocalBranch, false);
        assert_eq!(
            detached.offered(&blame),
            [
                vec![CheckOut],
                vec![Pull, Push],
                vec![BranchHere, NewWorktree],
                vec![Delete]
            ]
        );
    }

    #[test]
    fn a_branch_checked_out_in_another_worktree_opens_it_instead_of_checking_out_or_deleting() {
        let elsewhere = RefLabel {
            worktree: Some("diff-view".into()),
            ..label("feature/diff-view", RefKind::LocalBranch, false)
        };
        assert_eq!(
            texts(&on_main(), &elsewhere),
            [
                vec!["Open the worktree diff-view"],
                vec!["Pull (fast-forward only)", "Push"],
                vec![
                    "Merge feature/diff-view into main",
                    "Rebase main onto feature/diff-view"
                ],
                vec!["Create branch here…"],
            ]
        );
    }

    #[test]
    fn the_delete_question_says_how_many_commits_go_with_the_branch() {
        assert_eq!(
            what_deleting_loses("feature/blame", Some(1)),
            "1 commit is on no other branch or tag. Deleting feature/blame loses it."
        );
        assert_eq!(
            what_deleting_loses("feature/blame", Some(2_000)),
            "2,000 commits are on no other branch or tag. Deleting feature/blame loses them."
        );
        assert_eq!(
            what_deleting_loses("feature/stash", Some(0)),
            "Every commit of feature/stash is on another branch or tag too, so nothing is lost."
        );
    }
}
