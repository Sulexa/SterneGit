use gpui_kit::{
    Anchor, AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
        notification::Notification,
        spinner::Spinner,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    operation::{Operated, Operation},
    repository::Repository,
};
use crate::{
    git::{PullMode, Repo},
    ui::{Handler, Look as _, handler},
};

const HEIGHT: f32 = 48.;
const BUTTON_HEIGHT: f32 = 40.;
const BUTTON_MIN_WIDTH: f32 = 56.;
const CARET_WIDTH: f32 = 16.;
const SEPARATOR_HEIGHT: f32 = 24.;
const PULL_MENU_WIDTH: f32 = 280.;
const PULL_SPLIT: &str = "toolbar-pull-split";
const OTHER_HALF_HOVER_OPACITY: f32 = 0.25;
const GROUPS: [&[Operation]; 3] = [
    &[Operation::Pull, Operation::Push],
    &[Operation::Branch],
    &[Operation::Stash, Operation::Pop],
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ToolbarState {
    open: bool,
    busy: bool,
    running: Option<Operation>,
    on_branch: bool,
    has_commit: bool,
    has_changes: bool,
    has_stash: bool,
    pull_mode: PullMode,
}

impl ToolbarState {
    pub(crate) fn of(repository: &Repository) -> Self {
        let history = repository.history();
        Self {
            open: history.is_some() && repository.repo().is_some_and(Repo::has_work_tree),
            busy: repository.is_writing(),
            running: repository.running(),
            on_branch: history.is_some_and(|history| history.branch.is_some()),
            has_commit: history.is_some_and(|history| history.head_id.is_some()),
            has_changes: repository.has_changes(),
            has_stash: repository.has_stash(),
            pull_mode: repository.pull_mode(),
        }
    }

    fn enabled(&self, operation: Operation) -> bool {
        let possible = match operation {
            Operation::Pull => self.on_branch || self.pull_mode == PullMode::FetchOnly,
            Operation::Push => self.on_branch && self.has_commit,
            Operation::Branch => self.has_commit,
            Operation::Stash => self.has_changes && self.has_commit,
            Operation::Pop => self.has_stash,
        };
        self.open && !self.busy && possible
    }

    fn is_running(&self, operation: Operation) -> bool {
        self.running == Some(operation)
    }
}

#[derive(IntoElement)]
pub(crate) struct RepoToolbar {
    state: ToolbarState,
    on_run: Option<Handler<Operation>>,
    on_pull_mode: Option<Handler<PullMode>>,
}

impl RepoToolbar {
    pub(crate) fn new(state: ToolbarState) -> Self {
        Self {
            state,
            on_run: None,
            on_pull_mode: None,
        }
    }

    pub(crate) fn on_run(
        mut self,
        callback: impl Fn(&Operation, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_run = Some(handler(callback));
        self
    }

    pub(crate) fn on_pull_mode(
        mut self,
        callback: impl Fn(&PullMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pull_mode = Some(handler(callback));
        self
    }

    fn control(&self, operation: Operation, cx: &App) -> AnyElement {
        match operation {
            Operation::Pull => self.pull_button(cx),
            _ => self.toolbar_button(operation).into_any_element(),
        }
    }

    fn toolbar_button(&self, operation: Operation) -> Button {
        let running = self.state.is_running(operation);
        let glyph = if running {
            Spinner::new().into_any_element()
        } else {
            Icon::new(icon(operation)).into_any_element()
        };
        let on_run = self.on_run.clone();
        Button::new(button_id(operation))
            .ghost()
            .h(px(BUTTON_HEIGHT))
            .min_w(px(BUTTON_MIN_WIDTH))
            .px_2()
            .tooltip(tooltip(operation, self.state.pull_mode))
            .loading(running)
            .disabled(!running && !self.state.enabled(operation))
            .child(
                v_flex()
                    .items_center()
                    .gap_0p5()
                    .child(glyph)
                    .child(div().xs_text().child(operation.label())),
            )
            .when_some(on_run, |button, on_run| {
                button.on_click(move |_, window, cx| on_run(&operation, window, cx))
            })
    }

    fn pull_button(&self, cx: &App) -> AnyElement {
        let other_half = cx.theme().accent.opacity(OTHER_HALF_HOVER_OPACITY);
        let pull_enabled = self.state.enabled(Operation::Pull);
        let mode = self.state.pull_mode;
        let on_pull_mode = self.on_pull_mode.clone();
        h_flex()
            .group(PULL_SPLIT)
            .child(
                self.toolbar_button(Operation::Pull)
                    .rounded_r(px(0.))
                    .when(pull_enabled, |button| {
                        button.group_hover(PULL_SPLIT, |style| style.bg(other_half))
                    }),
            )
            .child(
                Button::new("toolbar-pull-mode")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronDown)
                    .h(px(BUTTON_HEIGHT))
                    .w(px(CARET_WIDTH))
                    .rounded_l(px(0.))
                    .tooltip("Choose how Pull works")
                    .disabled(!self.state.open)
                    .when(self.state.open, |button| {
                        button.group_hover(PULL_SPLIT, |style| style.bg(other_half))
                    })
                    .dropdown_menu_with_anchor(Anchor::TopLeft, move |menu, _, _| {
                        pull_mode_menu(menu, mode, on_pull_mode.clone())
                    }),
            )
            .into_any_element()
    }
}

impl RenderOnce for RepoToolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border = cx.theme().border;
        let mut items = Vec::with_capacity(GROUPS.len() * 2);
        for (ix, group) in GROUPS.into_iter().enumerate() {
            if ix > 0 {
                items.push(
                    div()
                        .w(px(1.))
                        .h(px(SEPARATOR_HEIGHT))
                        .bg(border)
                        .into_any_element(),
                );
            }
            let controls = group.iter().map(|&operation| self.control(operation, cx));
            items.push(h_flex().gap_1().children(controls).into_any_element());
        }
        h_flex()
            .flex_none()
            .h(px(HEIGHT))
            .justify_center()
            .gap_2()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(border)
            .children(items)
    }
}

fn pull_mode_menu(
    menu: PopupMenu,
    current: PullMode,
    on_pull_mode: Option<Handler<PullMode>>,
) -> PopupMenu {
    let item = |mode: PullMode| {
        let on_pull_mode = on_pull_mode.clone();
        PopupMenuItem::element(move |_, cx| {
            v_flex().child(mode_label(mode)).child(
                div()
                    .xs_text()
                    .text_color(cx.theme().muted_foreground)
                    .child(mode_about(mode)),
            )
        })
        .checked(mode == current)
        .on_click(move |_, window, cx| {
            if let Some(on_pull_mode) = &on_pull_mode {
                on_pull_mode(&mode, window, cx)
            }
        })
    };
    menu.min_w(px(PULL_MENU_WIDTH))
        .label("Pull mode")
        .item(item(PullMode::FastForwardIfPossible))
        .item(item(PullMode::FastForwardOnly))
        .item(item(PullMode::Rebase))
        .separator()
        .item(item(PullMode::FetchOnly))
}

pub(crate) fn notification(operated: &Operated) -> Notification {
    match &operated.outcome {
        Ok(message) => Notification::success(message.clone()),
        Err(failure) => Notification::error(failure.message())
            .title(operated.failed.clone())
            .autohide(false),
    }
}

fn button_id(operation: Operation) -> &'static str {
    match operation {
        Operation::Pull => "toolbar-pull",
        Operation::Push => "toolbar-push",
        Operation::Branch => "toolbar-branch",
        Operation::Stash => "toolbar-stash",
        Operation::Pop => "toolbar-pop",
    }
}

fn icon(operation: Operation) -> IconName {
    match operation {
        Operation::Pull => IconName::ArrowDownToLine,
        Operation::Push => IconName::ArrowUpFromLine,
        Operation::Branch => IconName::GitBranchPlus,
        Operation::Stash => IconName::Archive,
        Operation::Pop => IconName::ArchiveRestore,
    }
}

fn tooltip(operation: Operation, mode: PullMode) -> SharedString {
    match operation {
        Operation::Pull => format!("Pull: {}", mode_label(mode).to_lowercase()).into(),
        Operation::Push => "Send your new commits to the upstream branch".into(),
        Operation::Branch => "Create a branch where HEAD is and switch to it".into(),
        Operation::Stash => "Put every uncommitted change aside, untracked files included".into(),
        Operation::Pop => "Bring back the latest stash and drop it".into(),
    }
}

pub(super) fn mode_label(mode: PullMode) -> &'static str {
    match mode {
        PullMode::FastForwardIfPossible => "Fast-forward if possible",
        PullMode::FastForwardOnly => "Fast-forward only",
        PullMode::Rebase => "Rebase",
        PullMode::FetchOnly => "Fetch only",
    }
}

fn mode_about(mode: PullMode) -> &'static str {
    match mode {
        PullMode::FastForwardIfPossible => "Merges when the branches have diverged",
        PullMode::FastForwardOnly => "Stops when the branches have diverged",
        PullMode::Rebase => "Replays your commits on top of the upstream ones",
        PullMode::FetchOnly => "Updates the remote branches, leaves yours alone",
    }
}

#[cfg(test)]
mod tests {
    use super::{Operation, ToolbarState};
    use crate::git::PullMode;

    fn ready() -> ToolbarState {
        ToolbarState {
            open: true,
            on_branch: true,
            has_commit: true,
            has_changes: true,
            ..ToolbarState::default()
        }
    }

    fn enabled(state: ToolbarState) -> Vec<Operation> {
        [
            Operation::Pull,
            Operation::Push,
            Operation::Branch,
            Operation::Stash,
            Operation::Pop,
        ]
        .into_iter()
        .filter(|&operation| state.enabled(operation))
        .collect()
    }

    #[test]
    fn pop_waits_for_a_stash_and_stash_for_changes() {
        use Operation::*;
        assert_eq!(enabled(ready()), [Pull, Push, Branch, Stash]);
        let clean_with_a_stash = ToolbarState {
            has_changes: false,
            has_stash: true,
            ..ready()
        };
        assert_eq!(enabled(clean_with_a_stash), [Pull, Push, Branch, Pop]);
    }

    #[test]
    fn a_detached_head_can_only_fetch_branch_and_stash() {
        use Operation::*;
        let detached = ToolbarState {
            on_branch: false,
            ..ready()
        };
        assert_eq!(enabled(detached), [Branch, Stash]);
        let fetching = ToolbarState {
            pull_mode: PullMode::FetchOnly,
            ..detached
        };
        assert_eq!(enabled(fetching), [Pull, Branch, Stash]);
    }

    #[test]
    fn nothing_runs_while_git_writes_or_before_the_repository_is_read() {
        let writing = ToolbarState {
            busy: true,
            running: Some(Operation::Pull),
            ..ready()
        };
        assert!(enabled(writing).is_empty());
        assert!(writing.is_running(Operation::Pull));
        let unread = ToolbarState {
            open: false,
            ..ready()
        };
        assert!(enabled(unread).is_empty());
    }
}
