use std::path::PathBuf;

use url::Url;

use super::{Account, AccountBook, AccountId, account::authority};
use crate::git::{Access, Attendance, CredentialScope, Repo};

pub(crate) fn access_for_repo(repo: &Repo, attendance: Attendance) -> Access {
    let book = AccountBook::load();
    if book.is_empty() {
        return Access::attending(attendance);
    }
    access_for(
        &book,
        &repo.remote_urls(),
        std::env::current_exe().ok(),
        attendance,
    )
}

pub(crate) fn access_for_url(url: &str) -> Access {
    let book = AccountBook::load();
    access_for(&book, &[url.to_owned()], std::env::current_exe().ok(), Attendance::Attended)
}

pub(crate) fn serves(book: &AccountBook, remote_urls: &[String], account: &AccountId) -> bool {
    access_for(book, remote_urls, None, Attendance::Unattended)
        .scopes
        .iter()
        .any(|scope| scope.account == account.as_str())
}

pub(crate) fn access_as(account: &Account, owner: &str) -> Access {
    Access {
        helper: std::env::current_exe().ok(),
        scopes: vec![CredentialScope {
            prefix: owner_prefix(account, owner),
            account: account.id.as_str().to_owned(),
        }],
        attendance: Attendance::Attended,
    }
}

fn owner_prefix(account: &Account, owner: &str) -> String {
    let mut prefix = account.server.clone();
    prefix.set_path("");
    if let Ok(mut path) = prefix.path_segments_mut() {
        path.clear().extend(owner.split('/').filter(|part| !part.is_empty()));
    }
    prefix.as_str().trim_end_matches('/').to_owned()
}

fn access_for(
    book: &AccountBook,
    remote_urls: &[String],
    helper: Option<PathBuf>,
    attendance: Attendance,
) -> Access {
    let mut scopes = Vec::new();
    for url in remote_urls.iter().filter_map(|text| Url::parse(text).ok()) {
        if let Some(scope) = scope_for(book, &url).filter(|scope| !scopes.contains(scope)) {
            scopes.push(scope);
        }
    }
    Access {
        helper,
        scopes,
        attendance,
    }
}

fn scope_for(book: &AccountBook, remote: &Url) -> Option<CredentialScope> {
    let scheme = remote.scheme();
    if !matches!(scheme, "https" | "http") {
        return None;
    }
    let host = authority(remote)?;
    let server = format!("{scheme}://{host}");
    let on_host: Vec<_> = book
        .on_host(scheme, &host)
        .filter(|account| account.covers(remote.path()))
        .collect();
    match on_host.as_slice() {
        [] => None,
        [only] => Some(CredentialScope {
            prefix: as_written_in(remote, &server, only),
            account: only.id.as_str().to_owned(),
        }),
        _ => book
            .owners
            .iter()
            .filter(|choice| choice.host.eq_ignore_ascii_case(&host))
            .find(|choice| is_under(remote.path(), &choice.owner))
            .map(|choice| CredentialScope {
                prefix: format!("{server}/{}", choice.owner),
                account: choice.account.as_str().to_owned(),
            }),
    }
}

fn as_written_in(remote: &Url, server: &str, account: &Account) -> String {
    let depth = account
        .server
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .count();
    let written: Vec<&str> = remote.path_segments().into_iter().flatten().take(depth).collect();
    match written.as_slice() {
        [] => server.to_owned(),
        parts => format!("{server}/{}", parts.join("/")),
    }
}

fn is_under(path: &str, owner: &str) -> bool {
    path.trim_start_matches('/')
        .strip_prefix(owner)
        .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{access_for, owner_prefix, serves};
    use crate::{
        accounts::{AccountBook, account::fixtures},
        git::{Access, Attendance, CredentialScope},
    };

    fn remotes(urls: &[&str]) -> Vec<String> {
        urls.iter().map(|url| (*url).to_owned()).collect()
    }

    fn scoped(prefix: &str, account: &str) -> CredentialScope {
        CredentialScope {
            prefix: prefix.into(),
            account: account.into(),
        }
    }

    #[test]
    fn a_single_account_on_a_server_answers_for_the_whole_server() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        let access = access_for(
            &book,
            &remotes(&["https://github.com/a/b.git", "git@github.com:a/b.git"]),
            Some(PathBuf::from("sternegit")),
            Attendance::Attended,
        );
        assert_eq!(access.scopes, [scoped("https://github.com", "github:github.com:1")]);
    }

    #[test]
    fn two_accounts_on_a_server_answer_only_for_the_owners_chosen_for_them() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        book.put(fixtures::github("2"));
        book.choose("github.com", "work-org", fixtures::github("2").id);
        let access = access_for(
            &book,
            &remotes(&[
                "https://github.com/work-org/app.git",
                "https://github.com/work-organisation/other.git",
                "https://github.com/someone/fork.git",
            ]),
            Some(PathBuf::from("sternegit")),
            Attendance::Unattended,
        );
        assert_eq!(
            access.scopes,
            [scoped("https://github.com/work-org", "github:github.com:2")]
        );
        assert_eq!(access.attendance, Attendance::Unattended);
    }

    #[test]
    fn an_account_serves_the_remotes_it_answers_for_and_no_others() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        let on_github = remotes(&["https://github.com/a/b.git"]);
        let over_ssh = remotes(&["git@github.com:a/b.git"]);
        assert!(serves(&book, &on_github, &fixtures::github("1").id));
        assert!(!serves(&book, &on_github, &fixtures::github("2").id), "unknown account");
        assert!(!serves(&book, &over_ssh, &fixtures::github("1").id), "ssh");
        book.put(fixtures::github("2"));
        assert!(!serves(&book, &on_github, &fixtures::github("2").id), "two fit, none chosen");
        book.choose("github.com", "a", fixtures::github("2").id);
        assert!(serves(&book, &on_github, &fixtures::github("2").id), "chosen for the owner");
    }

    #[test]
    fn accounts_on_server_paths_answer_each_for_the_remotes_under_their_own_path() {
        let mut book = AccountBook::default();
        book.put(fixtures::account("https://dev.azure.com/contoso", "1", &[]));
        book.put(fixtures::account("https://dev.azure.com/fabrikam", "2", &[]));
        let access = access_for(
            &book,
            &remotes(&[
                "https://dev.azure.com/contoso/Web/_git/app",
                "https://fabrikam@dev.azure.com/Fabrikam/Game%20Tools/_git/build",
                "https://dev.azure.com/northwind/Web/_git/app",
            ]),
            Some(PathBuf::from("sternegit")),
            Attendance::Attended,
        );
        assert_eq!(
            access.scopes,
            [
                scoped("https://dev.azure.com/contoso", "github:dev.azure.com:1"),
                scoped("https://dev.azure.com/Fabrikam", "github:dev.azure.com:2"),
            ]
        );
    }

    #[test]
    fn cloning_as_an_account_scopes_it_to_the_owner_from_the_server_root() {
        let github = fixtures::github("1");
        assert_eq!(owner_prefix(&github, "octo"), "https://github.com/octo");
        let azure = fixtures::account("https://dev.azure.com/contoso/", "1", &[]);
        assert_eq!(owner_prefix(&azure, "contoso/Game Tools"), "https://dev.azure.com/contoso/Game%20Tools");
        let server = fixtures::account("http://devops.example.com:8080/tfs/DefaultCollection", "1", &[]);
        assert_eq!(owner_prefix(&server, "/tfs/DefaultCollection/Game/"), "http://devops.example.com:8080/tfs/DefaultCollection/Game");
    }

    #[test]
    fn remotes_on_servers_without_an_account_are_left_to_the_users_own_helpers() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        let access = access_for(
            &book,
            &remotes(&["https://gitlab.com/a/b.git", "/local/path"]),
            Some(PathBuf::from("sternegit")),
            Attendance::Attended,
        );
        assert_eq!(access, Access {
            helper: Some(PathBuf::from("sternegit")),
            ..Access::default()
        });
    }
}
