use std::path::PathBuf;

use fastgit_protocol::{Failure, NewSshKey};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Input, InputState},
        notification::Notification,
        v_flex,
    },
    div, px,
};

use super::{
    accounts_page::service_label,
    parts::{Note, field, shown_path},
    sign_in::{SignInStart, open_sign_in},
};
use crate::{
    accounts::{Account, AccountId, SignInMethod},
    app::connectors::{Connectors, change_account, note_failure},
    connect::{CallError, Forge},
    git::ssh_key::{KeyPair, generate, key_file_name, keys_folder, known_hosts_file, remember_host_keys},
    ui::Look as _,
};

const WIDTH: f32 = 520.;
const CANCEL: &str = "Cancel";
const CREATE_AND_ADD: &str = "Create and add";
const WITH_BROWSER: &str = "Sign in with the browser";
const SIGN_IN_AGAIN: &str = "Sign in again";
const DEFAULT_KEY_TYPE: &str = "ed25519";

type Done = Box<dyn Fn(&AccountId, &mut Window, &mut App)>;

struct SshKeyForm {
    account: Account,
    service: String,
    key_type: String,
    title: Entity<InputState>,
    problem: Option<SharedString>,
    forbidden: bool,
    busy: Option<Task<()>>,
    done: Done,
}

impl SshKeyForm {
    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let connectors = Connectors::of(cx);
        let (Some(forge), Some(folder), Some(known_hosts)) =
            (connectors.forge(&self.account.id, None), keys_folder(), known_hosts_file())
        else {
            self.problem = Some("FastGit can't reach this account's connector right now.".into());
            cx.notify();
            return;
        };
        let title = self.title.read(cx).value().trim().to_owned();
        if title.is_empty() {
            self.problem = Some("Give the key a title.".into());
            cx.notify();
            return;
        }
        let private = folder.join(key_file_name(self.account.id.as_str(), &self.key_type));
        let comment = format!("FastGit {}@{}", self.account.login, self.account.server.host_str().unwrap_or_default());
        let hosts = host_names(&self.account, cx);
        let key_type = self.key_type.clone();
        let work = create_and_add(forge, private, key_type, comment, title, hosts, known_hosts);
        let id = self.account.id.clone();
        self.problem = None;
        self.forbidden = false;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_spawn(work).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                match result {
                    Ok(private) => {
                        let path = private.display().to_string();
                        if let Err(error) = change_account(cx, &id, |account| account.ssh_key = Some(path)) {
                            this.problem = Some(format!("The key was added but couldn't be remembered: {error}").into());
                            cx.notify();
                            return;
                        }
                        window.close_dialog(cx);
                        window.push_notification(Notification::success(format!("Added an SSH key to {}", this.service)), cx);
                        (this.done)(&id, window, cx);
                    }
                    Err(Created::Refused(error)) => {
                        note_failure(cx, &id, &error);
                        this.forbidden = matches!(error, CallError::Failed(Failure::Forbidden { .. }));
                        this.problem = Some(match &error {
                            CallError::Failed(Failure::Forbidden { needs, .. }) => format!(
                                "{} refused: this account's token lacks {}.",
                                this.service,
                                needs.as_deref().unwrap_or("the permission to add keys")
                            ),
                            error => error.to_string(),
                        }
                        .into());
                        cx.notify();
                    }
                    Err(Created::Local(problem)) => {
                        this.problem = Some(problem.into());
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }
}

enum Created {
    Refused(CallError),
    Local(String),
}

async fn create_and_add(
    forge: Forge,
    private: PathBuf,
    key_type: String,
    comment: String,
    title: String,
    hosts: Vec<String>,
    known_hosts: PathBuf,
) -> Result<PathBuf, Created> {
    let pair = generate(&private, &key_type, &comment).map_err(|error| Created::Local(format!("{error:#}")))?;
    let added = forge
        .add_ssh_key(NewSshKey {
            title,
            key: pair.public.clone(),
        })
        .await;
    if let Err(error) = added {
        let _ = pair.delete();
        return Err(Created::Refused(error));
    }
    if let Ok(keys) = forge.host_keys().await {
        if !keys.is_empty() {
            let _ = remember_host_keys(&known_hosts, &hosts, &keys);
        }
    }
    Ok(pair.private)
}

pub(crate) fn host_names(account: &Account, cx: &App) -> Vec<String> {
    let mut hosts: Vec<String> = Connectors::of(cx)
        .service_of(account)
        .map(|choice| {
            choice
                .service()
                .remotes
                .iter()
                .flat_map(|remote| remote.hosts.iter())
                .filter(|host| !host.contains('{'))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if let Some(host) = account.server.host_str() {
        if !hosts.iter().any(|known| known.eq_ignore_ascii_case(host)) {
            hosts.push(host.to_owned());
        }
    }
    hosts
}

impl Render for SshKeyForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let folder = keys_folder().map_or_else(String::new, |folder| shown_path(&folder));
        let busy = self.busy.is_some();
        let offers_browser = Connectors::of(cx)
            .service_of(&self.account)
            .is_some_and(|choice| choice.service().sign_in.iter().any(|sign_in| sign_in.method == SignInMethod::Device));
        let id = self.account.id.clone();
        let primary = match (self.forbidden, offers_browser) {
            (true, browser) => Button::new("ssh-key-sign-in")
                .primary()
                .icon(IconName::LogIn)
                .label(if browser { WITH_BROWSER } else { SIGN_IN_AGAIN })
                .on_click(move |_, window, cx| {
                    window.close_dialog(cx);
                    let start = if browser { SignInStart::Browser(id.clone()) } else { SignInStart::Again(id.clone()) };
                    open_sign_in(start, window, cx);
                }),
            (false, _) => Button::new("ssh-key-create")
                .primary()
                .icon(IconName::KeyRound)
                .label(CREATE_AND_ADD)
                .loading(busy)
                .disabled(busy)
                .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
        };
        v_flex()
            .gap_3()
            .child(div().sm_text().text_color(muted).child(format!(
                "FastGit makes an {} key without a passphrase in {folder}, adds its public half to {}, \
                 and uses it only in the repositories you clone or publish with this account.",
                self.key_type, self.service
            )))
            .child(field(
                format!("Title on {}", self.service),
                Input::new(&self.title),
                self.problem.clone().map(Note::Problem),
                cx,
            ))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .mt_2()
                    .child(Button::new("ssh-key-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(primary),
            )
    }
}

pub(crate) fn open_ssh_key_dialog(
    id: AccountId,
    window: &mut Window,
    cx: &mut App,
    done: impl Fn(&AccountId, &mut Window, &mut App) + 'static,
) {
    let connectors = Connectors::of(cx);
    let Some(account) = connectors.account(&id) else {
        return;
    };
    let key_type = connectors
        .service_of(&account)
        .and_then(|choice| choice.manifest.ssh_key_type.clone())
        .unwrap_or_else(|| DEFAULT_KEY_TYPE.to_owned());
    let service = service_label(&account, cx);
    let computer = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "this computer".to_owned());
    let login = account.login.clone();
    let form = cx.new(|cx| SshKeyForm {
        title: cx.new(|cx| InputState::new(window, cx).default_value(format!("FastGit on {computer}"))),
        account,
        service,
        key_type,
        problem: None,
        forbidden: false,
        busy: None,
        done: Box::new(done),
    });
    let title = form.read(cx).title.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        let created = form.clone();
        dialog
            .w(px(WIDTH))
            .title(format!("Create an SSH key for {login}"))
            .child(form.clone())
            .on_ok(move |_, window, cx| {
                created.update(cx, |form, cx| {
                    if form.busy.is_none() && !form.forbidden {
                        form.create(window, cx)
                    }
                });
                false
            })
    });
    window.defer(cx, move |window, cx| title.update(cx, |state, cx| state.focus(window, cx)));
}

pub(crate) fn local_key(account: &Account) -> Option<KeyPair> {
    KeyPair::read(std::path::Path::new(account.ssh_key.as_deref()?))
}
