use gpui_kit::{
    AnyElement, App, Div, Hsla, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, StyledExt as _, h_flex},
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{look::Look as _, text::thousands};
use crate::git::{ChangeKind, RefKind, RefLabel};

const HEAD_FILL: f32 = 0.35;
const LABEL_FILL: f32 = 0.14;
const LABEL_BORDER: f32 = 0.6;
const CHANGE_FILL: f32 = 0.18;

fn ref_icon(kind: RefKind) -> IconName {
    match kind {
        RefKind::LocalBranch => IconName::GitBranch,
        RefKind::RemoteBranch => IconName::Cloud,
        RefKind::Tag => IconName::Tag,
    }
}

pub(crate) fn ref_badge(label: &RefLabel, color: Hsla, cx: &App) -> AnyElement {
    h_flex()
        .min_w(px(32.))
        .max_w(px(160.))
        .h(px(20.))
        .px_1p5()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(color.opacity(LABEL_BORDER))
        .bg(color.opacity(if label.is_head { HEAD_FILL } else { LABEL_FILL }))
        .text_xs()
        .text_color(cx.theme().foreground)
        .child(Icon::new(ref_icon(label.kind)).xsmall().text_color(color))
        .child(
            div()
                .ellipsis()
                .when(label.is_head, |this| this.font_semibold())
                .child(label.name.clone()),
        )
        .into_any_element()
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
        .text_xs()
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
