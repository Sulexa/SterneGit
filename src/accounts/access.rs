use std::path::PathBuf;

use url::Url;

use super::{Account, AccountBook, account::authority};
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

pub(crate) fn access_as(account: &Account, owner: &str) -> Access {
    let server = account.server.as_str().trim_end_matches('/');
    Access {
        helper: std::env::current_exe().ok(),
        scopes: vec![CredentialScope {
            prefix: format!("{server}/{}", owner.trim_matches('/')),
            account: account.id.as_str().to_owned(),
        }],
        attendance: Attendance::Attended,
    }
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
    let on_host: Vec<_> = book.on_host(scheme, &host).collect();
    match on_host.as_slice() {
        [] => None,
        [only] => Some(CredentialScope {
            prefix: server,
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

fn is_under(path: &str, owner: &str) -> bool {
    path.trim_start_matches('/')
        .strip_prefix(owner)
        .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::access_for;
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
            Some(PathBuf::from("fastgit")),
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
            Some(PathBuf::from("fastgit")),
            Attendance::Unattended,
        );
        assert_eq!(
            access.scopes,
            [scoped("https://github.com/work-org", "github:github.com:2")]
        );
        assert_eq!(access.attendance, Attendance::Unattended);
    }

    #[test]
    fn remotes_on_servers_without_an_account_are_left_to_the_users_own_helpers() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        let access = access_for(
            &book,
            &remotes(&["https://gitlab.com/a/b.git", "/local/path"]),
            Some(PathBuf::from("fastgit")),
            Attendance::Attended,
        );
        assert_eq!(access, Access {
            helper: Some(PathBuf::from("fastgit")),
            ..Access::default()
        });
    }
}
