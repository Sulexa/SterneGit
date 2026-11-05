use std::io::{self, BufRead, Write};

use super::{Account, AccountBook, AccountId, Token, account::authority, os_secrets};
use crate::git::{CredentialAnswer, CredentialRequest};

const GET: &str = "get";

pub(crate) fn answer_git(
    account: &str,
    operation: &str,
    input: impl BufRead,
    output: impl Write,
) -> io::Result<()> {
    let request = CredentialRequest::read(input)?;
    let answer = if operation == GET {
        let book = AccountBook::load();
        let found = AccountId::parse(account).and_then(|id| book.account(&id).cloned());
        credentials(&request, found.as_ref(), |id| os_secrets().read(id).ok().flatten())
    } else {
        CredentialAnswer::Nothing
    };
    answer.write(output)
}

fn credentials(
    request: &CredentialRequest,
    account: Option<&Account>,
    token: impl FnOnce(&AccountId) -> Option<Token>,
) -> CredentialAnswer {
    let Some(account) = account else {
        return CredentialAnswer::Quit;
    };
    let host = authority(&account.server);
    if !request.is_for(account.server.scheme(), host.as_deref()) {
        return CredentialAnswer::Quit;
    }
    match token(&account.id) {
        Some(token) => CredentialAnswer::Give {
            username: account.git_user.clone(),
            password: token.reveal().to_owned(),
        },
        None => CredentialAnswer::Quit,
    }
}

#[cfg(test)]
mod tests {
    use super::credentials;
    use crate::{
        accounts::{Token, account::fixtures},
        git::{CredentialAnswer, CredentialRequest},
    };

    fn asking(protocol: &str, host: &str) -> CredentialRequest {
        CredentialRequest {
            protocol: Some(protocol.into()),
            host: Some(host.into()),
        }
    }

    #[test]
    fn git_gets_the_accounts_user_and_token_for_its_server() {
        let account = fixtures::github("1");
        let answer = credentials(&asking("https", "github.com"), Some(&account), |_| {
            Token::new("gho_1")
        });
        assert_eq!(
            answer,
            CredentialAnswer::Give {
                username: "x-access-token".into(),
                password: "gho_1".into(),
            }
        );
    }

    #[test]
    fn git_is_told_to_stop_when_the_account_is_unknown_or_has_no_token() {
        let account = fixtures::github("1");
        let request = asking("https", "github.com");
        assert_eq!(credentials(&request, None, |_| Token::new("x")), CredentialAnswer::Quit);
        assert_eq!(credentials(&request, Some(&account), |_| None), CredentialAnswer::Quit);
    }

    #[test]
    fn a_token_never_goes_to_another_server_or_over_plain_http() {
        let account = fixtures::github("1");
        let token = |_: &_| Token::new("gho_1");
        assert_eq!(
            credentials(&asking("https", "evil.example"), Some(&account), token),
            CredentialAnswer::Quit
        );
        assert_eq!(
            credentials(&asking("http", "github.com"), Some(&account), token),
            CredentialAnswer::Quit
        );
    }
}
