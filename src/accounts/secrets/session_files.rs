use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use super::{Secrets, Token};
use crate::accounts::AccountId;

#[cfg(not(windows))]
const TOKENS: &str = "tokens";
const UNFINISHED: &str = "unfinished";

pub(crate) struct SessionFiles {
    folder: Option<PathBuf>,
}

impl SessionFiles {
    #[cfg(not(windows))]
    pub(crate) fn in_session_folder() -> Self {
        Self {
            folder: crate::settings::Folder::Session
                .path()
                .map(|path| path.join(TOKENS)),
        }
    }

    pub(crate) fn at(folder: PathBuf) -> Self {
        Self {
            folder: Some(folder),
        }
    }

    fn file(&self, account: &AccountId) -> io::Result<PathBuf> {
        let folder = self.folder.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "this session has no private folder to keep tokens in",
            )
        })?;
        Ok(folder.join(hex(account.as_str())))
    }
}

impl Secrets for SessionFiles {
    fn read(&self, account: &AccountId) -> io::Result<Option<Token>> {
        match std::fs::read_to_string(self.file(account)?) {
            Ok(text) => Ok(Token::new(&text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn write(&self, account: &AccountId, _login: &str, token: &Token) -> io::Result<()> {
        let file = self.file(account)?;
        if let Some(folder) = file.parent() {
            private_folder(folder)?;
        }
        let unfinished = file.with_extension(UNFINISHED);
        write_private(&unfinished, token.reveal().as_bytes())?;
        std::fs::rename(&unfinished, &file)
    }

    fn erase(&self, account: &AccountId) -> io::Result<()> {
        match std::fs::remove_file(self.file(account)?) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn private_folder(path: &Path) -> io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(path)?;
    #[cfg(unix)]
    std::fs::set_permissions(
        path,
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700),
    )?;
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)?.write_all(bytes)
}

#[cfg(test)]
mod tests {
    use super::SessionFiles;
    use crate::accounts::{AccountId, Secrets as _, Token};

    #[test]
    fn a_token_written_to_the_session_folder_reads_back_until_erased() {
        let folder = std::env::temp_dir().join(format!("fastgit-session-{}", std::process::id()));
        let secrets = SessionFiles::at(folder.clone());
        let account = AccountId::new("github", "github.com:8443", "1").unwrap();
        assert_eq!(secrets.read(&account).unwrap(), None);
        secrets.write(&account, "octo", &Token::new("gho_1").unwrap()).unwrap();
        let read = secrets.read(&account).unwrap();
        secrets.erase(&account).unwrap();
        let erased = secrets.read(&account).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read, Token::new("gho_1"));
        assert_eq!(erased, None);
    }

    #[cfg(unix)]
    #[test]
    fn only_the_user_can_read_the_token_files() {
        use std::os::unix::fs::PermissionsExt as _;

        let folder = std::env::temp_dir().join(format!("fastgit-private-{}", std::process::id()));
        let secrets = SessionFiles::at(folder.clone());
        let account = AccountId::new("github", "github.com", "1").unwrap();
        secrets.write(&account, "octo", &Token::new("gho_1").unwrap()).unwrap();
        let folder_mode = std::fs::metadata(&folder).unwrap().permissions().mode() & 0o777;
        let file_mode = std::fs::metadata(secrets.file(&account).unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!((folder_mode, file_mode), (0o700, 0o600));
    }
}
