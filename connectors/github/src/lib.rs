mod keys;
mod listing;
mod refusal;
mod repositories;

use fastgit_sdk::{
    AccountContext, Connector, Failure, Identity, NewRepository, NewSshKey, Owner, Page, RepoRef,
    Repository, RepositoryQuery, Request, Response, SshKey, export,
};
use serde_json::Value;

use listing::id_of;
use refusal::succeeded;

const MEDIA_TYPE: &str = "application/vnd.github+json";
const API_VERSION: &str = "2022-11-28";
const GRANTED_SCOPES: &str = "x-oauth-scopes";

pub struct GitHub;

impl Connector for GitHub {
    fn who_am_i(&self, _account: &AccountContext) -> Result<Identity, Failure> {
        let response = succeeded(api(Request::get("user")).fetch())?;
        let user: Value = response.json()?;
        let text = |name: &str| user[name].as_str().map(str::to_owned);
        let unnamed = || Failure::Protocol {
            message: "GitHub named no user for this token".to_owned(),
        };
        Ok(Identity {
            id: id_of(&user["id"]).ok_or_else(unnamed)?,
            login: text("login").ok_or_else(unnamed)?,
            name: text("name"),
            avatar_url: text("avatar_url"),
            scopes: granted_scopes(&response),
        })
    }

    fn owners(&self, account: &AccountContext) -> Result<Vec<Owner>, Failure> {
        repositories::owners(account)
    }

    fn repositories(
        &self,
        account: &AccountContext,
        query: RepositoryQuery,
    ) -> Result<Page<Repository>, Failure> {
        repositories::list(account, query)
    }

    fn repository(&self, _account: &AccountContext, repo: &RepoRef) -> Result<Repository, Failure> {
        repositories::get(repo)
    }

    fn create_repository(
        &self,
        account: &AccountContext,
        new: NewRepository,
    ) -> Result<Repository, Failure> {
        repositories::create(account, new)
    }

    fn ssh_keys(&self, account: &AccountContext) -> Result<Vec<SshKey>, Failure> {
        keys::list(account)
    }

    fn add_ssh_key(&self, _account: &AccountContext, new: NewSshKey) -> Result<SshKey, Failure> {
        keys::add(new)
    }

    fn host_keys(&self, _account: &AccountContext) -> Result<Vec<String>, Failure> {
        keys::host_keys()
    }
}

export!(GitHub);

pub(crate) fn api(request: Request) -> Request {
    request
        .header("Accept", MEDIA_TYPE)
        .header("X-GitHub-Api-Version", API_VERSION)
}

fn granted_scopes(response: &Response) -> Vec<String> {
    response
        .header(GRANTED_SCOPES)
        .map(|scopes| {
            scopes
                .split(',')
                .map(str::trim)
                .filter(|scope| !scope.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
