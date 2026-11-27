use gpui_kit::{
    App, AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div, px,
};

use crate::{
    app::readers::{Chosen, choose},
    ui::Look as _,
};

const WIDTH: f32 = 460.;
const AS_TEXT: &str = "Show as text";
const NOTE: &str = "Change it later in Settings → Readers.";

struct ReaderChoice {
    extension: String,
    candidates: Vec<Chosen>,
    picked: usize,
}

impl ReaderChoice {
    fn settle(&self, pick: Option<usize>, window: &mut Window, cx: &mut App) {
        let id = pick.and_then(|ix| self.candidates.get(ix)).map(|chosen| chosen.id.clone());
        let _ = choose(cx, &self.extension, id.as_ref());
        window.close_dialog(cx);
    }
}

impl Render for ReaderChoice {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, blue, border, active) = (theme.muted_foreground, theme.blue, theme.border, theme.list_active);
        let options = self.candidates.iter().enumerate().map(|(ix, chosen)| {
            let picked = ix == self.picked;
            let label = chosen.kind.label.clone();
            h_flex()
                .id(("reader-option", ix))
                .gap_3()
                .px_3()
                .py_2()
                .rounded(px(6.))
                .border_1()
                .border_color(border)
                .cursor_pointer()
                .when_picked(picked, active)
                .child(Icon::new(IconName::FileCode).text_color(muted))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(div().sm_text().font_semibold().child(chosen.name.clone()))
                        .child(div().xs_text().text_color(muted).child(label)),
                )
                .children(picked.then(|| Icon::new(IconName::Check).small().text_color(blue)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.picked = ix;
                    cx.notify();
                }))
        });
        let name = self.candidates.get(self.picked).map_or_else(String::new, |chosen| chosen.name.clone());
        v_flex()
            .gap_3()
            .child(
                div()
                    .sm_text()
                    .text_color(muted)
                    .child(format!(
                        "More than one reader knows {}. SterneGit remembers your choice for every repository.",
                        self.extension
                    )),
            )
            .child(v_flex().gap_2().children(options))
            .child(div().xs_text().text_color(muted).child(NOTE))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("reader-as-text").label(AS_TEXT).on_click(cx.listener(|this, _, window, cx| this.settle(None, window, cx))))
                    .child(
                        Button::new("reader-use")
                            .primary()
                            .label(SharedString::from(format!("Use {name}")))
                            .on_click(cx.listener(|this, _, window, cx| this.settle(Some(this.picked), window, cx))),
                    ),
            )
    }
}

trait PickedRow {
    fn when_picked(self, picked: bool, color: gpui_kit::Hsla) -> Self;
}

impl PickedRow for gpui_kit::Stateful<gpui_kit::Div> {
    fn when_picked(self, picked: bool, color: gpui_kit::Hsla) -> Self {
        if picked { self.bg(color) } else { self }
    }
}

pub(crate) fn ask_for_reader(extension: String, candidates: Vec<Chosen>, window: &mut Window, cx: &mut App) {
    let title = format!("Which reader opens {extension} files?");
    let form = cx.new(|_| ReaderChoice {
        extension,
        candidates,
        picked: 0,
    });
    window.open_dialog(cx, move |dialog, _, _| dialog.w(px(WIDTH)).title(title.clone()).child(form.clone()));
}
