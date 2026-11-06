use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div, px,
};

use super::{
    hosted::{Hosted, connect_origin, creating_accounts, remember_use},
    parts::{Note, field},
    sign_in::{SignInStart, open_sign_in},
};
use crate::{repo::Repository, ui::Look as _};

const WIDTH: f32 = 520.;
const ACCOUNT: &str = "Account";
const OWNER: &str = "Owner";
const NAME: &str = "Name";
const DESCRIPTION: &str = "Description";
const VISIBILITY: &str = "Visibility";
const PROTOCOL: &str = "Protocol";
const CANCEL: &str = "Cancel";
const PUBLISH: &str = "Publish";
const ADD_ACCOUNT: &str = "Add account…";

struct PublishForm {
    repository: Entity<Repository>,
    folder: PathBuf,
    shown: SharedString,
    branch: SharedString,
    name: Entity<InputState>,
    hosted: Entity<Hosted>,
    problem: Option<SharedString>,
    busy: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl PublishForm {
    fn publish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value().trim().to_owned();
        if name.is_empty() || self.busy.is_some() {
            return;
        }
        let Some(creation) = self.hosted.read(cx).creation(&name, cx) else {
            return;
        };
        let account = creation.account().clone();
        let owner = creation.owner().to_owned();
        let folder = self.folder.clone();
        let repository = self.repository.clone();
        self.problem = None;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let connected = folder.clone();
            let result = cx
                .background_spawn(async move {
                    let created = creation.create().await?;
                    connect_origin(&connected, &created).map_err(|error| SharedString::from(format!("{error:#}")))?;
                    Ok::<_, SharedString>(created.ssh_command.is_some())
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                match result {
                    Ok(over_ssh) => {
                        remember_use(&account, &owner, &folder, over_ssh, cx);
                        window.close_dialog(cx);
                        repository.update(cx, |repository, cx| repository.push_once_reread(cx));
                    }
                    Err(problem) => {
                        this.problem = Some(problem);
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }
}

impl Render for PublishForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let cancel = Button::new("publish-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx));
        if creating_accounts(cx).is_empty() {
            return v_flex()
                .gap_3()
                .child(div().sm_text().text_color(muted).child(format!(
                    "{} has no remote yet. Add an account to create it there; SterneGit then adds it as origin and pushes.",
                    self.shown
                )))
                .child(
                    h_flex().justify_end().gap_2().mt_2().child(cancel).child(
                        Button::new("publish-add-account")
                            .primary()
                            .icon(IconName::Plus)
                            .label(ADD_ACCOUNT)
                            .on_click(|_, window, cx| open_sign_in(SignInStart::Fresh, window, cx)),
                    ),
                );
        }
        let hosted = self.hosted.clone();
        let (account, owner, visibility, protocol) = hosted.update(cx, |hosted, cx| {
            (
                hosted.account_picker("publish-account", false, cx),
                hosted.owner_picker(cx),
                hosted.visibility_choice(cx),
                hosted.protocol_choice(cx),
            )
        });
        let description = hosted.read(cx).description_input();
        let busy = self.busy.is_some();
        let ready = !self.name.read(cx).value().trim().is_empty() && hosted.read(cx).account.is_some();
        v_flex()
            .gap_4()
            .child(div().sm_text().text_color(muted).child(format!(
                "{} has no remote yet. SterneGit creates it on your account, adds it as origin and pushes {} to it.",
                self.shown, self.branch
            )))
            .child(field(ACCOUNT, account, None, cx))
            .child(
                h_flex()
                    .items_start()
                    .gap_2()
                    .child(div().flex_none().w(px(180.)).child(field(OWNER, owner, None, cx)))
                    .child(div().pt_8().text_color(muted).child("/"))
                    .child(div().flex_1().min_w_0().child(field(
                        NAME,
                        Input::new(&self.name),
                        self.problem.clone().map(Note::Problem),
                        cx,
                    ))),
            )
            .child(field(DESCRIPTION, description, None, cx))
            .child(
                h_flex()
                    .items_start()
                    .gap_6()
                    .child(field(VISIBILITY, visibility, None, cx))
                    .child(field(PROTOCOL, protocol, None, cx)),
            )
            .child(
                h_flex().justify_end().gap_2().child(cancel).child(
                    Button::new("publish-confirm")
                        .primary()
                        .icon(IconName::Upload)
                        .label(PUBLISH)
                        .loading(busy)
                        .disabled(!ready || busy)
                        .on_click(cx.listener(|this, _, window, cx| this.publish(window, cx))),
                ),
            )
    }
}

pub(crate) fn open_publish(repository: Entity<Repository>, window: &mut Window, cx: &mut App) {
    let (shown, folder, branch) = {
        let read = repository.read(cx);
        let branch = read
            .history()
            .and_then(|history| history.branch.clone())
            .unwrap_or_else(|| "the current branch".into());
        (read.name(), read.path().to_path_buf(), branch)
    };
    let title = format!("Publish {shown}");
    let form = cx.new(|cx| {
        let name = cx.new(|cx| InputState::new(window, cx).default_value(shown.clone()));
        let hosted = cx.new(|cx| Hosted::new(window, cx));
        let subscriptions = vec![
            cx.subscribe(&name, |_, _, event, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify()
                }
            }),
            cx.observe(&hosted, |_, _, cx| cx.notify()),
            cx.observe_global::<crate::app::connectors::Connectors>(|_, cx| cx.notify()),
        ];
        PublishForm {
            repository,
            folder,
            shown,
            branch,
            name,
            hosted,
            problem: None,
            busy: None,
            _subscriptions: subscriptions,
        }
    });
    window.open_dialog(cx, move |dialog, _, _| {
        let published = form.clone();
        dialog.w(px(WIDTH)).title(title.clone()).child(form.clone()).on_ok(move |_, window, cx| {
            published.update(cx, |form, cx| form.publish(window, cx));
            false
        })
    });
}
