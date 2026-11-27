use gpui_kit::{
    AnyElement, App, AppContext as _, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, RenderOnce, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        tab::{Tab, TabBar},
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use crate::ui::nodding_logo;
use crate::{
    actions::{CloseTab, OpenRepository, WINDOW_CONTEXT},
    ui::{Handler, Look as _, handler},
};

const MIN_TAB_WIDTH: f32 = 120.;
const MAX_TAB_WIDTH: f32 = 240.;
const DRAGGED_TAB_OPACITY: f32 = 0.9;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabState {
    Reading,
    Failed,
    Ready,
}

#[derive(Clone)]
pub(crate) struct TabLabel {
    pub(crate) name: SharedString,
    pub(crate) state: TabState,
    pub(crate) icon: Option<IconName>,
}

#[derive(Clone, Copy)]
pub(crate) struct TabMove {
    pub(crate) from: usize,
    pub(crate) before: usize,
}

struct DraggedTab(usize);

#[derive(IntoElement)]
pub(crate) struct TabStrip {
    labels: Vec<TabLabel>,
    active: usize,
    on_select: Option<Handler<usize>>,
    on_close: Option<Handler<usize>>,
    on_move: Option<Handler<TabMove>>,
    on_open: Option<Handler<()>>,
}

impl TabStrip {
    pub(crate) fn new(labels: Vec<TabLabel>, active: usize) -> Self {
        Self {
            labels,
            active,
            on_select: None,
            on_close: None,
            on_move: None,
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

    pub(crate) fn on_move(
        mut self,
        callback: impl Fn(&TabMove, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(handler(callback));
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
        let on_close = self.on_close;
        let on_move = self.on_move;
        let past_the_last_tab = self.labels.len();
        let tabs = self
            .labels
            .into_iter()
            .enumerate()
            .map(|(ix, label)| {
                let on_close = on_close.clone();
                let dragged = DraggedTabPreview(label.clone());
                Tab::new()
                    .min_w(px(MIN_TAB_WIDTH))
                    .max_w(px(MAX_TAB_WIDTH))
                    .when_some(prefix(&label, cx), |tab, prefix| tab.prefix(prefix))
                    .child(div().flex_1().ellipsis().child(label.name))
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
                    .on_drag(DraggedTab(ix), move |_, _, _, cx| cx.new(|_| dragged.clone()))
                    .drag_over::<DraggedTab>(|style, _, _, cx| marks_the_drop(style, cx))
                    .when_some(on_move.clone(), |tab, on_move| {
                        tab.on_drop(move |dragged: &DraggedTab, window, cx| {
                            on_move(&TabMove { from: dragged.0, before: ix }, window, cx)
                        })
                    })
            })
            .collect::<Vec<_>>();
        let new_tab = h_flex()
            .h_full()
            .flex_grow(1.)
            .px_1()
            .drag_over::<DraggedTab>(|style, _, _, cx| marks_the_drop(style, cx))
            .when_some(on_move, |space, on_move| {
                space.on_drop(move |dragged: &DraggedTab, window, cx| {
                    on_move(&TabMove { from: dragged.0, before: past_the_last_tab }, window, cx)
                })
            })
            .child(
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
            );
        let bar = TabBar::new("repo-tabs")
            .children(tabs)
            .selected_index(self.active)
            .when_some(self.on_select, |bar, on_select| {
                bar.on_click(move |ix, window, cx| on_select(ix, window, cx))
            });
        right_after_the_last_tab(bar, new_tab)
    }
}

#[derive(Clone)]
struct DraggedTabPreview(TabLabel);

impl Render for DraggedTabPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .h_8()
            .min_w(px(MIN_TAB_WIDTH))
            .max_w(px(MAX_TAB_WIDTH))
            .font_family(theme.font_family.clone())
            .sm_text()
            .text_color(theme.tab_foreground)
            .bg(theme.tab_active)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .shadow_md()
            .opacity(DRAGGED_TAB_OPACITY)
            .children(prefix(&self.0, cx))
            .child(div().flex_1().px_3().ellipsis().child(self.0.name.clone()))
    }
}

fn prefix(label: &TabLabel, cx: &App) -> Option<AnyElement> {
    let icon = match label.state {
        TabState::Reading => nodding_logo().into_any_element(),
        TabState::Failed => Icon::new(IconName::TriangleAlert)
            .xsmall()
            .text_color(cx.theme().red)
            .into_any_element(),
        TabState::Ready => Icon::new(label.icon.clone()?)
            .xsmall()
            .text_color(cx.theme().muted_foreground)
            .into_any_element(),
    };
    Some(div().pl_3().child(icon).into_any_element())
}

fn marks_the_drop(style: StyleRefinement, cx: &App) -> StyleRefinement {
    style.border_l_2().border_r_0().border_color(cx.theme().drag_border)
}

fn right_after_the_last_tab(bar: TabBar, element: impl IntoElement) -> TabBar {
    let bar_draws_its_last_empty_space_only_beside_a_suffix = div();
    bar.last_empty_space(element)
        .suffix(bar_draws_its_last_empty_space_only_beside_a_suffix)
}
