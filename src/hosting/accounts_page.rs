use gpui_kit::{
    AnyElement, App, ClipboardItem, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        spinner::Spinner,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px,
};

use super::{
    failure::Fix,
    parts::{
        Heading, account_avatar, boxed, chip, crumbs, definition, group, hot_row, method_label,
        page, problem_line, row, server_name, slim_row,
    },
    remove::open_remove_account,
    settings_tab::{Page, ServerKeys, SettingsTab},
    sign_in::{SignInStart, open_sign_in},
    ssh_key::open_ssh_key_dialog,
};
use crate::{
    accounts::{Account, AccountId},
    app::connectors::{Connectors, forget_owner},
    git::ssh_key::{KeyPair, same_key},
    ui::{Look as _, counted},
};

const ACCOUNTS: &str = "Accounts";
const ADD_ACCOUNT: &str = "Add account…";
const SIGN_IN_AGAIN: &str = "Sign in again";
const REMOVE: &str = "Remove…";
const SIGN_IN: &str = "Sign-in";
const METHOD: &str = "Method";
const SCOPES: &str = "Scopes";
const TOKEN: &str = "Token";
const SSH_KEY: &str = "SSH key";
const CREATE_KEY: &str = "Create SSH key…";
const COPY_PUBLIC_KEY: &str = "Copy public key";
const FASTGIT_KEY: &str = "FastGit's key";
const READ_ONLY: &str = "Read-only";
const REMOTE_OWNERS: &str = "Remote owners";
const REMOTE_OWNERS_ABOUT: &str =
    "When another account also fits a remote, FastGit uses this one for these owners' repositories.";
const NO_OWNERS: &str = "None yet: FastGit asks when two accounts fit a remote.";
const FORGET: &str = "Forget";
const NO_ACCOUNTS: &str = "No accounts yet";
const NO_ACCOUNTS_ABOUT: &str = "Add one to clone your repositories, publish new ones and add SSH keys. \
     Git keeps using your own credentials meanwhile.";
const NO_SCOPES: &str = "None reported";
#[cfg(windows)]
const ACCOUNTS_ABOUT: &str = "FastGit signs in to hosting services to clone, create and publish repositories. \
     It keeps their tokens in Windows Credential Manager under its own names, apart from the credentials Git uses in a terminal.";
#[cfg(not(windows))]
const ACCOUNTS_ABOUT: &str = "FastGit signs in to hosting services to clone, create and publish repositories. \
     It keeps their tokens apart from the credentials Git uses in a terminal.";

pub(crate) fn service_label(account: &Account, cx: &App) -> String {
    Connectors::of(cx)
        .service_of(account)
        .map_or_else(|| account.service.clone(), |choice| choice.service().label.clone())
}

fn token_place(account: &AccountId) -> String {
    if cfg!(windows) {
        format!("In Windows Credential Manager as FastGit/{}", account.as_str())
    } else {
        "In FastGit's token folder for this login session".to_owned()
    }
}

fn day(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}

impl SettingsTab {
    pub(super) fn accounts_page(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let connectors = Connectors::of(cx);
        let accounts = connectors.accounts();
        let heading = Heading::new(ACCOUNTS).about(ACCOUNTS_ABOUT).action(
            Button::new("add-account")
                .primary()
                .icon(IconName::Plus)
                .label(ADD_ACCOUNT)
                .on_click(|_, window, cx| open_sign_in(SignInStart::Fresh, window, cx)),
        );
        let muted = cx.theme().muted_foreground;
        let list = if accounts.is_empty() {
            v_flex()
                .items_center()
                .gap_2()
                .px_6()
                .py_12()
                .border_1()
                .border_dashed()
                .border_color(cx.theme().border)
                .rounded(px(8.))
                .child(Icon::new(IconName::User).large().text_color(muted))
                .child(div().font_medium().child(NO_ACCOUNTS))
                .child(div().sm_text().text_center().text_color(muted).child(NO_ACCOUNTS_ABOUT))
                .into_any_element()
        } else {
            let rows = accounts
                .iter()
                .enumerate()
                .map(|(ix, account)| self.account_row(ix, account, cx))
                .collect();
            boxed(rows, cx).into_any_element()
        };
        page(v_flex().gap_6().child(heading.render(cx)).child(list)).into_any_element()
    }

    fn account_row(&self, ix: usize, account: &Account, cx: &mut Context<Self>) -> AnyElement {
        let signed_out = Connectors::of(cx).is_signed_out(&account.id);
        let service = service_label(account, cx);
        let muted = cx.theme().muted_foreground;
        let shown = account.name.clone().unwrap_or_else(|| account.login.clone());
        let second = if signed_out {
            problem_line(format!("Signed out: {service} refused the token"), cx).into_any_element()
        } else {
            div()
                .xs_text()
                .text_color(muted)
                .ellipsis()
                .child(format!("{service} · {} · {}", server_name(account), method_label(account.sign_in)))
                .into_any_element()
        };
        let trailing = if signed_out {
            let id = account.id.clone();
            Button::new(("sign-in-again", ix))
                .small()
                .icon(IconName::LogIn)
                .label(SIGN_IN_AGAIN)
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    open_sign_in(SignInStart::Again(id.clone()), window, cx)
                })
                .into_any_element()
        } else {
            h_flex()
                .gap_3()
                .text_color(muted)
                .when(account.ssh_key.is_some(), |this| {
                    this.child(
                        h_flex()
                            .gap_1()
                            .xs_text()
                            .child(Icon::new(IconName::KeyRound).xsmall())
                            .child("SSH"),
                    )
                })
                .child(Icon::new(IconName::ChevronRight).small())
                .into_any_element()
        };
        let id = account.id.clone();
        hot_row(("account-row", ix), cx)
            .cursor_pointer()
            .child(account_avatar(account, 32.))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .sm_text()
                            .child(div().font_semibold().child(shown))
                            .when(account.name.is_some(), |this| {
                                this.child(div().text_color(muted).child(account.login.clone()))
                            }),
                    )
                    .child(second),
            )
            .child(trailing)
            .on_click(cx.listener(move |this, _, _, cx| this.show(Page::Account(id.clone()), cx)))
            .into_any_element()
    }

    pub(super) fn account_page(&mut self, id: &AccountId, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(account) = Connectors::of(cx).account(id) else {
            return div().into_any_element();
        };
        let service = service_label(&account, cx);
        let muted = cx.theme().muted_foreground;
        let shown = account.name.clone().unwrap_or_else(|| account.login.clone());
        let back = cx.entity().downgrade();
        let again = account.id.clone();
        let removed = account.id.clone();
        let heading = Heading::new(shown.clone())
            .media(account_avatar(&account, 48.))
            .subtitle(
                div()
                    .sm_text()
                    .text_color(muted)
                    .child(format!("{} · {service} · {}", account.login, server_name(&account))),
            )
            .action(
                Button::new("account-sign-in-again")
                    .icon(IconName::LogIn)
                    .label(SIGN_IN_AGAIN)
                    .on_click(move |_, window, cx| open_sign_in(SignInStart::Again(again.clone()), window, cx)),
            )
            .action(
                Button::new("account-remove")
                    .danger()
                    .icon(IconName::Trash)
                    .label(REMOVE)
                    .on_click(move |_, window, cx| open_remove_account(removed.clone(), window, cx)),
            );
        let scopes = match account.scopes.is_empty() {
            true => div().sm_text().text_color(muted).child(NO_SCOPES).into_any_element(),
            false => h_flex()
                .gap_1p5()
                .flex_wrap()
                .children(account.scopes.iter().map(|scope| chip(scope.clone(), None, cx)))
                .into_any_element(),
        };
        let sign_in = group(
            SIGN_IN,
            None,
            vec![
                definition(METHOD, div().sm_text().child(method_label(account.sign_in)), cx),
                definition(SCOPES, scopes, cx),
                definition(TOKEN, div().sm_text().text_color(muted).child(token_place(&account.id)), cx),
            ],
            cx,
        );
        let ssh = group(
            SSH_KEY,
            Some(format!("The key FastGit made for this account, then the other keys {service} lists for it.").into()),
            self.key_rows(&account, &service, cx),
            cx,
        );
        let owners = Connectors::of(cx)
            .book_copy()
            .owners
            .into_iter()
            .filter(|choice| choice.account == account.id)
            .collect::<Vec<_>>();
        let owner_rows = match owners.is_empty() {
            true => vec![slim_row().child(div().sm_text().text_color(muted).child(NO_OWNERS)).into_any_element()],
            false => owners
                .into_iter()
                .enumerate()
                .map(|(ix, choice)| {
                    let (host, owner) = (choice.host.clone(), choice.owner.clone());
                    row()
                        .child(Icon::new(IconName::Building2).small().text_color(muted))
                        .child(div().flex_1().sm_text().child(format!("{}/{}", choice.host, choice.owner)))
                        .child(
                            Button::new(("forget-owner", ix))
                                .ghost()
                                .small()
                                .label(FORGET)
                                .on_click(move |_, _, cx| {
                                    let _ = forget_owner(cx, &host, &owner);
                                }),
                        )
                        .into_any_element()
                })
                .collect(),
        };
        let remembered = group(REMOTE_OWNERS, Some(REMOTE_OWNERS_ABOUT.into()), owner_rows, cx);
        page(
            v_flex()
                .gap_6()
                .child(
                    v_flex()
                        .gap_2()
                        .child(crumbs(ACCOUNTS, shown, move |_, cx| {
                            let _ = back.update(cx, |this, cx| this.show(Page::Accounts, cx));
                        }, cx))
                        .child(heading.render(cx)),
                )
                .child(sign_in)
                .child(ssh)
                .child(remembered)
                .children(self.note.clone().map(|note| div().sm_text().text_color(muted).child(note))),
        )
        .into_any_element()
    }

    fn key_rows(&self, account: &Account, service: &str, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme();
        let (muted, blue) = (theme.muted_foreground, theme.blue);
        let mono = theme.mono_font_family.clone();
        let local = account
            .ssh_key
            .as_deref()
            .and_then(|path| KeyPair::read(std::path::Path::new(path)));
        let server = match &self.server_keys {
            Some((shown, keys)) if *shown == account.id => Some(keys),
            _ => None,
        };
        let listed: Vec<_> = match server {
            Some(ServerKeys::Read(keys)) => keys.clone(),
            _ => Vec::new(),
        };
        let mut rows = Vec::new();
        match &local {
            Some(pair) => {
                let on_server = listed.iter().find(|key| same_key(&key.key, &pair.public));
                let title = on_server.map_or_else(|| local_title(pair), |key| key.title.clone());
                let used = account.ssh_repositories.len();
                let usage = match used {
                    0 => "Not used by any repository yet".to_owned(),
                    used => format!("Used by {}", counted(used, "repository", "repositories")),
                };
                let public = pair.public.clone();
                rows.push(
                    row()
                        .child(Icon::new(IconName::KeyRound).small().text_color(muted))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    h_flex()
                                        .gap_1p5()
                                        .child(div().sm_text().font_semibold().ellipsis().child(title))
                                        .when(on_server.is_some(), |this| this.child(chip(FASTGIT_KEY, Some(blue), cx))),
                                )
                                .child(
                                    h_flex()
                                        .gap_1()
                                        .xs_text()
                                        .text_color(muted)
                                        .child(div().font_family(mono.clone()).ellipsis().child(format!(
                                            "{} · {} ·",
                                            pair.kind().trim_start_matches("ssh-"),
                                            pair.fingerprint().unwrap_or_default()
                                        )))
                                        .child(div().flex_none().child(usage)),
                                ),
                        )
                        .child(
                            Button::new("copy-public-key")
                                .ghost()
                                .small()
                                .icon(IconName::Copy)
                                .label(COPY_PUBLIC_KEY)
                                .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(public.clone()))),
                        )
                        .into_any_element(),
                );
            }
            None => {
                let offers_keys = Connectors::of(cx)
                    .service_of(account)
                    .is_some_and(|choice| choice.manifest.features.iter().any(|feature| feature == "ssh_keys"));
                let id = account.id.clone();
                let reload = cx.entity().downgrade();
                rows.push(
                    row()
                        .child(Icon::new(IconName::KeyRound).small().text_color(muted))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .sm_text()
                                .text_color(muted)
                                .child(format!("No SSH key yet. FastGit can make one and add it to {service}, so this account's clones can use SSH.")),
                        )
                        .when(offers_keys, |this| {
                            this.child(Button::new("create-ssh-key").small().label(CREATE_KEY).on_click(
                                move |_, window, cx| {
                                    let reload = reload.clone();
                                    open_ssh_key_dialog(id.clone(), window, cx, move |id, _, cx| {
                                        let _ = reload.update(cx, |this, cx| this.read_server_keys(id.clone(), cx));
                                    })
                                },
                            ))
                        })
                        .into_any_element(),
                );
            }
        }
        match server {
            Some(ServerKeys::Reading(_)) => rows.push(
                slim_row()
                    .child(Spinner::new().small())
                    .child(div().sm_text().text_color(muted).child(format!("Reading the keys {service} lists…")))
                    .into_any_element(),
            ),
            Some(ServerKeys::Failed(trouble)) => {
                let again = account.id.clone();
                let action = matches!(trouble.fix, Fix::SignInAgain | Fix::SignInWithBrowser).then(|| {
                    Button::new("keys-sign-in-again")
                        .small()
                        .label(SIGN_IN_AGAIN)
                        .on_click(move |_, window, cx| open_sign_in(SignInStart::Again(again.clone()), window, cx))
                        .into_any_element()
                });
                rows.push(div().p_2().child(trouble.render(action, cx)).into_any_element())
            }
            Some(ServerKeys::Read(_)) | None => {}
        }
        for (ix, key) in listed.iter().enumerate() {
            let ours = local.as_ref().is_some_and(|pair| same_key(&key.key, &pair.public));
            if ours {
                continue;
            }
            rows.push(
                row()
                    .child(Icon::new(IconName::KeyRound).small().text_color(muted))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .child(div().sm_text().font_semibold().ellipsis().child(key.title.clone()))
                                    .when(key.read_only, |this| this.child(chip(READ_ONLY, None, cx))),
                            )
                            .child(
                                div()
                                    .xs_text()
                                    .font_family(mono.clone())
                                    .text_color(muted)
                                    .ellipsis()
                                    .child(crate::git::ssh_key::fingerprint(&key.key).unwrap_or_default()),
                            ),
                    )
                    .children(key.created_at.as_deref().map(|created| {
                        div()
                            .flex_none()
                            .xs_text()
                            .text_color(muted)
                            .child(SharedString::from(format!("Added {}", day(created))))
                    }))
                    .id(("server-key", ix))
                    .into_any_element(),
            );
        }
        rows
    }
}

fn local_title(pair: &KeyPair) -> String {
    let comment: Vec<&str> = pair.public.split_whitespace().skip(2).collect();
    match comment.is_empty() {
        false => comment.join(" "),
        true => pair
            .private
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
    }
}
