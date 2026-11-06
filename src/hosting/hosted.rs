use std::path::Path;

use fastgit_protocol::{Failure, NewRepository, Owner, Visibility};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _,
    SharedString, Styled as _, Subscription, Task, Window,
    component::{
        h_flex,
        input::{Input, InputState},
        menu::PopupMenuItem,
        radio::Radio,
    },
};

use super::{
    accounts_page::service_label,
    failure::Trouble,
    parts::{Picker, account_avatar, toggle_pair},
    ssh_key::local_key,
};
use crate::{
    accounts::{Account, AccountId},
    app::connectors::{Connectors, change_account, choose_owner, note_failure},
    connect::{CallError, Forge},
    git::{
        add_remote,
        ssh_key::{known_hosts_file, knows_host, set_ssh_command, ssh_command},
    },
    ui::run_async_in_background,
};

const CREATES_REPOSITORIES: &str = "repositories.create";
const ORIGIN: &str = "origin";
const SSH_NEEDS_KEY: &str = "Create an SSH key for this account first, in Settings → Accounts";

pub(crate) struct Hosted {
    pub(crate) account: Option<AccountId>,
    pub(crate) owners: Vec<Owner>,
    pub(crate) owner: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) over_ssh: bool,
    pub(crate) description: Entity<InputState>,
    reading_owners: Option<Task<()>>,
    _connectors: Subscription,
}

pub(crate) struct Creation {
    forge: Forge,
    service: String,
    account: Account,
    new: NewRepository,
    ssh: Option<(std::path::PathBuf, std::path::PathBuf)>,
}

pub(crate) struct Created {
    pub(crate) url: String,
    pub(crate) ssh_command: Option<String>,
}

pub(crate) fn creating_accounts(cx: &App) -> Vec<Account> {
    let connectors = Connectors::of(cx);
    let accounts: Vec<Account> = connectors
        .accounts()
        .into_iter()
        .filter(|account| {
            connectors
                .service_of(account)
                .is_some_and(|choice| choice.manifest.features.iter().any(|feature| feature == CREATES_REPOSITORIES))
        })
        .collect();
    accounts
}

fn first_usable(cx: &App) -> Option<Account> {
    let accounts = creating_accounts(cx);
    let connectors = Connectors::of(cx);
    let usable = accounts.iter().find(|account| !connectors.is_signed_out(&account.id)).cloned();
    usable.or_else(|| accounts.into_iter().next())
}

impl Hosted {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let first = first_usable(cx);
        let mut this = Self {
            account: None,
            owners: Vec::new(),
            owner: None,
            visibility: Visibility::Private,
            over_ssh: false,
            description: cx.new(|cx| InputState::new(window, cx).placeholder("Optional")),
            reading_owners: None,
            _connectors: cx.observe_global::<Connectors>(|this, cx| {
                let gone = this.account.as_ref().is_some_and(|id| Connectors::of(cx).account(id).is_none());
                if this.account.is_none() || gone {
                    match first_usable(cx) {
                        Some(first) => this.use_account(first.id, cx),
                        None => this.account = None,
                    }
                }
                cx.notify();
            }),
        };
        if let Some(first) = first {
            this.use_account(first.id, cx);
        }
        this
    }

    pub(crate) fn chosen_account(&self, cx: &App) -> Option<Account> {
        Connectors::of(cx).account(self.account.as_ref()?)
    }

    pub(crate) fn use_account(&mut self, id: AccountId, cx: &mut Context<Self>) {
        let account = Connectors::of(cx).account(&id);
        self.over_ssh = account
            .as_ref()
            .is_some_and(|account| account.clone_over_ssh && account.ssh_key.is_some());
        self.owner = account.as_ref().map(|account| account.login.clone());
        self.owners.clear();
        self.account = Some(id.clone());
        if let Some(forge) = Connectors::of(cx).forge(&id, None) {
            self.reading_owners = Some(run_async_in_background(cx, forge.owners(), move |this, owners, cx| {
                this.reading_owners = None;
                match owners {
                    Ok(owners) => {
                        let creatable = owners.into_iter().filter(|owner| owner.can_create).collect::<Vec<_>>();
                        if let Some(first) = creatable.first() {
                            let keep = this.owner.as_ref().is_some_and(|owner| creatable.iter().any(|known| &known.login == owner));
                            if !keep {
                                this.owner = Some(first.login.clone());
                            }
                        }
                        this.owners = creatable;
                        this.fit_visibility();
                    }
                    Err(error) => note_failure(cx, &id, &error),
                }
                cx.notify();
            }));
        }
        cx.notify();
    }

    fn visibilities(&self) -> Vec<Visibility> {
        self.owners
            .iter()
            .find(|owner| Some(&owner.login) == self.owner.as_ref())
            .map(|owner| owner.visibilities.clone())
            .filter(|visibilities| !visibilities.is_empty())
            .unwrap_or_else(|| vec![Visibility::Public, Visibility::Private])
    }

    fn fit_visibility(&mut self) {
        let offered = self.visibilities();
        if !offered.contains(&self.visibility) {
            self.visibility = offered.iter().copied().find(|visibility| *visibility == Visibility::Private).unwrap_or(offered[0]);
        }
    }

    pub(crate) fn account_picker(&self, id: &'static str, disabled: bool, cx: &mut Context<Self>) -> AnyElement {
        let accounts = creating_accounts(cx);
        let chosen = self.chosen_account(cx);
        let shown = chosen
            .as_ref()
            .map_or_else(String::new, |account| format!("{} · {}", account.login, service_label(account, cx)));
        let this = cx.entity().downgrade();
        let labels: Vec<(AccountId, String)> = accounts
            .iter()
            .map(|account| (account.id.clone(), format!("{} · {}", account.login, service_label(account, cx))))
            .collect();
        let mut picker = Picker::new(id, shown).disabled(disabled);
        if let Some(account) = &chosen {
            picker = picker.lead(account_avatar(account, 18.));
        }
        picker
            .menu(move |menu, _, _| {
                labels.iter().fold(menu, |menu, (id, label)| {
                    let this = this.clone();
                    let id = id.clone();
                    menu.item(PopupMenuItem::new(label.clone()).on_click(move |_, _, cx| {
                        let _ = this.update(cx, |this, cx| this.use_account(id.clone(), cx));
                    }))
                })
            })
            .into_any_element()
    }

    pub(crate) fn owner_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let owners: Vec<String> = self.owners.iter().map(|owner| owner.login.clone()).collect();
        let this = cx.entity().downgrade();
        Picker::new("hosted-owner", self.owner.clone().unwrap_or_default())
            .menu(move |menu, _, _| {
                owners.iter().fold(menu, |menu, login| {
                    let this = this.clone();
                    let chosen = login.clone();
                    menu.item(PopupMenuItem::new(login.clone()).on_click(move |_, _, cx| {
                        let _ = this.update(cx, |this, cx| {
                            this.owner = Some(chosen.clone());
                            this.fit_visibility();
                            cx.notify();
                        });
                    }))
                })
            })
            .into_any_element()
    }

    pub(crate) fn visibility_choice(&self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .gap_4()
            .h_8()
            .children(self.visibilities().into_iter().map(|visibility| {
                let label = match visibility {
                    Visibility::Public => "Public",
                    Visibility::Private => "Private",
                    Visibility::Internal => "Internal",
                    Visibility::Unknown => "Other",
                };
                Radio::new(SharedString::from(label))
                    .label(label)
                    .checked(self.visibility == visibility)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.visibility = visibility;
                        cx.notify();
                    }))
            }))
            .into_any_element()
    }

    pub(crate) fn protocol_choice(&self, cx: &mut Context<Self>) -> AnyElement {
        let has_key = self.chosen_account(cx).is_some_and(|account| account.ssh_key.is_some());
        let this = cx.entity().downgrade();
        toggle_pair(
            "hosted-protocol",
            ["HTTPS", "SSH"],
            usize::from(self.over_ssh && has_key),
            [true, has_key],
            [None, (!has_key).then_some(SSH_NEEDS_KEY)],
            move |ix, _, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.over_ssh = ix == 1;
                    cx.notify();
                });
            },
            cx,
        )
    }

    pub(crate) fn description_input(&self) -> Input {
        Input::new(&self.description)
    }

    pub(crate) fn creation(&self, name: &str, cx: &App) -> Option<Creation> {
        let account = self.chosen_account(cx)?;
        let forge = Connectors::of(cx).forge(&account.id, None)?;
        let owner = self.owner.clone().unwrap_or_else(|| account.login.clone());
        let description = self.description.read(cx).value().trim().to_owned();
        let ssh = match (self.over_ssh, local_key(&account), known_hosts_file()) {
            (true, Some(key), Some(known_hosts)) => Some((key.private, known_hosts)),
            _ => None,
        };
        Some(Creation {
            forge,
            service: service_label(&account, cx),
            new: NewRepository {
                owner,
                name: name.trim().to_owned(),
                description: (!description.is_empty()).then_some(description),
                visibility: self.visibility,
                initialize: false,
            },
            account,
            ssh,
        })
    }
}

impl Creation {
    pub(crate) fn account(&self) -> &Account {
        &self.account
    }

    pub(crate) fn owner(&self) -> &str {
        &self.new.owner
    }

    pub(crate) fn problem(&self, error: &CallError) -> SharedString {
        name_problem(error, &self.new.owner, &self.new.name, &self.account.login, &self.service)
    }

    pub(crate) async fn create(self) -> Result<Created, SharedString> {
        let created = self.forge.create_repository(self.new.clone()).await;
        let repository = created.map_err(|error| self.problem(&error))?;
        let ssh = self.ssh.and_then(|(key, known_hosts)| {
            let url = repository.clone.ssh.clone()?;
            let host = url.split_once('@').map_or(url.as_str(), |(_, rest)| rest);
            let host = host.split([':', '/']).next().unwrap_or_default();
            Some((url.clone(), ssh_command(&key, &known_hosts, knows_host(&known_hosts, host))))
        });
        let (url, ssh_command) = match ssh {
            Some((url, command)) => (url, Some(command)),
            None => (repository.clone.https.clone(), None),
        };
        Ok(Created {
            url,
            ssh_command,
        })
    }
}

pub(crate) fn connect_origin(folder: &Path, created: &Created) -> anyhow::Result<()> {
    add_remote(folder, ORIGIN, &created.url)?;
    if let Some(command) = &created.ssh_command {
        set_ssh_command(folder, command)?;
    }
    Ok(())
}

pub(crate) fn remember_use(account: &Account, owner: &str, folder: &Path, over_ssh: bool, cx: &mut App) {
    let folder = folder.to_path_buf();
    let _ = change_account(cx, &account.id, |account| {
        account.clone_over_ssh = over_ssh;
        if over_ssh && !account.ssh_repositories.contains(&folder) {
            account.ssh_repositories.push(folder);
        }
    });
    let sharing_server = Connectors::of(cx)
        .accounts()
        .iter()
        .filter(|other| other.server == account.server)
        .count();
    if sharing_server > 1 {
        if let Some(host) = account.server.host_str() {
            let _ = choose_owner(cx, host, owner, account.id.clone());
        }
    }
}

pub(crate) fn name_problem(error: &CallError, owner: &str, name: &str, login: &str, service: &str) -> SharedString {
    match error {
        CallError::Failed(Failure::Invalid { fields, message }) => {
            match fields.iter().find(|field| field.field == "name") {
                Some(field) if field.message.contains("already exists") => {
                    format!("{owner} already has a repository named {name}.").into()
                }
                Some(field) => field.message.clone().into(),
                None => message.clone().into(),
            }
        }
        error => {
            let trouble = Trouble::described(error, login, service, service, Vec::new());
            format!("{}. {}", trouble.title, trouble.message).into()
        }
    }
}

#[cfg(test)]
mod tests {
    use fastgit_protocol::{FieldError, Failure};

    use super::name_problem;
    use crate::connect::CallError;

    #[test]
    fn a_taken_name_is_said_plainly() {
        let taken = CallError::Failed(Failure::Invalid {
            fields: vec![FieldError {
                field: "name".into(),
                message: "name already exists on this account".into(),
            }],
            message: "Repository creation failed.".into(),
        });
        assert_eq!(
            name_problem(&taken, "octocat", "starfall", "octocat", "GitHub"),
            "octocat already has a repository named starfall."
        );
        assert_eq!(
            name_problem(&CallError::Failed(Failure::SignedOut), "o", "n", "octocat", "GitHub"),
            "GitHub refused the token of octocat. It expired or was revoked. Sign in again to keep using this account."
        );
    }
}
