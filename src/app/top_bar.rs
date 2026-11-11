use gpui_kit::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _, TitleBar,
        button::{Button, ButtonCustomVariant, ButtonVariants as _},
        h_flex,
        menu::AppMenuBar,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    logo::logo,
    updates::{
        DOWNLOADING, OPEN_RELEASE_PAGE, RESTART_TO_UPDATE, Status, UPDATE_THEN_RESTART, WHATS_NEW, download,
        downloading, ready_tip, release_out, restart_to_update, update_to,
    },
};
use crate::{update::Release, ui::Look as _};

const TINT: f32 = 0.16;
const TINT_HOVERED: f32 = 0.24;
const TINT_PRESSED: f32 = 0.30;

#[derive(IntoElement)]
pub(crate) struct TopBar {
    menu_bar: Option<Entity<AppMenuBar>>,
    repository: Option<SharedString>,
    worktree: Option<SharedString>,
    head: Option<SharedString>,
    update: Option<Status>,
}

impl TopBar {
    pub(crate) fn new(menu_bar: Option<Entity<AppMenuBar>>) -> Self {
        Self {
            menu_bar,
            repository: None,
            worktree: None,
            head: None,
            update: None,
        }
    }

    pub(crate) fn update(mut self, status: &Status) -> Self {
        self.update = Some(status.clone());
        self
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
        let update = self.update.and_then(|status| update_button(&status, cx));
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
                })
                .when_some(update, |this, update| this.child(update)),
        )
    }
}

pub(crate) fn update_variant(cx: &App) -> ButtonCustomVariant {
    let green = cx.theme().green;
    ButtonCustomVariant::new(cx)
        .color(green.opacity(TINT))
        .foreground(green)
        .hover(green.opacity(TINT_HOVERED))
        .active(green.opacity(TINT_PRESSED))
}

pub(crate) fn whats_new(id: &'static str, release: &Release) -> Button {
    let notes = release.notes.to_string();
    Button::new(id)
        .ghost()
        .label(WHATS_NEW)
        .on_click(move |_, _, cx| cx.open_url(&notes))
}

fn update_button(status: &Status, cx: &App) -> Option<AnyElement> {
    let buttons: Vec<AnyElement> = match status {
        Status::Ready { release, .. } => vec![
            Button::new("restart-to-update")
                .custom(update_variant(cx))
                .xsmall()
                .icon(IconName::RefreshCw)
                .label(RESTART_TO_UPDATE)
                .tooltip(ready_tip(release))
                .on_click(|_, _, cx| restart_to_update(cx))
                .into_any_element(),
            whats_new("top-bar-whats-new", release).xsmall().into_any_element(),
        ],
        Status::Out(release) => vec![
            Button::new("update-to")
                .custom(update_variant(cx))
                .xsmall()
                .icon(IconName::Download)
                .label(update_to(release))
                .tooltip(UPDATE_THEN_RESTART)
                .on_click(|_, _, cx| download(cx))
                .into_any_element(),
            whats_new("top-bar-whats-new", release).xsmall().into_any_element(),
        ],
        Status::Downloading { release, asked: true } => vec![
            Button::new("downloading-update")
                .custom(update_variant(cx))
                .xsmall()
                .loading(true)
                .disabled(true)
                .label(DOWNLOADING)
                .tooltip(downloading(release))
                .into_any_element(),
        ],
        Status::ReadOnly(release) => {
            let notes = release.notes.to_string();
            vec![
                Button::new("release-is-out")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ExternalLink)
                    .label(release_out(release))
                    .tooltip(OPEN_RELEASE_PAGE)
                    .on_click(move |_, _, cx| cx.open_url(&notes))
                    .into_any_element(),
            ]
        }
        _ => return None,
    };
    Some(h_flex().flex_none().gap_1().occlude().children(buttons).into_any_element())
}
