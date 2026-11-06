use std::io;

use serde_json::{Value, json};

use super::{Account, AccountId, account::authority};
use crate::settings::{Folder, JsonFile};

const FILE_NAME: &str = "accounts.json";
const VERSION: u64 = 1;
const ACCOUNTS: &str = "accounts";
const REMOTE_OWNERS: &str = "remote_owners";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OwnerChoice {
    pub(crate) host: String,
    pub(crate) owner: String,
    pub(crate) account: AccountId,
}

impl OwnerChoice {
    fn from_json(value: &Value) -> Option<Self> {
        let text = |name: &str| value.get(name)?.as_str();
        Some(Self {
            host: text("host")?.to_owned(),
            owner: text("owner")?.trim_matches('/').to_owned(),
            account: AccountId::parse(text("account")?)?,
        })
    }

    fn to_json(&self) -> Value {
        json!({ "host": self.host, "owner": self.owner, "account": self.account.as_str() })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccountBook {
    pub(crate) accounts: Vec<Account>,
    pub(crate) owners: Vec<OwnerChoice>,
    unreadable_accounts: Vec<Value>,
    unreadable_owners: Vec<Value>,
}

impl AccountBook {
    pub(crate) fn load() -> Self {
        file().map_or_else(Self::default, |file| Self::read_from(&file))
    }

    pub(crate) fn read_from(file: &JsonFile) -> Self {
        let fields = file.read();
        let listed = |name: &str| {
            fields
                .get(name)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let (accounts, unreadable_accounts) = read_each(listed(ACCOUNTS), Account::from_json);
        let (owners, unreadable_owners) = read_each(listed(REMOTE_OWNERS), OwnerChoice::from_json);
        Self {
            accounts,
            owners,
            unreadable_accounts,
            unreadable_owners,
        }
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        let file = file().ok_or_else(|| io::Error::other("There is no folder for FastGit's settings"))?;
        self.write_to(&file)
    }

    pub(crate) fn write_to(&self, file: &JsonFile) -> io::Result<()> {
        let accounts = self.accounts.iter().map(Account::to_json);
        let owners = self.owners.iter().map(OwnerChoice::to_json);
        file.update(|fields| {
            fields.insert("version".into(), VERSION.into());
            fields.insert(
                ACCOUNTS.into(),
                accounts.chain(self.unreadable_accounts.iter().cloned()).collect(),
            );
            fields.insert(
                REMOTE_OWNERS.into(),
                owners.chain(self.unreadable_owners.iter().cloned()).collect(),
            );
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub(crate) fn account(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.iter().find(|account| &account.id == id)
    }

    pub(crate) fn on_host<'a>(
        &'a self,
        scheme: &'a str,
        host: &'a str,
    ) -> impl Iterator<Item = &'a Account> + 'a {
        self.accounts.iter().filter(move |account| {
            account.server.scheme() == scheme
                && authority(&account.server).is_some_and(|server| server.eq_ignore_ascii_case(host))
        })
    }

    pub(crate) fn put(&mut self, account: Account) {
        match self.accounts.iter_mut().find(|known| known.id == account.id) {
            Some(known) => *known = account,
            None => self.accounts.push(account),
        }
    }

    pub(crate) fn sign_in(&mut self, mut account: Account) {
        if let Some(known) = self.account(&account.id) {
            account.ssh_key = known.ssh_key.clone();
            account.ssh_repositories = known.ssh_repositories.clone();
            account.clone_over_ssh = known.clone_over_ssh;
            let mut others = known.others.clone();
            others.extend(account.others);
            account.others = others;
        }
        self.put(account);
    }

    pub(crate) fn remove(&mut self, id: &AccountId) {
        self.accounts.retain(|account| &account.id != id);
        self.owners.retain(|choice| &choice.account != id);
    }

    pub(crate) fn choose(&mut self, host: &str, owner: &str, account: AccountId) {
        let owner = owner.trim_matches('/');
        self.owners
            .retain(|choice| !(choice.host.eq_ignore_ascii_case(host) && choice.owner == owner));
        self.owners.push(OwnerChoice {
            host: host.to_owned(),
            owner: owner.to_owned(),
            account,
        });
    }
}

fn read_each<T>(values: Vec<Value>, read: impl Fn(&Value) -> Option<T>) -> (Vec<T>, Vec<Value>) {
    let mut read_ones = Vec::new();
    let mut unreadable = Vec::new();
    for value in values {
        match read(&value) {
            Some(one) => read_ones.push(one),
            None => unreadable.push(value),
        }
    }
    (read_ones, unreadable)
}

fn file() -> Option<JsonFile> {
    JsonFile::in_folder(Folder::Config, FILE_NAME)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{AccountBook, OwnerChoice};
    use crate::{accounts::account::fixtures, settings::JsonFile};

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("fastgit-book-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn a_book_reads_back_its_accounts_and_owner_choices() {
        let folder = scratch("round-trip");
        let file = JsonFile::at(folder.join("accounts.json"));
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        book.put(fixtures::github("2"));
        book.choose("github.com", "/fastgit-org/", fixtures::github("2").id);
        book.write_to(&file).unwrap();
        let read = AccountBook::read_from(&file);
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(read, book);
        assert_eq!(
            read.owners,
            [OwnerChoice {
                host: "github.com".into(),
                owner: "fastgit-org".into(),
                account: fixtures::github("2").id,
            }]
        );
    }

    #[test]
    fn putting_an_account_again_replaces_it() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        let mut renamed = fixtures::github("1");
        renamed.login = "new-login".into();
        book.put(renamed.clone());
        assert_eq!(book.accounts, [renamed]);
    }

    #[test]
    fn signing_in_again_keeps_what_fastgit_set_up_for_the_account() {
        let mut book = AccountBook::default();
        let mut known = fixtures::github("1");
        known.ssh_key = Some("C:/keys/github-github.com-1_ed25519".into());
        known.ssh_repositories = vec!["E:/Projects/fastgit".into()];
        known.clone_over_ssh = true;
        book.put(known.clone());
        let mut again = fixtures::github("1");
        again.scopes = vec!["repo".into(), "write:public_key".into()];
        book.sign_in(again);
        let kept = book.account(&known.id).unwrap();
        assert_eq!(kept.ssh_key, known.ssh_key);
        assert_eq!(kept.ssh_repositories, known.ssh_repositories);
        assert!(kept.clone_over_ssh);
        assert_eq!(kept.scopes, ["repo", "write:public_key"]);
    }

    #[test]
    fn removing_an_account_forgets_the_owners_chosen_for_it() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        book.put(fixtures::github("2"));
        book.choose("github.com", "a", fixtures::github("1").id);
        book.choose("github.com", "b", fixtures::github("2").id);
        book.remove(&fixtures::github("1").id);
        assert_eq!(book.accounts, [fixtures::github("2")]);
        assert_eq!(book.owners.len(), 1);
        assert_eq!(book.owners[0].owner, "b");
    }

    #[test]
    fn choosing_again_for_an_owner_replaces_the_earlier_choice() {
        let mut book = AccountBook::default();
        book.choose("github.com", "a", fixtures::github("1").id);
        book.choose("GitHub.com", "a", fixtures::github("2").id);
        assert_eq!(book.owners.len(), 1);
        assert_eq!(book.owners[0].account, fixtures::github("2").id);
    }

    #[test]
    fn entries_a_newer_fastgit_wrote_survive_a_rewrite() {
        let folder = scratch("unreadable");
        let file = JsonFile::at(folder.join("accounts.json"));
        let strange = json!({ "id": "something-else", "shape": "new" });
        file.write(
            json!({ "version": 2, "accounts": [strange.clone()], "remote_owners": [] })
                .as_object()
                .unwrap(),
        )
        .unwrap();
        let mut book = AccountBook::read_from(&file);
        book.put(fixtures::github("1"));
        book.write_to(&file).unwrap();
        let written = file.read();
        let _ = std::fs::remove_dir_all(&folder);
        let accounts = written["accounts"].as_array().unwrap();
        assert_eq!(accounts.len(), 2);
        assert!(accounts.contains(&strange));
    }

    #[test]
    fn accounts_on_a_host_match_its_scheme_and_port() {
        let mut book = AccountBook::default();
        book.put(fixtures::github("1"));
        book.put(fixtures::account("https://ghe.example.com:8443", "2", &[]));
        assert_eq!(book.on_host("https", "github.com").count(), 1);
        assert_eq!(book.on_host("https", "ghe.example.com:8443").count(), 1);
        assert_eq!(book.on_host("https", "ghe.example.com").count(), 0);
        assert_eq!(book.on_host("http", "github.com").count(), 0);
    }
}
