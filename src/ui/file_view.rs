use gpui_kit::{
    App, Global, IntoElement, ParentElement as _, RenderOnce, Styled as _, Window,
    assets::IconName,
    component::{
        Selectable as _, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum FileView {
    #[default]
    List,
    Tree,
}

impl Global for FileView {}

impl FileView {
    const BOTH: [FileView; 2] = [FileView::List, FileView::Tree];

    pub(crate) fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    fn choose(self, window: &mut Window, cx: &mut App) {
        cx.set_global(self);
        window.refresh();
    }

    fn button_id(self) -> &'static str {
        match self {
            FileView::List => "files-as-list",
            FileView::Tree => "files-as-tree",
        }
    }

    fn icon(self) -> IconName {
        match self {
            FileView::List => IconName::List,
            FileView::Tree => IconName::ListTree,
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            FileView::List => "Show files as a list",
            FileView::Tree => "Show files as a tree",
        }
    }
}

#[derive(IntoElement)]
pub(crate) struct FileViewToggle;

impl RenderOnce for FileViewToggle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let current = FileView::current(cx);
        h_flex()
            .flex_none()
            .gap_0p5()
            .children(FileView::BOTH.map(|view| {
                Button::new(view.button_id())
                    .ghost()
                    .xsmall()
                    .icon(view.icon())
                    .selected(view == current)
                    .tooltip(view.tooltip())
                    .on_click(move |_, window, cx| view.choose(window, cx))
            }))
    }
}
