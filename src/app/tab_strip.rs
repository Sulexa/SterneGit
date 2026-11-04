use gpui_kit::{
    App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        spinner::Spinner,
        tab::{Tab, TabBar},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    actions::{CloseTab, OpenRepository, WINDOW_CONTEXT},
    ui::{Handler, handler},
};

const MIN_TAB_WIDTH: f32 = 120.;
const MAX_TAB_WIDTH: f32 = 240.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabState {
    Reading,
    Failed,
    Ready,
}

pub(crate) struct TabLabel {
    pub(crate) name: SharedString,
    pub(crate) state: TabState,
}

#[derive(IntoElement)]
pub(crate) struct TabStrip {
    labels: Vec<TabLabel>,
    active: usize,
    on_select: Option<Handler<usize>>,
    on_close: Option<Handler<usize>>,
    on_open: Option<Handler<()>>,
}

impl TabStrip {
    pub(crate) fn new(labels: Vec<TabLabel>, active: usize) -> Self {
        Self {
            labels,
            active,
            on_select: None,
            on_close: None,
            on_open: None,
        }
    }

    pub(crate) fn on_select(
        mut self,
        callback: impl Fn(&usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(handler(callback));
        self
    }

    pub(crate) fn on_close(
        mut self,
        callback: impl Fn(&usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(handler(callback));
        self
    }

    pub(crate) fn on_open(
        mut self,
        callback: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(handler(callback));
        self
    }
}

impl RenderOnce for TabStrip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let failed_color = cx.theme().red;
        let on_close = self.on_close;
        let tabs = self.labels.into_iter().enumerate().map(|(ix, label)| {
            let on_close = on_close.clone();
            Tab::new()
                .min_w(px(MIN_TAB_WIDTH))
                .max_w(px(MAX_TAB_WIDTH))
                .map(|tab| match label.state {
                    TabState::Reading => tab.prefix(div().pl_3().child(Spinner::new().xsmall())),
                    TabState::Failed => tab.prefix(
                        div().pl_3().child(
                            Icon::new(IconName::TriangleAlert)
                                .xsmall()
                                .text_color(failed_color),
                        ),
                    ),
                    TabState::Ready => tab,
                })
                .child(div().truncate().child(label.name))
                .suffix(
                    Button::new(("close-tab", ix))
                        .ghost()
                        .icon(IconName::Close)
                        .xsmall()
                        .ml(-px(8.))
                        .mr_2()
                        .tab_stop(false)
                        .tooltip_with_action("Close tab", &CloseTab, Some(WINDOW_CONTEXT))
                        .when_some(on_close, |button, on_close| {
                            button.on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                on_close(&ix, window, cx)
                            })
                        }),
                )
        });
        TabBar::new("repo-tabs")
            .children(tabs.collect::<Vec<_>>())
            .selected_index(self.active)
            .when_some(self.on_select, |bar, on_select| {
                bar.on_click(move |ix, window, cx| on_select(ix, window, cx))
            })
            .suffix(
                h_flex().h_full().px_1().child(
                    Button::new("new-tab")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Plus)
                        .tooltip_with_action(
                            "Open a repository in a new tab",
                            &OpenRepository,
                            Some(WINDOW_CONTEXT),
                        )
                        .when_some(self.on_open, |button, on_open| {
                            button.on_click(move |_, window, cx| on_open(&(), window, cx))
                        }),
                ),
            )
    }
}
