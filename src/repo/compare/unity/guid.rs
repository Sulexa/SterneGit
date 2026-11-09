use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{UnityCompare, UnityEvent};
use crate::{
    ui::{Look as _, counted},
    unity::GuidChange,
};

pub(super) fn warning(change: GuidChange, cx: &mut Context<UnityCompare>) -> AnyElement {
    let theme = cx.theme();
    let count = change.users.len();
    let title = if count == 0 {
        format!("The GUID of {} changed", change.asset)
    } else {
        format!("The GUID changed: {} {}", counted(count, "file loses", "files lose"), change.asset)
    };
    let about = if count == 0 {
        "Unity finds assets by GUID, not by path. No file in the repository uses the old GUID, so nothing breaks."
    } else {
        "Unity finds assets by GUID, not by path. These files still use the old GUID, so Unity shows the asset as Missing in them:"
    };
    let users = change.users.iter().map(|user| {
        let (folder, name) = user.path.rsplit_once('/').unwrap_or(("", user.path.as_str()));
        let icon = if name.ends_with(".unity") {
            IconName::Map
        } else if name.ends_with(".prefab") {
            IconName::Box
        } else {
            IconName::FileBox
        };
        h_flex()
            .h(px(26.))
            .px_2()
            .gap_2()
            .rounded(px(4.))
            .bg(gpui_kit::white().opacity(0.03))
            .child(Icon::new(icon).xsmall().text_color(theme.muted_foreground))
            .child(div().flex_none().sm_text().child(name.to_owned()))
            .child(div().flex_none().xs_text().text_color(theme.muted_foreground).child(folder.to_owned()))
            .child(div().flex_1())
            .child(div().xs_text().ellipsis().child(user.places.join(" · ")))
    });
    let keep = change.clone();
    let point = change.clone();
    v_flex()
        .flex_none()
        .m_3()
        .p_3()
        .gap_1p5()
        .rounded(px(6.))
        .border_1()
        .border_color(theme.yellow.opacity(0.45))
        .bg(theme.yellow.opacity(0.07))
        .child(
            h_flex()
                .gap_2()
                .child(Icon::new(IconName::TriangleAlert).small().text_color(theme.yellow))
                .child(div().sm_text().font_weight(gpui_kit::FontWeight::MEDIUM).child(title)),
        )
        .child(div().xs_text().text_color(theme.muted_foreground).child(about))
        .when(count > 0, |this| this.child(v_flex().gap_0p5().children(users)))
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("keep-old-guid")
                        .small()
                        .primary()
                        .label("Keep the old GUID")
                        .tooltip("Write the old GUID back into the .meta file")
                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(UnityEvent::KeepOldGuid(keep.clone())))),
                )
                .when(count > 0, |this| {
                    this.child(
                        Button::new("point-to-new-guid")
                            .small()
                            .label(match change.users.as_slice() {
                                [only] => format!("Point {} to the new GUID", only.path.rsplit('/').next().unwrap_or(&only.path)),
                                _ => format!("Point the {} to the new GUID", counted(count, "file", "files")),
                            })
                            .tooltip("Replace the old GUID with the new one in those files")
                            .on_click(cx.listener(move |_, _, _, cx| cx.emit(UnityEvent::PointToNewGuid(point.clone())))),
                    )
                }),
        )
        .into_any_element()
}
