use gpui_kit::{
    App, Entity, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _, TitleBar, h_flex, menu::AppMenuBar,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::logo::logo;
use crate::ui::Look as _;

#[derive(IntoElement)]
pub(crate) struct TopBar {
    menu_bar: Option<Entity<AppMenuBar>>,
    repository: Option<SharedString>,
    worktree: Option<SharedString>,
    head: Option<SharedString>,
}

impl TopBar {
    pub(crate) fn new(menu_bar: Option<Entity<AppMenuBar>>) -> Self {
        Self {
            menu_bar,
            repository: None,
            worktree: None,
            head: None,
        }
    }

    pub(crate) fn worktree(mut self, worktree: Option<SharedString>) -> Self {
        self.worktree = worktree;
        self
    }

    pub(crate) fn repository(
        mut self,
        name: Option<SharedString>,
        head: Option<SharedString>,
    ) -> Self {
        self.repository = name;
        self.head = head;
        self
    }
}

impl RenderOnce for TopBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, border) = (theme.muted_foreground, theme.border);
        let head = self.head;
        let worktree = self.worktree;
        let pill = move |icon: IconName, text: SharedString| {
            h_flex()
                .min_w_0()
                .gap_1()
                .px_1p5()
                .py_0p5()
                .rounded(px(4.))
                .border_1()
                .border_color(border)
                .xs_text()
                .text_color(muted)
                .child(Icon::new(icon).xsmall())
                .child(div().truncate().child(text))
        };
        TitleBar::new().child(
            h_flex()
                .size_full()
                .px_1()
                .pr_3()
                .gap_2()
                .child(logo())
                .when_some(self.menu_bar, |this, menu_bar| {
                    this.child(div().flex_none().h_full().child(menu_bar))
                })
                .child(div().flex_1())
                .when_some(self.repository, |this, name| {
                    this.child(
                        h_flex()
                            .min_w_0()
                            .gap_2()
                            .child(div().sm_text().font_medium().truncate().child(name))
                            .when_some(worktree, |this, worktree| {
                                this.child(pill(IconName::FolderGit, worktree))
                            })
                            .when_some(head, |this, head| this.child(pill(IconName::GitBranch, head))),
                    )
                }),
        )
    }
}
