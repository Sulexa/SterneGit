use gpui_kit::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use sternegit_reader_protocol::Fix;

use super::repository::Repository;
use crate::ui::{Look as _, counted};

const WIDTH: f32 = 900.;
const LIST_WIDTH: f32 = 230.;
const HEIGHT: f32 = 360.;
const NOTE: &str = "Apply writes each file only if it hasn't changed since it was read; then the changes list reads them again.";

pub(crate) struct Preview {
    pub(crate) fix: Fix,
    pub(crate) patches: Vec<String>,
}

struct FixPreview {
    preview: Preview,
    picked: usize,
    repository: Entity<Repository>,
}

fn counts(patch: &str) -> (usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    for line in patch.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            added += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed += 1;
        }
    }
    (added, removed)
}

fn shown_lines(patch: &str) -> impl Iterator<Item = &str> {
    patch.lines().skip_while(|line| !line.starts_with("@@"))
}

impl Render for FixPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, border, active, green, red, mono) =
            (theme.muted_foreground, theme.border, theme.list_active, theme.green, theme.red, theme.mono_font_family.clone());
        let files = self.preview.fix.edits.iter().enumerate().map(|(ix, edit)| {
            let (folder, name) = edit.path.rsplit_once('/').unwrap_or(("", edit.path.as_str()));
            let (added, removed) = self.preview.patches.get(ix).map_or((0, 0), |patch| counts(patch));
            h_flex()
                .id(("fix-file", ix))
                .gap_2()
                .px_2()
                .py_1p5()
                .border_b_1()
                .border_color(border)
                .cursor_pointer()
                .when(ix == self.picked, |row| row.bg(active))
                .child(Icon::new(IconName::FilePen).small().text_color(muted))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(div().sm_text().ellipsis().child(name.to_owned()))
                        .child(div().xs_text().text_color(muted).ellipsis().child(folder.to_owned())),
                )
                .child(div().xs_text().text_color(green).child(format!("+{added}")))
                .child(div().xs_text().text_color(red).child(format!("−{removed}")))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.picked = ix;
                    cx.notify();
                }))
        });
        let patch = self.preview.patches.get(self.picked).cloned().unwrap_or_default();
        let lines = shown_lines(&patch)
            .map(|line| {
                let color = match line.as_bytes().first() {
                    Some(b'+') => Some(green),
                    Some(b'-') => Some(red),
                    Some(b'@') => Some(muted),
                    _ => None,
                };
                let row = div().whitespace_nowrap().child(SharedString::from(line.to_owned()));
                match color {
                    Some(color) => row.text_color(color),
                    None => row,
                }
            })
            .collect::<Vec<_>>();
        let edits = self.preview.fix.edits.clone();
        let repository = self.repository.clone();
        let count = edits.len();
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .h(px(HEIGHT))
                    .gap_3()
                    .child(
                        v_flex()
                            .id("fix-files")
                            .w(px(LIST_WIDTH))
                            .h_full()
                            .flex_none()
                            .overflow_y_scroll()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(border)
                            .children(files),
                    )
                    .child(
                        v_flex()
                            .id("fix-patch")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .p_2()
                            .overflow_scroll()
                            .rounded(px(6.))
                            .border_1()
                            .border_color(border)
                            .font_family(mono)
                            .xs_text()
                            .children(lines),
                    ),
            )
            .child(div().xs_text().text_color(muted).child(NOTE))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("fix-cancel").label("Cancel").on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("fix-apply")
                            .primary()
                            .icon(IconName::Check)
                            .disabled(count == 0)
                            .label(SharedString::from(format!("Apply to {}", counted(count, "file", "files"))))
                            .on_click(move |_, window, cx| {
                                let edits = edits.clone();
                                repository.update(cx, |repository, cx| repository.apply_edits(edits, cx));
                                window.close_dialog(cx);
                            }),
                    ),
            )
    }
}

pub(crate) fn show_fix(preview: Preview, repository: Entity<Repository>, reader: String, window: &mut Window, cx: &mut App) {
    let title = preview.fix.title.clone();
    let about = format!("{reader} proposes these edits. Nothing is written until you apply them.");
    let form = cx.new(|_| FixPreview {
        preview,
        picked: 0,
        repository,
    });
    window.open_dialog(cx, move |dialog, _, cx| {
        let muted = cx.theme().muted_foreground;
        dialog
            .w(px(WIDTH))
            .title(title.clone())
            .child(v_flex().gap_3().child(div().sm_text().text_color(muted).child(about.clone())).child(form.clone()))
    });
}

pub(crate) fn show_fix_failure(problem: String, window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, move |dialog, _, cx| {
        let red = cx.theme().red;
        dialog
            .w(px(520.))
            .title("Couldn't prepare the fix")
            .child(div().sm_text().text_color(red).child(problem.clone()))
    });
}

#[cfg(test)]
mod tests {
    use super::{counts, shown_lines};

    #[test]
    fn a_patch_counts_its_lines_without_the_file_headers() {
        let patch = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n";
        assert_eq!(counts(patch), (1, 1));
        assert_eq!(shown_lines(patch).collect::<Vec<_>>(), ["@@ -1 +1 @@", "-old", "+new"]);
    }
}
