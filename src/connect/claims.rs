use sternegit_protocol::RepoRef;

use super::{
    catalog::{Catalog, Connector},
    manifest::RemotePattern,
    remote::Remote,
};
use crate::{
    accounts::{Account, AccountBook, AccountId},
    net::services::ConnectorId,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Claim {
    pub(crate) connector: ConnectorId,
    pub(crate) service: String,
    pub(crate) repo: RepoRef,
    pub(crate) accounts: Vec<AccountId>,
}

impl Catalog {
    pub(crate) fn claims(&self, remote_url: &str, book: &AccountBook) -> Vec<Claim> {
        let Some(remote) = Remote::parse(remote_url) else {
            return Vec::new();
        };
        self.connectors()
            .iter()
            .filter(|connector| connector.started.is_some())
            .flat_map(|connector| claims_of(connector, &remote, book))
            .collect()
    }
}

fn claims_of(connector: &Connector, remote: &Remote, book: &AccountBook) -> Vec<Claim> {
    let Some(manifest) = &connector.manifest else {
        return Vec::new();
    };
    manifest
        .services
        .iter()
        .filter_map(|service| {
            let accounts: Vec<&Account> = book
                .accounts
                .iter()
                .filter(|account| account.connector == connector.id.0 && account.service == service.id)
                .collect();
            service.remotes.iter().find_map(|pattern| {
                let repo = pattern.path.locate(&remote.path)?;
                let fitting = fitting_accounts(pattern, &accounts, &remote.host, &repo.owner)?;
                Some(Claim {
                    connector: connector.id.clone(),
                    service: service.id.clone(),
                    accounts: chosen(fitting, book, &remote.host, &repo.owner),
                    repo,
                })
            })
        })
        .collect()
}

fn fitting_accounts(
    pattern: &RemotePattern,
    accounts: &[&Account],
    host: &str,
    owner: &str,
) -> Option<Vec<AccountId>> {
    let mut fitting = Vec::new();
    let mut claimed = false;
    for listed in &pattern.hosts {
        if listed.contains('{') {
            for account in accounts {
                if account.server.host_str().is_some_and(|server| server.eq_ignore_ascii_case(host)) {
                    claimed = true;
                    if account.covers(owner) {
                        fitting.push(account.id.clone());
                    }
                }
            }
        } else if listed.eq_ignore_ascii_case(host) {
            claimed = true;
            fitting.extend(
                accounts
                    .iter()
                    .filter(|account| account.covers(owner))
                    .map(|account| account.id.clone()),
            );
        }
    }
    fitting.sort();
    fitting.dedup();
    claimed.then_some(fitting)
}

fn chosen(fitting: Vec<AccountId>, book: &AccountBook, host: &str, owner: &str) -> Vec<AccountId> {
    if fitting.len() < 2 {
        return fitting;
    }
    let remembered = book.owners.iter().find(|choice| {
        choice.host.eq_ignore_ascii_case(host)
            && is_under(owner, &choice.owner)
            && fitting.contains(&choice.account)
    });
    match remembered {
        Some(choice) => vec![choice.account.clone()],
        None => fitting,
    }
}

fn is_under(owner: &str, chosen: &str) -> bool {
    owner == chosen
        || owner
            .strip_prefix(chosen)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use sternegit_protocol::RepoRef;

    use super::Claim;
    use crate::{
        accounts::{Account, AccountBook, AccountId, authority, fixtures},
        connect::{
            Approvals,
            catalog::{
                Catalog, ConnectorFolders,
                tests::{sources, started_bundled},
            },
        },
        net::services::ConnectorId,
    };

    fn claim(service: &str, owner: &str, name: &str, accounts: &[&str]) -> Claim {
        claim_by("github", service, owner, name, accounts)
    }

    fn claim_by(connector: &str, service: &str, owner: &str, name: &str, accounts: &[&str]) -> Claim {
        Claim {
            connector: ConnectorId(connector.into()),
            service: service.into(),
            repo: RepoRef {
                owner: owner.into(),
                name: name.into(),
            },
            accounts: accounts.iter().map(|id| AccountId::parse(id).unwrap()).collect(),
        }
    }

    fn azure_devops(service: &str, server: &str, user: &str) -> Account {
        let mut account = fixtures::account(server, user, &[("api", server, true)]);
        account.connector = "azure-devops".into();
        account.service = service.into();
        account.id = AccountId::new("azure-devops", &authority(&account.server).unwrap(), user).unwrap();
        account
    }

    fn book_of(users: &[&str]) -> AccountBook {
        let mut book = AccountBook::default();
        for user in users {
            book.put(fixtures::github(user));
        }
        book
    }

    #[test]
    fn a_github_remote_is_claimed_by_github_with_the_accounts_on_it() {
        let catalog = started_bundled();
        let book = book_of(&["1"]);
        for remote in ["git@github.com:octo/app.git", "https://github.com/octo/app", "ssh://git@ssh.github.com:443/octo/app.git"] {
            assert_eq!(
                catalog.claims(remote, &book),
                [claim("github.com", "octo", "app", &["github:github.com:1"])],
                "{remote}"
            );
        }
    }

    #[test]
    fn a_claim_without_an_account_still_names_its_service() {
        assert_eq!(
            started_bundled().claims("https://github.com/octo/app.git", &AccountBook::default()),
            [claim("github.com", "octo", "app", &[])]
        );
    }

    #[test]
    fn remotes_no_started_connector_knows_are_not_claimed() {
        let catalog = started_bundled();
        let book = book_of(&["1"]);
        for remote in ["https://gitlab.com/group/app.git", "/home/me/app", "https://github.com/octo"] {
            assert!(catalog.claims(remote, &book).is_empty(), "{remote}");
        }
        let folders = ConnectorFolders {
            source: Some(sources()),
            ..ConnectorFolders::default()
        };
        let unstarted = Catalog::discover(&folders, Approvals::default());
        assert!(unstarted.claims("https://github.com/octo/app", &book).is_empty());
    }

    #[test]
    fn two_fitting_accounts_narrow_to_the_one_remembered_for_the_owner() {
        let catalog = started_bundled();
        let mut book = book_of(&["1", "2"]);
        let both = ["github:github.com:1", "github:github.com:2"];
        assert_eq!(catalog.claims("git@github.com:work-org/app.git", &book)[0].accounts, both.map(|id| AccountId::parse(id).unwrap()));
        book.choose("github.com", "work-org", fixtures::github("2").id);
        assert_eq!(
            catalog.claims("git@github.com:work-org/app.git", &book),
            [claim("github.com", "work-org", "app", &["github:github.com:2"])]
        );
        assert_eq!(catalog.claims("git@github.com:work-organisation/app.git", &book)[0].accounts.len(), 2);
    }

    #[test]
    fn an_enterprise_server_is_claimed_only_through_an_account_on_it() {
        let catalog = started_bundled();
        let mut account = fixtures::account(
            "https://ghe.example.com",
            "7",
            &[("api", "https://ghe.example.com/api/v3/", true)],
        );
        account.service = "enterprise".into();
        let mut book = AccountBook::default();
        book.put(account);
        assert_eq!(
            catalog.claims("git@ghe.example.com:team/tool.git", &book),
            [claim("enterprise", "team", "tool", &["github:ghe.example.com:7"])]
        );
        assert!(catalog.claims("https://git.other.com/team/tool.git", &book).is_empty());
    }

    #[test]
    fn an_azure_devops_remote_is_claimed_by_the_account_of_its_own_organization() {
        let catalog = started_bundled();
        let mut book = AccountBook::default();
        book.put(azure_devops("dev.azure.com", "https://dev.azure.com/contoso/", "contoso.1"));
        book.put(azure_devops("dev.azure.com", "https://dev.azure.com/fabrikam/", "fabrikam.1"));
        for remote in [
            "https://dev.azure.com/contoso/Web/_git/app",
            "https://contoso@dev.azure.com/Contoso/Web/_git/app",
            "git@ssh.dev.azure.com:v3/contoso/Web/app",
            "contoso@vs-ssh.visualstudio.com:v3/contoso/Web/app",
        ] {
            let claims = catalog.claims(remote, &book);
            let [only] = claims.as_slice() else {
                panic!("{remote} gave {claims:?}");
            };
            assert_eq!(only.repo.name, "app", "{remote}");
            assert_eq!(only.accounts, [AccountId::parse("azure-devops:dev.azure.com:contoso.1").unwrap()], "{remote}");
        }
        assert_eq!(
            catalog.claims("https://dev.azure.com/fabrikam/Game%20Tools/_git/build%20scripts", &book),
            [claim_by("azure-devops", "dev.azure.com", "fabrikam/Game%20Tools", "build%20scripts", &["azure-devops:dev.azure.com:fabrikam.1"])]
        );
        assert_eq!(
            catalog.claims("git@ssh.dev.azure.com:v3/northwind/Web/app", &book),
            [claim_by("azure-devops", "dev.azure.com", "northwind/Web", "app", &[])]
        );
    }

    #[test]
    fn a_collection_on_an_azure_devops_server_claims_the_remotes_under_it() {
        let catalog = started_bundled();
        let mut book = AccountBook::default();
        let collection = "http://devops.example.com:8080/tfs/DefaultCollection/";
        book.put(azure_devops("server", collection, "tfs.defaultcollection.1"));
        let account = "azure-devops:devops.example.com:8080:tfs.defaultcollection.1";
        for remote in [
            "http://devops.example.com:8080/tfs/DefaultCollection/Game/_git/client",
            "ssh://devops.example.com:22/tfs/DefaultCollection/Game/_git/client",
        ] {
            assert_eq!(
                catalog.claims(remote, &book),
                [claim_by("azure-devops", "server", "tfs/DefaultCollection/Game", "client", &[account])],
                "{remote}"
            );
        }
        assert_eq!(
            catalog.claims("http://devops.example.com:8080/tfs/Other/Game/_git/client", &book),
            [claim_by("azure-devops", "server", "tfs/Other/Game", "client", &[])]
        );
    }
}
