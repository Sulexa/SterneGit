use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Context, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Window,
    component::{
        ActiveTheme as _, WindowExt as _,
        button::{Button, ButtonVariants as _},
        checkbox::Checkbox,
        h_flex, v_flex,
    },
    div, px,
};

use super::{accounts_page::service_label, ssh_key::local_key};
use crate::{
    accounts::{Account, AccountId},
    app::connectors::{Connectors, forget_account},
    git::ssh_key::{names_key, ssh_command_of, unset_ssh_command},
    ui::{Look as _, counted},
};

const WIDTH: f32 = 520.;
const CANCEL: &str = "Cancel";
const REMOVE: &str = "Remove";
const UNDO: &str = "Remove core.sshCommand from them, so they use your own SSH setup again";

struct RemoveForm {
    account: Account,
    service: String,
    using: Vec<PathBuf>,
    undo: bool,
    problem: Option<SharedString>,
}

impl RemoveForm {
    fn remove(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.undo {
            for repository in &self.using {
                let _ = unset_ssh_command(repository);
            }
        }
        if let Some(pair) = local_key(&self.account) {
            let _ = pair.delete();
        }
        match forget_account(cx, &self.account.id) {
            Ok(()) => window.close_dialog(cx),
            Err(error) => {
                self.problem = Some(format!("Couldn't remove the account: {error}").into());
                cx.notify();
            }
        }
    }
}

impl Render for RemoveForm {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (muted, border, red) = (theme.muted_foreground, theme.border, theme.red);
        let mono = theme.mono_font_family.clone();
        let store = if cfg!(windows) { " from Windows Credential Manager" } else { "" };
        let key = match self.account.ssh_key.is_some() {
            true => format!(" It deletes its copy of the SSH key too; {} keeps the public half until you delete it there.", self.service),
            false => String::new(),
        };
        let using = (!self.using.is_empty()).then(|| {
            v_flex()
                .gap_1()
                .p_3()
                .border_1()
                .border_color(border)
                .rounded(px(8.))
                .child(div().sm_text().child(format!(
                    "{} this account's SSH key:",
                    counted(self.using.len(), "repository uses", "repositories use")
                )))
                .children(self.using.iter().map(|path| {
                    div().xs_text().font_family(mono.clone()).text_color(muted).child(path.display().to_string())
                }))
                .child(
                    Checkbox::new("undo-ssh-command")
                        .mt_1()
                        .label(UNDO)
                        .checked(self.undo)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.undo = *checked;
                            cx.notify();
                        })),
                )
        });
        v_flex()
            .gap_3()
            .child(div().sm_text().text_color(muted).child(format!(
                "FastGit forgets the account and deletes its token{store}.{key}"
            )))
            .children(using)
            .children(self.problem.clone().map(|problem| div().xs_text().text_color(red).child(problem)))
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .mt_2()
                    .child(Button::new("remove-cancel").label(CANCEL).on_click(|_, window, cx| window.close_dialog(cx)))
                    .child(
                        Button::new("remove-account-confirm")
                            .danger()
                            .label(REMOVE)
                            .on_click(cx.listener(|this, _, window, cx| this.remove(window, cx))),
                    ),
            )
    }
}

pub(crate) fn open_remove_account(id: AccountId, window: &mut Window, cx: &mut App) {
    let Some(account) = Connectors::of(cx).account(&id) else {
        return;
    };
    let using = repositories_using_key(&account);
    let service = service_label(&account, cx);
    let login = account.login.clone();
    let form = cx.new(|_| RemoveForm {
        account,
        service,
        using,
        undo: true,
        problem: None,
    });
    window.open_dialog(cx, move |dialog, _, _| {
        dialog.w(px(WIDTH)).title(format!("Remove {login}?")).child(form.clone())
    });
}

fn repositories_using_key(account: &Account) -> Vec<PathBuf> {
    let Some(key) = account.ssh_key.as_deref() else {
        return Vec::new();
    };
    let key = std::path::Path::new(key);
    account
        .ssh_repositories
        .iter()
        .filter(|repository| ssh_command_of(repository).is_some_and(|command| names_key(&command, key)))
        .cloned()
        .collect()
}
