use std::io;

use windows::{
    Win32::{
        Foundation::ERROR_NOT_FOUND,
        Security::Credentials::{
            CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
            CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW,
        },
    },
    core::{PCWSTR, PWSTR},
};

use super::{Secrets, Token};
use crate::accounts::AccountId;

const TARGET_PREFIX: &str = "SterneGit/";

pub(crate) struct CredentialManager;

impl Secrets for CredentialManager {
    fn read(&self, account: &AccountId) -> io::Result<Option<Token>> {
        let target = wide(&target_of(account));
        let mut found: *mut CREDENTIALW = std::ptr::null_mut();
        if let Err(error) =
            unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut found) }
        {
            return if is_not_found(&error) {
                Ok(None)
            } else {
                Err(io::Error::other(error))
            };
        }
        let blob = unsafe {
            let credential = &*found;
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
            .to_vec()
        };
        unsafe { CredFree(found.cast()) };
        Ok(String::from_utf8(blob).ok().and_then(|text| Token::new(&text)))
    }

    fn write(&self, account: &AccountId, login: &str, token: &Token) -> io::Result<()> {
        let mut blob = token.reveal().as_bytes().to_vec();
        if blob.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the token is too long for Windows Credential Manager",
            ));
        }
        let mut target = wide(&target_of(account));
        let mut user = wide(login);
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_mut_ptr()),
            UserName: PWSTR(user.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        unsafe { CredWriteW(&credential, 0) }.map_err(io::Error::other)
    }

    fn erase(&self, account: &AccountId) -> io::Result<()> {
        let target = wide(&target_of(account));
        match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
            Err(error) if !is_not_found(&error) => Err(io::Error::other(error)),
            _ => Ok(()),
        }
    }
}

fn target_of(account: &AccountId) -> String {
    format!("{TARGET_PREFIX}{}", account.as_str())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn is_not_found(error: &windows::core::Error) -> bool {
    error.code() == ERROR_NOT_FOUND.to_hresult()
}

#[cfg(test)]
mod tests {
    use super::CredentialManager;
    use crate::accounts::{AccountId, Secrets as _, Token};

    #[test]
    #[ignore = "writes a SterneGit test entry to this machine's Windows Credential Manager"]
    fn a_token_stored_in_credential_manager_reads_back_until_erased() {
        let account = AccountId::new("sternegit-test", "localhost", "1").unwrap();
        let token = Token::new("sternegit-test-token").unwrap();
        CredentialManager.write(&account, "sternegit-test", &token).unwrap();
        let read = CredentialManager.read(&account).unwrap();
        CredentialManager.erase(&account).unwrap();
        assert_eq!(read, Some(token));
        assert_eq!(CredentialManager.read(&account).unwrap(), None);
    }
}
