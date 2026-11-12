use gpui_kit::{
    AnyElement, App, Div, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::{DropdownMenu as _, PopupMenu},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use crate::{
    accounts::{Account, SignInMethod},
    ui::{Look as _, identicon},
};

pub(crate) const PAGE_WIDTH: f32 = 880.;
const ROW_MIN_HEIGHT: f32 = 56.;
const SLIM_ROW_HEIGHT: f32 = 40.;
const TERM_WIDTH: f32 = 120.;
const RADIUS: f32 = 8.;
const PICKER_MENU_MIN_WIDTH: f32 = 240.;

pub(crate) fn method_label(method: SignInMethod) -> &'static str {
    match method {
        SignInMethod::Device => "Signed in with the browser",
        SignInMethod::PastedToken => "Pasted token",
        SignInMethod::GitCredential => "Token from Git Credential Manager",
    }
}

pub(crate) fn account_avatar(account: &Account, size: f32) -> impl IntoElement {
    identicon(account.id.as_str(), px(size))
}

pub(crate) fn server_name(account: &Account) -> String {
    let host = account.server.host_str().unwrap_or_default();
    let path = account.server.path().trim_end_matches('/');
    match account.server.port() {
        Some(port) => format!("{host}:{port}{path}"),
        None => format!("{host}{path}"),
    }
}

pub(crate) fn shown_path(path: &std::path::Path) -> String {
    let shortened = ["LOCALAPPDATA", "APPDATA"].into_iter().find_map(|variable| {
        let base = std::path::PathBuf::from(std::env::var_os(variable)?);
        let rest = path.strip_prefix(&base).ok()?;
        Some(format!("%{variable}%{}{}", std::path::MAIN_SEPARATOR, rest.display()))
    });
    match shortened {
        Some(shortened) if cfg!(windows) => shortened,
        _ => path.display().to_string(),
    }
}

pub(crate) fn page(content: impl IntoElement) -> Div {
    v_flex()
        .w_full()
        .max_w(px(PAGE_WIDTH))
        .px_8()
        .pt_6()
        .pb_8()
        .gap_6()
        .child(content)
}

pub(crate) fn crumbs(
    back: &'static str,
    here: impl Into<SharedString>,
    on_back: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let foreground = cx.theme().foreground;
    let muted = cx.theme().muted_foreground;
    h_flex()
        .gap_1()
        .xs_text()
        .text_color(muted)
        .child(
            div()
                .id("crumb-back")
                .cursor_pointer()
                .hover(move |style| style.text_color(foreground))
                .child(back)
                .on_click(move |_, window, cx| on_back(window, cx)),
        )
        .child(Icon::new(IconName::ChevronRight).xsmall())
        .child(div().text_color(foreground).child(here.into()))
        .into_any_element()
}

pub(crate) struct Heading {
    pub(crate) title: SharedString,
    pub(crate) about: Option<SharedString>,
    pub(crate) subtitle: Option<AnyElement>,
    pub(crate) media: Option<AnyElement>,
    pub(crate) actions: Vec<AnyElement>,
}

impl Heading {
    pub(crate) fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            about: None,
            subtitle: None,
            media: None,
            actions: Vec::new(),
        }
    }

    pub(crate) fn about(mut self, about: impl Into<SharedString>) -> Self {
        self.about = Some(about.into());
        self
    }

    pub(crate) fn subtitle(mut self, subtitle: impl IntoElement) -> Self {
        self.subtitle = Some(subtitle.into_any_element());
        self
    }

    pub(crate) fn media(mut self, media: impl IntoElement) -> Self {
        self.media = Some(media.into_any_element());
        self
    }

    pub(crate) fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub(crate) fn render(self, cx: &App) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        h_flex()
            .items_start()
            .gap_4()
            .children(self.media)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(div().lg_text().font_semibold().child(self.title))
                    .children(self.subtitle)
                    .when_some(self.about, |this, about| {
                        this.child(div().max_w(px(640.)).sm_text().text_color(muted).child(about))
                    }),
            )
            .child(h_flex().flex_none().gap_2().children(self.actions))
    }
}

pub(crate) fn group(
    title: &'static str,
    about: Option<SharedString>,
    rows: Vec<AnyElement>,
    cx: &App,
) -> AnyElement {
    let muted = cx.theme().muted_foreground;
    v_flex()
        .gap_2()
        .child(
            v_flex()
                .child(div().sm_text().font_semibold().child(title))
                .when_some(about, |this, about| {
                    this.child(div().xs_text().text_color(muted).child(about))
                }),
        )
        .child(boxed(rows, cx))
        .into_any_element()
}

pub(crate) fn boxed(rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    let border = cx.theme().border;
    v_flex()
        .border_1()
        .border_color(border)
        .rounded(px(RADIUS))
        .overflow_hidden()
        .children(rows.into_iter().enumerate().map(move |(ix, row)| {
            div()
                .when(ix > 0, |this| this.border_t_1().border_color(border))
                .child(row)
        }))
        .into_any_element()
}

pub(crate) fn row() -> Div {
    h_flex().min_h(px(ROW_MIN_HEIGHT)).px(px(14.)).py(px(10.)).gap_3()
}

pub(crate) fn slim_row() -> Div {
    h_flex().min_h(px(SLIM_ROW_HEIGHT)).px(px(14.)).py_2().gap_3()
}

pub(crate) fn definition(term: &'static str, value: impl IntoElement, cx: &App) -> AnyElement {
    slim_row()
        .child(
            div()
                .flex_none()
                .w(px(TERM_WIDTH))
                .sm_text()
                .text_color(cx.theme().muted_foreground)
                .child(term),
        )
        .child(h_flex().flex_1().min_w_0().gap_1p5().flex_wrap().child(value))
        .into_any_element()
}

pub(crate) fn chip(label: impl Into<SharedString>, color: Option<Hsla>, cx: &App) -> AnyElement {
    let label: SharedString = label.into();
    let muted = cx.theme().muted_foreground;
    let border = cx.theme().border;
    h_flex()
        .flex_none()
        .h(px(20.))
        .px_1p5()
        .rounded(px(4.))
        .border_1()
        .xs_text()
        .whitespace_nowrap()
        .map(|this| match color {
            Some(color) => this
                .text_color(color)
                .border_color(color.opacity(0.35))
                .bg(color.opacity(0.1)),
            None => this.text_color(muted).border_color(border),
        })
        .child(label)
        .into_any_element()
}

pub(crate) fn mark(icon: IconName, large: bool, cx: &App) -> AnyElement {
    let size = if large { 48. } else { 32. };
    h_flex()
        .flex_none()
        .size(px(size))
        .justify_center()
        .rounded(px(if large { 8. } else { 6. }))
        .bg(cx.theme().secondary)
        .text_color(cx.theme().muted_foreground)
        .child(Icon::new(icon).map(|icon| if large { icon.large() } else { icon.small() }))
        .into_any_element()
}

pub(crate) fn problem_line(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    let text: SharedString = text.into();
    h_flex()
        .gap_1()
        .xs_text()
        .text_color(cx.theme().red)
        .child(Icon::new(IconName::TriangleAlert).xsmall())
        .child(div().ellipsis().child(text))
        .into_any_element()
}

pub(crate) fn callout(icon: IconName, text: impl Into<SharedString>, color: Hsla) -> AnyElement {
    let text: SharedString = text.into();
    h_flex()
        .items_start()
        .gap_3()
        .px(px(14.))
        .py_3()
        .border_1()
        .border_color(color.opacity(0.35))
        .bg(color.opacity(0.07))
        .rounded(px(RADIUS))
        .child(div().mt_0p5().child(Icon::new(icon).small().text_color(color)))
        .child(div().flex_1().min_w_0().sm_text().child(text))
        .into_any_element()
}

pub(crate) enum Note {
    Plain(SharedString),
    Problem(SharedString),
}

pub(crate) fn field(label: impl Into<SharedString>, control: impl IntoElement, note: Option<Note>, cx: &App) -> Div {
    let label: SharedString = label.into();
    let theme = cx.theme();
    let (muted, red) = (theme.muted_foreground, theme.red);
    v_flex()
        .min_w_0()
        .gap_1p5()
        .child(div().sm_text().font_medium().child(label))
        .child(control)
        .when_some(note, |this, note| {
            let (text, color) = match note {
                Note::Plain(text) => (text, muted),
                Note::Problem(text) => (text, red),
            };
            this.child(div().min_w_0().xs_text().text_color(color).child(text))
        })
}

pub(crate) struct Picker {
    id: ElementId,
    value: SharedString,
    lead: Option<AnyElement>,
    disabled: bool,
}

impl Picker {
    pub(crate) fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            lead: None,
            disabled: false,
        }
    }

    pub(crate) fn lead(mut self, lead: impl IntoElement) -> Self {
        self.lead = Some(lead.into_any_element());
        self
    }

    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(crate) fn menu(
        self,
        build: impl Fn(PopupMenu, &mut Window, &mut gpui_kit::Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> impl IntoElement {
        Button::new(self.id)
            .outline()
            .w_full()
            .disabled(self.disabled)
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .children(self.lead)
                    .child(div().flex_1().ellipsis().text_left().child(self.value))
                    .child(Icon::new(IconName::ChevronDown).xsmall()),
            )
            .dropdown_menu(move |menu, window, cx| build(menu.min_w(px(PICKER_MENU_MIN_WIDTH)), window, cx))
    }
}

pub(crate) fn toggle_pair(
    id: &'static str,
    labels: [&'static str; 2],
    chosen: usize,
    enabled: [bool; 2],
    tooltips: [Option<&'static str>; 2],
    on_choose: impl Fn(usize, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let on_choose = std::rc::Rc::new(on_choose);
    h_flex()
        .gap_0p5()
        .p(px(3.))
        .border_1()
        .border_color(cx.theme().border)
        .rounded(px(6.))
        .children((0..2).map(|ix| {
            let on_choose = on_choose.clone();
            Button::new((id, ix))
                .ghost()
                .small()
                .label(labels[ix])
                .selected(ix == chosen)
                .disabled(!enabled[ix])
                .when_some(tooltips[ix], |button, tooltip| button.tooltip(tooltip))
                .on_click(move |_, window, cx| on_choose(ix, window, cx))
        }))
        .into_any_element()
}

pub(crate) fn hot_row(id: impl Into<ElementId>, cx: &App) -> gpui_kit::Stateful<Div> {
    let hover = cx.theme().list_hover;
    row().id(id).hover(move |style| style.bg(hover))
}

#[cfg(test)]
mod tests {
    use super::server_name;
    use crate::accounts::fixtures;

    #[test]
    fn an_account_is_named_by_its_server_with_its_port_and_path() {
        let named = |server: &str| server_name(&fixtures::account(server, "1", &[]));
        assert_eq!(named("https://github.com"), "github.com");
        assert_eq!(named("https://dev.azure.com/contoso/"), "dev.azure.com/contoso");
        assert_eq!(named("http://devops.example.com:8080/tfs/DefaultCollection"), "devops.example.com:8080/tfs/DefaultCollection");
    }
}
