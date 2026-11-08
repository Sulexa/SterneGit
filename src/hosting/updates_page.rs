use std::time::SystemTime;

use gpui_kit::{
    AnyElement, App, Context, Hsla, IntoElement, ParentElement as _, SharedString, Styled as _,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        spinner::Spinner,
        switch::Switch,
        v_flex,
    },
    div,
};

use super::{
    parts::{Heading, boxed, page, row, shown_path},
    settings_tab::SettingsTab,
};
use crate::{
    app::{
        top_bar::{update_variant, whats_new},
        updates::{
            OPEN_RELEASE_PAGE, RESTART_TO_UPDATE, Status, UPDATE, Updates, check, checked_ago, download, downloading,
            restart_to_update, set_automatic,
        },
    },
    update::{Installation, Version},
    ui::Look as _,
};

const UPDATES: &str = "Updates";
const UPDATES_ABOUT: &str = "SterneGit looks for a new release on GitHub when it starts and every six hours.";
const BUILT_HERE: &str = "Built on this computer from source";
const CHECK_NOW: &str = "Check for updates";
const LOOKING: &str = "Looking for a new release…";
const NOT_CHECKED: &str = "Not checked yet";
const BUILT_STATUS: &str = "This copy never updates itself. Install SterneGit from a release to get updates.";
const INSTALLING: &str = "Installing…";
const AUTOMATIC: &str = "Update automatically";
const AUTOMATIC_ABOUT: &str = "Downloads each new release in the background and installs it when you restart or quit \
     SterneGit. When off, SterneGit only tells you a release is out.";
const NOT_SAVED: &str = "The setting couldn't be saved";

struct Line {
    icon: IconName,
    color: Hsla,
    text: String,
    spinning: bool,
}

impl SettingsTab {
    pub(super) fn updates_page(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let updates = Updates::of(cx);
        let built = updates.installation().feed().is_none();
        let muted = cx.theme().muted_foreground;
        let line = status_line(updates, cx);
        let version_row = row()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(div().sm_text().font_medium().child(format!("SterneGit {}", Version::running())))
                    .child(div().xs_text().text_color(muted).child(where_it_is(updates.installation())))
                    .child(
                        h_flex()
                            .mt_1()
                            .gap_1p5()
                            .xs_text()
                            .child(match line.spinning {
                                true => Spinner::new().xsmall().into_any_element(),
                                false => Icon::new(line.icon).xsmall().text_color(line.color).into_any_element(),
                            })
                            .child(div().min_w_0().child(line.text)),
                    ),
            )
            .child(h_flex().flex_none().gap_2().children(actions(updates.status(), built, cx)))
            .into_any_element();
        let automatic_row = row()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_0p5()
                    .child(div().sm_text().font_medium().child(AUTOMATIC))
                    .child(div().xs_text().text_color(muted).child(AUTOMATIC_ABOUT)),
            )
            .child(
                Switch::new("update-automatically")
                    .small()
                    .checked(updates.automatic() && !built)
                    .disabled(built)
                    .accessibility_label(AUTOMATIC)
                    .on_change(cx.listener(|this, automatic: &bool, _, cx| {
                        if set_automatic(*automatic, cx).is_err() {
                            this.tell(NOT_SAVED, cx);
                        }
                        cx.notify();
                    })),
            )
            .into_any_element();
        page(
            v_flex()
                .gap_6()
                .child(Heading::new(UPDATES).about(UPDATES_ABOUT).render(cx))
                .child(boxed(vec![version_row, automatic_row], cx)),
        )
        .into_any_element()
    }
}

pub(super) fn ready_version(cx: &App) -> Option<SharedString> {
    match Updates::of(cx).status() {
        Status::Ready { release, .. } => Some(release.version.to_string().into()),
        _ => None,
    }
}

fn where_it_is(installation: &Installation) -> String {
    let folder = |exe: &std::path::Path| exe.parent().map(shown_path).unwrap_or_default();
    match installation {
        Installation::BuiltHere => BUILT_HERE.to_owned(),
        Installation::Setup { exe, .. } => format!("Installed for you in {}", folder(exe)),
        Installation::Portable { exe, .. } => format!("Portable, in {}", folder(exe)),
    }
}

fn status_line(updates: &Updates, cx: &App) -> Line {
    let theme = cx.theme();
    let line = |icon: IconName, color: Hsla, text: String| Line { icon, color, text, spinning: false };
    let spinning = |text: String| Line { icon: IconName::Loader, color: theme.muted_foreground, text, spinning: true };
    if updates.installation().feed().is_none() {
        return line(IconName::Info, theme.muted_foreground, BUILT_STATUS.to_owned());
    }
    match updates.status() {
        Status::NotChecked => line(IconName::Info, theme.muted_foreground, NOT_CHECKED.to_owned()),
        Status::Checking => spinning(LOOKING.to_owned()),
        Status::UpToDate => {
            let ago = updates
                .checked_at()
                .and_then(|at| SystemTime::now().duration_since(at).ok())
                .map(checked_ago)
                .unwrap_or_default();
            line(IconName::CircleCheck, theme.green, format!("Up to date · checked {ago}"))
        }
        Status::Out(release) => line(IconName::Info, theme.blue, format!("SterneGit {} is out.", release.version)),
        Status::Downloading { release, .. } => spinning(downloading(release)),
        Status::Ready { release, .. } => line(
            IconName::CircleCheck,
            theme.green,
            format!("SterneGit {} is ready. It installs when you restart, or when you quit.", release.version),
        ),
        Status::ReadOnly(release) => line(
            IconName::TriangleAlert,
            theme.yellow,
            format!(
                "SterneGit {} is out, but this folder can't be written to, so download it from its release page.",
                release.version
            ),
        ),
        Status::Failed(error) => line(IconName::CircleAlert, theme.red, format!("Couldn't check: {error}")),
        Status::Installing => spinning(INSTALLING.to_owned()),
    }
}

fn actions(status: &Status, built: bool, cx: &App) -> Vec<AnyElement> {
    let check_now = |disabled: bool| {
        Button::new("check-for-updates")
            .small()
            .icon(IconName::RefreshCw)
            .label(CHECK_NOW)
            .disabled(disabled)
            .on_click(|_, _, cx| check(cx))
            .into_any_element()
    };
    if built {
        return vec![check_now(true)];
    }
    match status {
        Status::NotChecked | Status::UpToDate | Status::Failed(_) => vec![check_now(false)],
        Status::Checking | Status::Downloading { .. } | Status::Installing => vec![check_now(true)],
        Status::Ready { release, .. } => vec![
            whats_new("updates-whats-new", release).small().into_any_element(),
            Button::new("updates-restart")
                .custom(update_variant(cx))
                .small()
                .icon(IconName::RefreshCw)
                .label(RESTART_TO_UPDATE)
                .on_click(|_, _, cx| restart_to_update(cx))
                .into_any_element(),
        ],
        Status::Out(release) => vec![
            whats_new("updates-whats-new", release).small().into_any_element(),
            Button::new("updates-download")
                .custom(update_variant(cx))
                .small()
                .icon(IconName::Download)
                .label(UPDATE)
                .on_click(|_, _, cx| download(cx))
                .into_any_element(),
        ],
        Status::ReadOnly(release) => {
            let notes = release.notes.to_string();
            vec![
                Button::new("updates-release-page")
                    .small()
                    .icon(IconName::ExternalLink)
                    .label(OPEN_RELEASE_PAGE)
                    .on_click(move |_, _, cx| cx.open_url(&notes))
                    .into_any_element(),
            ]
        }
    }
}
