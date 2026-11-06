use std::{path::Path, sync::Arc};

use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement as _, SharedString, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

use crate::{
    git::Worktree,
    ui::{Handler, Look as _},
};

pub(crate) fn worktree_bar(worktree: &Worktree, on_open: Handler<Arc<Path>>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let (muted, border, background) = (theme.muted_foreground, theme.border, theme.title_bar);
    let location = worktree.location.clone();
    let shown = location.clone();
    h_flex()
        .flex_none()
        .gap_2p5()
        .pl_3()
        .pr_2()
        .py_2()
        .border_b_1()
        .border_color(border)
        .bg(background)
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
                        .child(
                            h_flex()
                                .min_w_0()
                                .gap_1()
                                .xs_text()
                                .text_color(muted)
                                .when(worktree.branch.is_some(), |this| {
                                    this.child(Icon::new(IconName::GitBranch).xsmall())
                                })
                                .child(div().truncate().child(worktree.checked_out())),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .xs_text()
                        .text_color(muted)
                        .child(SharedString::from(worktree.location.display().to_string())),
                ),
        )
        .child(
            Button::new("worktree-show-folder")
                .ghost()
                .xsmall()
                .icon(IconName::FolderOpen)
                .tooltip("Show the folder in the file manager")
                .on_click(move |_, _, cx| cx.reveal_path(&shown)),
        )
        .child(
            Button::new("worktree-open-tab")
                .ghost()
                .xsmall()
                .icon(IconName::ExternalLink)
                .tooltip("Open this worktree in a new tab")
                .on_click(move |_, window, cx| on_open(&location, window, cx)),
        )
        .into_any_element()
}
