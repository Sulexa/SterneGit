mod checks;
mod conversation;
mod graphql;
mod keys;
mod listing;
mod moments;
mod pulls;
mod refusal;
mod repositories;

use sternegit_sdk::{
    AccountContext, BranchQuery, Check, Comment, CommitQuery, Connector, ConversationQuery, Failure,
    Identity, Merge, Merged, NewComment, NewPull, NewRepository, NewSshKey, Owner, Page, PullDetails,
    PullNumber, PullSummary, PullsQuery, RepoRef, Repository, RepositoryQuery, Request, Response,
    SshKey, export,
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

    fn pulls(
        &self,
        account: &AccountContext,
        repo: &RepoRef,
        query: PullsQuery,
    ) -> Result<Page<PullSummary>, Failure> {
        pulls::list(account, repo, query)
    }

    fn pulls_for_branch(
        &self,
        _account: &AccountContext,
        repo: &RepoRef,
        query: BranchQuery,
    ) -> Result<Vec<PullSummary>, Failure> {
        pulls::for_branch(repo, query)
    }

    fn pull(&self, account: &AccountContext, repo: &RepoRef, query: PullNumber) -> Result<PullDetails, Failure> {
        pulls::get(account, repo, query)
    }

    fn checks(&self, _account: &AccountContext, repo: &RepoRef, query: CommitQuery) -> Result<Vec<Check>, Failure> {
        checks::list(repo, query)
    }

    fn conversation(
        &self,
        account: &AccountContext,
        repo: &RepoRef,
        query: ConversationQuery,
    ) -> Result<Page<Comment>, Failure> {
        conversation::list(account, repo, query)
    }

    fn comment(&self, _account: &AccountContext, repo: &RepoRef, new: NewComment) -> Result<Comment, Failure> {
        conversation::add(repo, new)
    }

    fn create_pull(&self, _account: &AccountContext, repo: &RepoRef, new: NewPull) -> Result<PullSummary, Failure> {
        pulls::create(repo, new)
    }

    fn merge(&self, _account: &AccountContext, repo: &RepoRef, merge: Merge) -> Result<Merged, Failure> {
        pulls::merge(repo, merge)
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
