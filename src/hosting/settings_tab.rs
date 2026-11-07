use sternegit_protocol::SshKey;
use gpui_kit::{
    AnyElement, App, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Task, Window,
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex},
    div,
    px,
};

use super::{failure::Trouble, interface_page::zoom_label};
use crate::{
    accounts::AccountId,
    app::{
        connectors::{Connectors, mark_seen, note_failure},
        zoom::{Zoom, current_zoom},
    },
    connect::Standing,
    net::services::ConnectorId,
    ui::{Look as _, SectionHeader, list_row, run_async_in_background},
};

const NAV_WIDTH: f32 = 224.;
const SETTINGS: &str = "Settings";
const ACCOUNTS: &str = "Accounts";
const CONNECTORS: &str = "Connectors";
const INTERFACE: &str = "Interface";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Accounts,
    Account(AccountId),
    Connectors,
    Connector(ConnectorId),
    Interface,
}

pub(crate) enum ServerKeys {
    Reading(#[expect(dead_code)] Task<()>),
    Read(Vec<SshKey>),
    Failed(Trouble),
}

pub(crate) struct SettingsTab {
    pub(super) page: Page,
    pub(super) server_keys: Option<(AccountId, ServerKeys)>,
    pub(super) note: Option<SharedString>,
    focus: FocusHandle,
    _connectors: Subscription,
}

impl SettingsTab {
    pub(crate) fn new(page: Page, cx: &mut Context<Self>) -> Self {
        let connectors = cx.observe_global::<Connectors>(|this, cx| {
            this.settle_page(cx);
            cx.notify();
        });
        let mut this = Self {
            page: Page::Accounts,
            server_keys: None,
            note: None,
            focus: cx.focus_handle(),
            _connectors: connectors,
        };
        this.show(page, cx);
        this
    }

    pub(crate) fn show(&mut self, page: Page, cx: &mut Context<Self>) {
        self.note = None;
        if let Page::Connector(id) = &page {
            let updated = Connectors::of(cx)
                .connector(id)
                .is_some_and(|connector| connector.standing == Standing::Updated);
            if updated {
                mark_seen(cx, id);
            }
        }
        if let Page::Account(id) = &page {
            self.read_server_keys(id.clone(), cx);
        }
        self.page = page;
        cx.notify();
    }

    pub(super) fn tell(&mut self, note: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.note = Some(note.into());
        cx.notify();
    }

    pub(super) fn read_server_keys(&mut self, id: AccountId, cx: &mut Context<Self>) {
        let connectors = Connectors::of(cx);
        let Some(forge) = connectors.forge(&id, None) else {
            self.server_keys = None;
            return;
        };
        let offers_keys = connectors
            .account(&id)
            .and_then(|account| connectors.service_of(&account))
            .is_some_and(|choice| choice.manifest.features.iter().any(|feature| feature == "ssh_keys"));
        if !offers_keys {
            self.server_keys = None;
            return;
        }
        let account = id.clone();
        let task = run_async_in_background(cx, forge.ssh_keys(), move |this, result, cx| {
            if let Err(error) = &result {
                note_failure(cx, &account, error);
            }
            let read = match result {
                Ok(keys) => ServerKeys::Read(keys),
                Err(error) => ServerKeys::Failed(Trouble::of(&error, &account, cx)),
            };
            if this.server_keys.as_ref().is_some_and(|(shown, _)| *shown == account) {
                this.server_keys = Some((account, read));
                cx.notify();
            }
        });
        self.server_keys = Some((id, ServerKeys::Reading(task)));
    }

    fn settle_page(&mut self, cx: &mut Context<Self>) {
        let connectors = Connectors::of(cx);
        let gone = match &self.page {
            Page::Account(id) => connectors.is_ready() && connectors.account(id).is_none(),
            Page::Connector(id) => connectors.is_ready() && connectors.connector(id).is_none(),
            Page::Accounts | Page::Connectors | Page::Interface => false,
        };
        if gone {
            self.page = match self.page {
                Page::Account(_) => Page::Accounts,
                _ => Page::Connectors,
            };
        }
    }

    fn nav(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let connectors = Connectors::of(cx);
        let accounts = connectors.accounts().len();
        let waiting = connectors.waiting().count();
        let on_accounts = matches!(self.page, Page::Accounts | Page::Account(_));
        let on_connectors = matches!(self.page, Page::Connectors | Page::Connector(_));
        let zoom = current_zoom(cx);
        let theme = cx.theme();
        let (muted, yellow, sidebar, border) = (theme.muted_foreground, theme.yellow, theme.sidebar, theme.border);
        let nav_row = |id: &'static str, icon: IconName, label: &'static str, selected: bool, trailing: AnyElement| {
            list_row(id, selected).px_3().child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .child(Icon::new(icon).small().text_color(muted))
                    .child(div().flex_1().sm_text().child(label))
                    .child(trailing),
            )
        };
        let count = div().xs_text().text_color(muted).child(accounts.to_string()).into_any_element();
        let zoomed = match zoom == Zoom::ACTUAL_SIZE {
            true => div().into_any_element(),
            false => div().xs_text().text_color(muted).child(zoom_label(zoom)).into_any_element(),
        };
        let badge = match waiting {
            0 => div().into_any_element(),
            waiting => div()
                .min_w(px(18.))
                .h(px(18.))
                .px(px(5.))
                .rounded(px(9.))
                .bg(yellow)
                .text_color(gpui_kit::black())
                .text_size(px(11.))
                .line_height(px(18.))
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .text_center()
                .child(waiting.to_string())
                .into_any_element(),
        };
        v_flex()
            .flex_none()
            .w(px(NAV_WIDTH))
            .h_full()
            .bg(sidebar)
            .border_r_1()
            .border_color(border)
            .child(SectionHeader::new(SETTINGS))
            .child(
                nav_row("settings-accounts", IconName::User, ACCOUNTS, on_accounts, count)
                    .on_click(cx.listener(|this, _, _, cx| this.show(Page::Accounts, cx))),
            )
            .child(
                nav_row("settings-connectors", IconName::Plug, CONNECTORS, on_connectors, badge)
                    .on_click(cx.listener(|this, _, _, cx| this.show(Page::Connectors, cx))),
            )
            .child(
                nav_row("settings-interface", IconName::Monitor, INTERFACE, self.page == Page::Interface, zoomed)
                    .on_click(cx.listener(|this, _, _, cx| this.show(Page::Interface, cx))),
            )
    }
}

impl Focusable for SettingsTab {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SettingsTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page.clone() {
            Page::Accounts => self.accounts_page(window, cx),
            Page::Account(id) => self.account_page(&id, window, cx),
            Page::Connectors => self.connectors_page(cx),
            Page::Connector(id) => self.connector_page(&id, cx),
            Page::Interface => self.interface_page(cx),
        };
        h_flex()
            .size_full()
            .track_focus(&self.focus)
            .child(self.nav(cx))
            .child(
                div()
                    .id("settings-page")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_y_scroll()
                    .child(content),
            )
    }
}
