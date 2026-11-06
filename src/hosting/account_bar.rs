use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Sizable as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        menu::PopupMenuItem,
    },
    div, px,
};

use super::{
    accounts_page::service_label,
    parts::{Picker, account_avatar, server_name},
};
use crate::{
    accounts::AccountId,
    app::connectors::{Connectors, choose_owner},
    connect::Remote,
    ui::{Handler, Look as _},
};

const USE: &str = "Use";
const NOT_NOW: &str = "Not now: Git keeps using your own credentials";
const BAR_HEIGHT: f32 = 44.;
const PICKER_WIDTH: f32 = 210.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AccountQuestion {
    pub(crate) host: String,
    pub(crate) owner: String,
    pub(crate) accounts: Vec<AccountId>,
    pub(crate) chosen: usize,
}

pub(crate) fn account_question(remote_urls: &[String], cx: &App) -> Option<AccountQuestion> {
    let connectors = Connectors::of(cx);
    remote_urls.iter().find_map(|url| {
        let remote = Remote::parse(url)?;
        connectors
            .claims(url)
            .into_iter()
            .find(|claim| claim.accounts.len() > 1)
            .map(|claim| AccountQuestion {
                host: remote.host.clone(),
                owner: claim.repo.owner,
                accounts: claim.accounts,
                chosen: 0,
            })
    })
}

pub(crate) fn account_bar(
    question: &AccountQuestion,
    on_pick: Handler<usize>,
    on_dismiss: Handler<()>,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let (blue, border) = (theme.blue, theme.border);
    let connectors = Connectors::of(cx);
    let accounts: Vec<_> = question
        .accounts
        .iter()
        .filter_map(|id| connectors.account(id))
        .collect();
    let chosen = accounts.get(question.chosen).cloned();
    let count = match accounts.len() {
        2 => "Two".to_owned(),
        count => count.to_string(),
    };
    let labels: Vec<(String, String)> = accounts
        .iter()
        .map(|account| (account.login.clone(), format!("{} · {}", service_label(account, cx), server_name(account))))
        .collect();
    let picked = question.chosen;
    let pick = on_pick.clone();
    let mut picker = Picker::new("account-choice", chosen.as_ref().map_or_else(String::new, |account| account.login.clone()));
    if let Some(account) = &chosen {
        picker = picker.lead(account_avatar(account, 18.));
    }
    let picker = picker.menu(move |menu, _, _| {
        labels.iter().enumerate().fold(menu, |menu, (ix, (login, server))| {
            let pick = pick.clone();
            menu.item(
                PopupMenuItem::new(format!("{login}  ·  {server}"))
                    .checked(ix == picked)
                    .on_click(move |_, window, cx| pick(&ix, window, cx)),
            )
        })
    });
    let (host, owner) = (question.host.clone(), question.owner.clone());
    let chosen_id = chosen.map(|account| account.id);
    h_flex()
        .flex_none()
        .h(px(BAR_HEIGHT))
        .gap_2p5()
        .pl_3()
        .pr_2()
        .bg(blue.opacity(0.08))
        .border_b_1()
        .border_color(border)
        .child(Icon::new(IconName::User).small().text_color(blue))
        .child(div().flex_1().min_w_0().sm_text().ellipsis().child(format!(
            "{count} accounts fit {}/{}. Which one should SterneGit use for {}'s repositories?",
            question.host, question.owner, question.owner
        )))
        .child(div().flex_none().w(px(PICKER_WIDTH)).child(picker))
        .child(
            Button::new("use-account")
                .primary()
                .small()
                .label(USE)
                .on_click(move |_, _, cx: &mut App| {
                    if let Some(id) = &chosen_id {
                        let _ = choose_owner(cx, &host, &owner, id.clone());
                    }
                }),
        )
        .child(
            Button::new("dismiss-account-choice")
                .ghost()
                .small()
                .icon(IconName::Close)
                .tooltip(NOT_NOW)
                .on_click(move |_, window: &mut Window, cx| on_dismiss(&(), window, cx)),
        )
        .into_any_element()
}
