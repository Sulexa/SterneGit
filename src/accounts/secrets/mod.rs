#[cfg(windows)]
mod credential_manager;
mod session_files;

use std::{fmt, io, sync::Arc};

use super::AccountId;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Token(String);

impl Token {
    pub(crate) fn new(text: &str) -> Option<Self> {
        let text = text.trim();
        (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_graphic()))
            .then(|| Self(text.to_owned()))
    }

    pub(crate) fn reveal(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(•••)")
    }
}

pub(crate) trait Secrets: Send + Sync {
    fn read(&self, account: &AccountId) -> io::Result<Option<Token>>;
    fn write(&self, account: &AccountId, login: &str, token: &Token) -> io::Result<()>;
    fn erase(&self, account: &AccountId) -> io::Result<()>;
}

#[cfg(windows)]
pub(crate) fn os_secrets() -> Arc<dyn Secrets> {
    Arc::new(credential_manager::CredentialManager)
}

#[cfg(not(windows))]
pub(crate) fn os_secrets() -> Arc<dyn Secrets> {
    Arc::new(session_files::SessionFiles::in_session_folder())
}

#[cfg(test)]
pub(crate) mod memory {
    use std::{collections::HashMap, io, sync::Mutex};

    use super::{Secrets, Token};
    use crate::accounts::AccountId;

    #[derive(Default)]
    pub(crate) struct MemorySecrets(Mutex<HashMap<AccountId, Token>>);

    impl MemorySecrets {
        pub(crate) fn holding(account: &AccountId, token: &str) -> Self {
            let secrets = Self::default();
            secrets.write(account, "login", &Token::new(token).unwrap()).unwrap();
            secrets
        }
    }

    impl Secrets for MemorySecrets {
        fn read(&self, account: &AccountId) -> io::Result<Option<Token>> {
            Ok(self.0.lock().unwrap().get(account).cloned())
        }

        fn write(&self, account: &AccountId, _login: &str, token: &Token) -> io::Result<()> {
            self.0.lock().unwrap().insert(account.clone(), token.clone());
            Ok(())
        }

        fn erase(&self, account: &AccountId) -> io::Result<()> {
            self.0.lock().unwrap().remove(account);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Token;

    #[test]
    fn a_token_is_trimmed_and_never_printed() {
        let token = Token::new(" gho_abc\n").unwrap();
        assert_eq!(token.reveal(), "gho_abc");
        assert_eq!(format!("{token:?}"), "Token(•••)");
    }

    #[test]
    fn a_token_that_could_break_a_header_is_refused() {
        assert_eq!(Token::new("abc\r\nX-Evil: 1"), None);
        assert_eq!(Token::new("with space"), None);
        assert_eq!(Token::new(""), None);
    }
}
