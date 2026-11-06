use gpui_kit::{
    AnyElement, App, Div, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex, tooltip::Tooltip},
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{look::Look as _, text::thousands};
use crate::git::{ChangeKind, RefKind, ShownRef};

const HEAD_FILL: f32 = 0.35;
const LABEL_FILL: f32 = 0.14;
const LABEL_BORDER: f32 = 0.6;
const CHANGE_FILL: f32 = 0.18;
const PULL_FILL: f32 = 0.16;

fn ref_icon(kind: RefKind) -> IconName {
    match kind {
        RefKind::LocalBranch => IconName::GitBranch,
        RefKind::RemoteBranch => IconName::Cloud,
        RefKind::Tag => IconName::Tag,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PullMark {
    pub(crate) number: u64,
    pub(crate) draft: bool,
}

pub(crate) fn pull_chip(mark: PullMark, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let (color, icon) = if mark.draft {
        (theme.muted_foreground, IconName::GitPullRequestDraft)
    } else {
        (theme.green, IconName::GitPullRequest)
    };
    let words = format!(
        "Pull request #{}{}",
        mark.number,
        if mark.draft { ", a draft" } else { "" }
    );
    h_flex()
        .id(SharedString::from(format!("pull-chip-{}", mark.number)))
        .flex_none()
        .h(px(16.))
        .pl(px(3.))
        .pr(px(5.))
        .gap(px(2.))
        .rounded(px(8.))
        .bg(color.opacity(PULL_FILL))
        .text_color(color)
        .text_size(px(11.))
        .line_height(px(16.))
        .font_medium()
        .child(Icon::new(icon).xsmall().text_color(color))
        .child(mark.number.to_string())
        .tooltip(move |window, cx| Tooltip::new(words.clone()).build(window, cx))
        .into_any_element()
}

pub(crate) fn ref_badge(shown: ShownRef, color: Hsla, pull: Option<PullMark>, cx: &App) -> Stateful<Div> {
    let label = shown.label;
    let icons = h_flex()
        .flex_none()
        .gap(px(2.))
        .child(Icon::new(ref_icon(label.kind)).xsmall().text_color(color))
        .when(shown.with_upstream, |this| {
            this.child(
                Icon::new(ref_icon(RefKind::RemoteBranch))
                    .xsmall()
                    .text_color(color),
            )
        });
    badge(icons, label.name.clone(), label.is_head, color, cx)
        .when_some(pull, |badge, pull| badge.child(pull_chip(pull, cx)))
        .id(label.full_name())
}

pub(crate) fn stash_badge(name: SharedString, color: Hsla, cx: &App) -> AnyElement {
    let icon = Icon::new(IconName::Archive).xsmall().text_color(color);
    badge(icon, name, false, color, cx).into_any_element()
}

fn badge(
    icons: impl IntoElement,
    name: SharedString,
    checked_out: bool,
    color: Hsla,
    cx: &App,
) -> Div {
    h_flex()
        .min_w(px(32.))
        .max_w(px(160.))
        .h(px(20.))
        .px_1p5()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(color.opacity(LABEL_BORDER))
        .bg(color.opacity(if checked_out { HEAD_FILL } else { LABEL_FILL }))
        .xs_text()
        .text_color(cx.theme().foreground)
        .child(icons)
        .child(
            div()
                .ellipsis()
                .when(checked_out, |this| this.font_semibold())
                .child(name),
        )
}

fn change_color(kind: ChangeKind, cx: &App) -> Hsla {
    let theme = cx.theme();
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => theme.green,
        ChangeKind::Deleted => theme.red,
        ChangeKind::Modified => theme.yellow,
        ChangeKind::Renamed | ChangeKind::Copied => theme.blue,
        ChangeKind::Conflicted => theme.magenta,
    }
}

pub(crate) fn change_badge(kind: ChangeKind, cx: &App) -> AnyElement {
    let color = change_color(kind, cx);
    div()
        .flex_none()
        .w(px(18.))
        .h(px(18.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(color.opacity(CHANGE_FILL))
        .xs_text()
        .font_semibold()
        .font_family(cx.theme().mono_font_family.clone())
        .text_color(color)
        .child(kind.status_letter())
        .into_any_element()
}

pub(crate) fn additions(count: usize, cx: &App) -> Div {
    div()
        .text_color(cx.theme().green)
        .child(format!("+{}", thousands(count)))
}

pub(crate) fn deletions(count: usize, cx: &App) -> Div {
    div()
        .text_color(cx.theme().red)
        .child(format!("−{}", thousands(count)))
}
