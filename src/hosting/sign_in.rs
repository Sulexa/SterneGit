use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use gpui_kit::{
    AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Disableable as _, Icon, Sizable as _, StyledExt as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::{Input, InputEvent, InputState},
        menu::PopupMenuItem,
        notification::Notification,
        spinner::Spinner,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};
use url::Url;

use super::{
    failure::Trouble,
    parts::{Note, Picker, field},
};
use crate::{
    accounts::{
        Account, AccountId, DeviceCode, DeviceFlow, Polled, SignInMethod, Token, token_git_holds,
    },
    app::connectors::{Connectors, ServiceChoice, keep_account},
    connect::{CallError, Draft, FieldKind, draft, identify},
    net::services::{ConnectorId, cancel_pair},
    ui::Look as _,
};

const WIDTH: f32 = 520.;
const ADD_TITLE: &str = "Add an account";
const SERVICE: &str = "Service";
const CANCEL: &str = "Cancel";
const SIGN_IN: &str = "Sign in";
const TOKEN: &str = "Token";
const USE_TOKEN: &str = "Use this token";
const COPY: &str = "Copy";
const OPEN_AGAIN: &str = "Open the page again";
const NEW_CODE: &str = "Get a new code";
const TRY_AGAIN: &str = "Try again";
const CHECKING: &str = "Checking…";
const NOT_A_TOKEN: &str = "That doesn't look like a token: paste it whole, without spaces.";
const CREDENTIAL_TITLE: &str = "Use Git Credential Manager's token?";
const KEYS_SCOPE: &str = "write:public_key";
const ALL_KEYS_SCOPE: &str = "admin:public_key";
const NO_SERVICES: &str = "No connector is ready yet. FastGit starts them a moment after it opens.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SignInStart {
    Fresh,
    Again(AccountId),
    Browser(AccountId),
}

enum Step {
    Choose,
    Token { input: Entity<InputState> },
    Credential,
    Device(Device),
}

enum Device {
    Asking,
    Waiting(DeviceCode),
    Expired(DeviceCode),
    Denied(DeviceCode),
}

pub(crate) struct SignIn {
    services: Vec<ServiceChoice>,
    chosen: usize,
    server: Entity<InputState>,
    step: Step,
    problem: Option<SharedString>,
    busy: Option<Task<()>>,
    _typing: Subscription,
}

impl SignIn {
    fn new(start: SignInStart, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let connectors = Connectors::of(cx);
        let services = connectors.services();
        let account = match &start {
            SignInStart::Fresh => None,
            SignInStart::Again(id) | SignInStart::Browser(id) => connectors.account(id),
        };
        let chosen = account
            .as_ref()
            .and_then(|account| connectors.service_of(account))
            .and_then(|choice| services.iter().position(|known| known.is(&choice)))
            .unwrap_or(0);
        let typed_server = account
            .as_ref()
            .map(|account| account.server.as_str().trim_end_matches('/').to_owned())
            .unwrap_or_default();
        let server = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("https://github.example.com")
                .default_value(typed_server)
        });
        let typing = cx.subscribe(&server, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let mut this = Self {
            services,
            chosen,
            server,
            step: Step::Choose,
            problem: None,
            busy: None,
            _typing: typing,
        };
        match (&start, account) {
            (SignInStart::Browser(_), Some(_)) => this.choose(SignInMethod::Device, window, cx),
            (SignInStart::Again(_), Some(account)) => this.choose(account.sign_in, window, cx),
            _ => {}
        }
        this
    }

    fn service(&self) -> Option<&ServiceChoice> {
        self.services.get(self.chosen)
    }

    fn label(&self) -> String {
        self.service()
            .map_or_else(String::new, |choice| choice.service().label.clone())
    }

    fn offers(&self, method: SignInMethod) -> bool {
        self.service()
            .is_some_and(|choice| choice.service().sign_in.iter().any(|sign_in| sign_in.method == method))
    }

    fn title(&self) -> String {
        match &self.step {
            Step::Choose => ADD_TITLE.to_owned(),
            Step::Token { .. } => format!("Sign in to {} with a token", self.label()),
            Step::Credential => CREDENTIAL_TITLE.to_owned(),
            Step::Device(_) => format!("Sign in to {}", self.label()),
        }
    }

    fn typed_fields(&self, cx: &App) -> BTreeMap<String, String> {
        let Some(choice) = self.service() else {
            return BTreeMap::new();
        };
        choice
            .service()
            .fields
            .iter()
            .filter(|field| field.kind == FieldKind::Url)
            .map(|field| (field.id.clone(), self.server.read(cx).value().to_string()))
            .collect()
    }

    fn drafted(&self, method: SignInMethod, cx: &App) -> Result<Draft, String> {
        let choice = self.service().ok_or_else(|| NO_SERVICES.to_owned())?;
        draft(choice.connector(), choice.service(), method, &self.typed_fields(cx))
    }

    fn needs_server(&self) -> bool {
        self.service()
            .is_some_and(|choice| choice.service().fields.iter().any(|field| field.kind == FieldKind::Url))
    }

    fn choose(&mut self, method: SignInMethod, window: &mut Window, cx: &mut Context<Self>) {
        self.problem = None;
        if let Err(problem) = self.drafted(method, cx) {
            self.problem = Some(problem.into());
            self.step = Step::Choose;
            cx.notify();
            return;
        }
        self.step = match method {
            SignInMethod::PastedToken => {
                let input = cx.new(|cx| InputState::new(window, cx).masked(true).placeholder("ghp_…"));
                input.update(cx, |state, cx| state.focus(window, cx));
                Step::Token { input }
            }
            SignInMethod::GitCredential => Step::Credential,
            SignInMethod::Device => {
                self.ask_for_code(window, cx);
                Step::Device(Device::Asking)
            }
        };
        cx.notify();
    }

    fn check(&mut self, method: SignInMethod, token: Token, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(drafted) = self.drafted(method, cx) else {
            return;
        };
        let connectors = Connectors::of(cx);
        let connector = ConnectorId(drafted.account.connector.clone());
        let (Some(started), Some(gateway)) = (connectors.started(&connector), connectors.gateway()) else {
            self.problem = Some(NO_SERVICES.into());
            cx.notify();
            return;
        };
        let label = self.label();
        let kept = token.clone();
        let checked = identify(started, gateway, drafted.account, token);
        self.problem = None;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_spawn(checked).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                match result {
                    Ok(account) => this.signed_in(account, &kept, &label, window, cx),
                    Err(error) => {
                        this.problem = Some(refusal(&error, &label));
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }

    fn signed_in(&mut self, account: Account, token: &Token, label: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = keep_account(cx, account.clone(), token) {
            self.problem = Some(format!("Couldn't keep the account: {error}").into());
            cx.notify();
            return;
        }
        window.close_dialog(cx);
        let lacks_keys = !account.scopes.is_empty()
            && !account.scopes.iter().any(|scope| scope == KEYS_SCOPE || scope == ALL_KEYS_SCOPE);
        let signed = format!("Signed in as {} on {label}", account.login);
        let notification = match lacks_keys {
            true => Notification::warning(format!(
                "This token lacks {KEYS_SCOPE}, so FastGit can't add SSH keys with it. Signing in works anyway."
            ))
            .title(signed),
            false => Notification::success(signed),
        };
        window.push_notification(notification, cx);
    }

    fn focused_input(&self) -> Option<Entity<InputState>> {
        match &self.step {
            Step::Token { input } => Some(input.clone()),
            Step::Choose if self.needs_server() => Some(self.server.clone()),
            _ => None,
        }
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy.is_some() {
            return;
        }
        match &self.step {
            Step::Token { .. } => self.submit_token(window, cx),
            Step::Credential => self.borrow_from_git(window, cx),
            Step::Choose | Step::Device(_) => {}
        }
    }

    fn submit_token(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Step::Token { input } = &self.step else {
            return;
        };
        match Token::new(&input.read(cx).value()) {
            Some(token) => self.check(SignInMethod::PastedToken, token, window, cx),
            None => {
                self.problem = Some(NOT_A_TOKEN.into());
                cx.notify();
            }
        }
    }

    fn borrow_from_git(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(drafted) = self.drafted(SignInMethod::GitCredential, cx) else {
            return;
        };
        let server = drafted.account.server.clone();
        self.problem = None;
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let found = cx.background_spawn(async move { token_git_holds(&server).map(|token| (token, server)) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.busy = None;
                match found {
                    Some((token, _)) => this.check(SignInMethod::GitCredential, token, window, cx),
                    None => {
                        let host = drafted.account.server.host_str().unwrap_or_default().to_owned();
                        this.problem = Some(format!("Git Credential Manager has no token for {host}.").into());
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
    }

    fn ask_for_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(drafted) = self.drafted(SignInMethod::Device, cx) else {
            return;
        };
        let Some(flow) = device_flow(&drafted, cx) else {
            self.problem = Some("This service has no browser sign-in configured.".into());
            return;
        };
        self.problem = None;
        self.step = Step::Device(Device::Asking);
        self.busy = Some(cx.spawn_in(window, async move |this, cx| {
            let (_keep, cancel) = cancel_pair();
            let asked = cx.background_spawn(flow.ask_for_code(cancel)).await;
            let Ok(code) = asked.map_err(|problem| {
                let _ = this.update(cx, |this, cx| {
                    this.problem = Some(problem.into());
                    cx.notify();
                });
            }) else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                cx.open_url(code.verification_uri.as_str());
                this.step = Step::Device(Device::Waiting(code.clone()));
                cx.notify();
            });
            let deadline = Instant::now() + code.lifetime;
            let mut interval = code.interval;
            loop {
                cx.background_executor().timer(interval).await;
                if Instant::now() >= deadline {
                    let _ = this.update(cx, |this, cx| this.end_wait(Device::Expired(code.clone()), cx));
                    return;
                }
                let (_keep, cancel) = cancel_pair();
                let polled = cx.background_spawn(flow.poll(&code, cancel)).await;
                interval = polled.next_interval(interval);
                if polled.keeps_waiting() {
                    continue;
                }
                match polled {
                    Polled::Waiting | Polled::SlowDown => continue,
                    Polled::Granted(token) => {
                        let _ = this.update_in(cx, |this, window, cx| {
                            this.check(SignInMethod::Device, token, window, cx)
                        });
                        return;
                    }
                    Polled::Expired => {
                        let _ = this.update(cx, |this, cx| this.end_wait(Device::Expired(code.clone()), cx));
                        return;
                    }
                    Polled::Denied => {
                        let _ = this.update(cx, |this, cx| this.end_wait(Device::Denied(code.clone()), cx));
                        return;
                    }
                    Polled::Failed(problem) => {
                        let _ = this.update(cx, |this, cx| {
                            this.problem = Some(problem.into());
                            cx.notify();
                        });
                        interval += Duration::from_secs(5);
                    }
                }
            }
        }));
    }

    fn end_wait(&mut self, ended: Device, cx: &mut Context<Self>) {
        self.busy = None;
        self.step = Step::Device(ended);
        cx.notify();
    }


    fn choose_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, border, hover) = (theme.muted_foreground, theme.border, theme.list_hover);
        if self.services.is_empty() {
            return div().sm_text().text_color(muted).child(NO_SERVICES).into_any_element();
        }
        let label = self.label();
        let shown = self.service().map(service_name).unwrap_or_default();
        let services = self.services.clone();
        let this = cx.entity().downgrade();
        let picker = Picker::new("sign-in-service", shown)
            .lead(Icon::new(IconName::Plug).small().text_color(muted))
            .menu(move |menu, _, _| {
                services.iter().enumerate().fold(menu, |menu, (ix, choice)| {
                    let this = this.clone();
                    menu.item(PopupMenuItem::new(service_name(choice)).on_click(move |_, _, cx| {
                        let _ = this.update(cx, |this, cx| {
                            this.chosen = ix;
                            this.problem = None;
                            cx.notify();
                        });
                    }))
                })
            });
        let host = self
            .drafted(SignInMethod::GitCredential, cx)
            .ok()
            .and_then(|drafted| drafted.account.server.host_str().map(str::to_owned))
            .unwrap_or_else(|| "this server".to_owned());
        let options = [
            (SignInMethod::Device, IconName::LogIn, "Sign in with the browser".to_owned(), format!("{label} shows a short code to confirm. Recommended.")),
            (SignInMethod::PastedToken, IconName::KeyRound, "Paste a token".to_owned(), format!("Make a token on {label} with the scopes FastGit needs.")),
            (SignInMethod::GitCredential, IconName::ShieldCheck, "Use Git Credential Manager's token".to_owned(), format!("Reuse the token Git already has for {host}.")),
        ];
        let rows = options
            .into_iter()
            .filter(|(method, ..)| self.offers(*method))
            .map(|(method, icon, title, about)| {
                h_flex()
                    .id(SharedString::from(title.clone()))
                    .gap_3()
                    .px_3()
                    .py_2p5()
                    .border_1()
                    .border_color(border)
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .child(Icon::new(icon).small().text_color(muted))
                    .child(
                        v_flex()
                            .flex_1()
                            .child(div().sm_text().font_semibold().child(title))
                            .child(div().xs_text().text_color(muted).child(about)),
                    )
                    .child(Icon::new(IconName::ChevronRight).small().text_color(muted))
                    .on_click(cx.listener(move |this, _, window, cx| this.choose(method, window, cx)))
            })
            .collect::<Vec<_>>();
        v_flex()
            .gap_4()
            .child(field(SERVICE, picker, None, cx))
            .when(self.needs_server(), |this| {
                this.child(field(
                    "Server address",
                    Input::new(&self.server),
                    Some(Note::Plain("The address you open in the browser".into())),
                    cx,
                ))
            })
            .child(v_flex().gap_2().children(rows))
            .into_any_element()
    }

    fn token_view(&self, input: &Entity<InputState>, cx: &mut Context<Self>) -> AnyElement {
        let label = self.label();
        let create = self
            .drafted(SignInMethod::PastedToken, cx)
            .ok()
            .and_then(|drafted| drafted.url("create_url").cloned());
        let link = create.map(|url| {
            Button::new("create-token")
                .ghost()
                .small()
                .icon(IconName::ExternalLink)
                .label(format!("Create a token on {label}"))
                .on_click(move |_, _, cx| cx.open_url(url.as_str()))
        });
        let note = self.problem.clone().map(Note::Problem);
        let label_row = h_flex()
            .justify_between()
            .child(div().sm_text().font_medium().child(TOKEN))
            .children(link);
        v_flex()
            .gap_1p5()
            .child(label_row)
            .child(Input::new(input))
            .children(note.map(|note| match note {
                Note::Problem(text) | Note::Plain(text) => div().xs_text().text_color(cx.theme().red).child(text),
            }))
            .into_any_element()
    }

    fn device_view(&self, device: &Device, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let (muted, input, yellow, red) = (theme.muted_foreground, theme.input, theme.yellow, theme.red);
        let mono = theme.mono_font_family.clone();
        let label = self.label();
        let code = match device {
            Device::Asking => None,
            Device::Waiting(code) | Device::Expired(code) | Device::Denied(code) => Some(code),
        };
        let stale = matches!(device, Device::Expired(_) | Device::Denied(_));
        let shown_code = code.map(|code| {
            let user_code = code.user_code.clone();
            h_flex()
                .justify_center()
                .gap_4()
                .mt_3()
                .p(px(18.))
                .border_1()
                .border_dashed()
                .border_color(input)
                .rounded(px(8.))
                .child(
                    div()
                        .font_family(mono)
                        .text_size(px(28.))
                        .line_height(px(36.))
                        .font_semibold()
                        .when(stale, |this| this.text_color(muted).line_through())
                        .child(code.user_code.clone()),
                )
                .when(!stale, |this| {
                    this.child(
                        Button::new("copy-device-code")
                            .small()
                            .icon(IconName::Copy)
                            .label(COPY)
                            .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(user_code.clone()))),
                    )
                })
        });
        let status = match device {
            Device::Asking => h_flex().gap_2().child(Spinner::new().small()).child(div().text_color(muted).child(format!("Asking {label} for a code…"))),
            Device::Waiting(_) => h_flex().gap_2().child(Spinner::new().small()).child(div().text_color(muted).child(format!("Waiting for you to approve FastGit on {label}…"))),
            Device::Expired(_) => h_flex().gap_2().child(Icon::new(IconName::Clock).small().text_color(yellow)).child("The code expired. Nothing was saved."),
            Device::Denied(_) => h_flex().gap_2().child(Icon::new(IconName::CircleX).small().text_color(red)).child(format!("{label} says you declined. Nothing was saved.")),
        };
        let page = code.map(|code| {
            let page = code.verification_uri.as_str().trim_start_matches("https://").trim_end_matches('/').to_owned();
            format!("FastGit opened {page} in your browser. Enter this code there:")
        });
        v_flex()
            .children(page.map(|page| div().sm_text().text_color(muted).child(page)))
            .children(shown_code)
            .child(status.mt_3().sm_text())
            .into_any_element()
    }

    fn buttons(&self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy.is_some();
        let cancel = Button::new("sign-in-cancel")
            .label(CANCEL)
            .on_click(|_, window, cx| window.close_dialog(cx));
        let row = h_flex().justify_end().gap_2().mt_4();
        match &self.step {
            Step::Choose => row.child(cancel),
            Step::Token { .. } => row.child(cancel).child(
                Button::new("sign-in-token")
                    .primary()
                    .label(if busy { CHECKING } else { SIGN_IN })
                    .loading(busy)
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, window, cx| this.submit_token(window, cx))),
            ),
            Step::Credential => row.child(cancel).child(
                Button::new("sign-in-credential")
                    .primary()
                    .label(if busy { CHECKING } else { USE_TOKEN })
                    .loading(busy)
                    .disabled(busy)
                    .on_click(cx.listener(|this, _, window, cx| this.borrow_from_git(window, cx))),
            ),
            Step::Device(Device::Asking) => row.child(cancel),
            Step::Device(Device::Waiting(code)) => {
                let page = code.verification_uri.clone();
                row.child(
                    Button::new("open-device-page")
                        .ghost()
                        .icon(IconName::ExternalLink)
                        .label(OPEN_AGAIN)
                        .on_click(move |_, _, cx| cx.open_url(page.as_str())),
                )
                .child(cancel)
            }
            Step::Device(ended) => row.child(cancel).child(
                Button::new("device-again")
                    .primary()
                    .label(if matches!(ended, Device::Expired(_)) { NEW_CODE } else { TRY_AGAIN })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.ask_for_code(window, cx);
                        cx.notify();
                    })),
            ),
        }
        .into_any_element()
    }
}

impl Render for SignIn {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let description = match &self.step {
            Step::Token { .. } => Some(token_about(self, cx)),
            Step::Credential => {
                let host = self
                    .drafted(SignInMethod::GitCredential, cx)
                    .ok()
                    .and_then(|drafted| drafted.account.server.host_str().map(str::to_owned))
                    .unwrap_or_default();
                Some(format!(
                    "Git Credential Manager may have a token for {host}. FastGit copies it into its own entry and signs in with it; \
                     Git Credential Manager keeps its copy. These tokens usually can't add SSH keys{}.",
                    if self.offers(SignInMethod::Device) { ": sign in with the browser for that" } else { "" }
                ))
            }
            Step::Choose | Step::Device(_) => None,
        };
        let body = match &self.step {
            Step::Choose => self.choose_view(cx),
            Step::Token { input } => {
                let input = input.clone();
                self.token_view(&input, cx)
            }
            Step::Credential => div().into_any_element(),
            Step::Device(device) => {
                let device = match device {
                    Device::Asking => Device::Asking,
                    Device::Waiting(code) => Device::Waiting(code.clone()),
                    Device::Expired(code) => Device::Expired(code.clone()),
                    Device::Denied(code) => Device::Denied(code.clone()),
                };
                self.device_view(&device, cx)
            }
        };
        let problem = match &self.step {
            Step::Token { .. } => None,
            _ => self.problem.clone(),
        };
        v_flex()
            .gap_3()
            .children(description.map(|text| div().sm_text().text_color(muted).child(text)))
            .child(body)
            .children(problem.map(|problem| div().xs_text().text_color(cx.theme().red).child(problem)))
            .child(self.buttons(cx))
    }
}

fn service_name(choice: &ServiceChoice) -> String {
    let service = choice.service();
    match service.fields.is_empty() {
        true => {
            let host = service
                .remotes
                .iter()
                .flat_map(|remote| &remote.hosts)
                .find(|host| !host.contains('{'))
                .cloned()
                .unwrap_or_default();
            format!("{} — {host}", service.label)
        }
        false => service.label.clone(),
    }
}

fn token_about(sign_in: &SignIn, cx: &App) -> String {
    let scopes = sign_in
        .drafted(SignInMethod::PastedToken, cx)
        .ok()
        .and_then(|drafted| drafted.url("create_url").cloned())
        .and_then(|url| scopes_asked(&url));
    match scopes {
        Some(scopes) => format!(
            "Create a classic token with the scopes {scopes}, then paste it here. FastGit checks it before saving it."
        ),
        None => "Create a token, then paste it here. FastGit checks it before saving it.".to_owned(),
    }
}

fn scopes_asked(url: &Url) -> Option<String> {
    let (_, scopes) = url.query_pairs().find(|(name, _)| name == "scopes")?;
    let scopes: Vec<&str> = scopes.split(',').map(str::trim).filter(|scope| !scope.is_empty()).collect();
    match scopes.as_slice() {
        [] => None,
        [only] => Some((*only).to_owned()),
        [first @ .., last] => Some(format!("{} and {last}", first.join(", "))),
    }
}

fn device_flow(drafted: &Draft, cx: &App) -> Option<DeviceFlow> {
    Some(DeviceFlow::new(
        Connectors::of(cx).transport()?,
        drafted.client_id.clone()?,
        drafted.scopes.clone(),
        drafted.url("device_code_url")?.clone(),
        drafted.url("token_url")?.clone(),
    ))
}

fn refusal(error: &CallError, label: &str) -> SharedString {
    match error {
        CallError::Failed(fastgit_protocol::Failure::SignedOut) => {
            format!("{label} refused this token: it may have expired or been revoked.").into()
        }
        error => Trouble::described(error, "this token", label, "the server", Vec::new()).message,
    }
}

pub(crate) fn open_sign_in(start: SignInStart, window: &mut Window, cx: &mut App) {
    let form = cx.new(|cx| SignIn::new(start, window, cx));
    let focused = form.read(cx).focused_input();
    window.open_dialog(cx, move |dialog, _, cx| {
        let title = form.read(cx).title();
        let submitted = form.clone();
        dialog.w(px(WIDTH)).title(title).child(form.clone()).on_ok(move |_, window, cx| {
            submitted.update(cx, |form, cx| form.submit(window, cx));
            false
        })
    });
    if let Some(input) = focused {
        window.defer(cx, move |window, cx| input.update(cx, |state, cx| state.focus(window, cx)));
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::scopes_asked;

    #[test]
    fn the_scopes_a_token_needs_are_read_from_its_creation_page() {
        let url = Url::parse("https://github.com/settings/tokens/new?scopes=repo,read:org,write:public_key,workflow&description=FastGit").unwrap();
        assert_eq!(scopes_asked(&url).as_deref(), Some("repo, read:org, write:public_key and workflow"));
        assert_eq!(scopes_asked(&Url::parse("https://ghe.example/settings/tokens/new").unwrap()), None);
    }
}
