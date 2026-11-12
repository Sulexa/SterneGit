mod checks;
mod conversation;
mod moments;
mod place;
mod pulls;
mod refusal;
mod repositories;

use sternegit_sdk::{
    AccountContext, BranchQuery, Check, Comment, CommitQuery, Connector, ConversationQuery, Failure,
    Identity, Merge, Merged, NewComment, NewPull, NewRepository, Owner, Page, PullDetails,
    PullNumber, PullSummary, PullsQuery, RepoRef, Repository, RepositoryQuery, Request, export,
};
use serde_json::Value;

use place::Collection;
use refusal::succeeded;

const UNAUTHENTICATED: [&str; 2] = ["Anonymous", "Unauthenticated"];

pub struct AzureDevOps;

impl Connector for AzureDevOps {
    fn who_am_i(&self, account: &AccountContext) -> Result<Identity, Failure> {
        let collection = Collection::of(account)?;
        let data: Value = succeeded(api(Request::get("_apis/connectionData")).fetch())?.json()?;
        let user = &data["authenticatedUser"];
        let descriptor = user["descriptor"].as_str().unwrap_or_default();
        if UNAUTHENTICATED.iter().any(|word| descriptor.contains(word)) {
            return Err(Failure::SignedOut);
        }
        let text = |value: &Value| value.as_str().filter(|text| !text.trim().is_empty()).map(str::to_owned);
        let id = text(&user["id"]).ok_or_else(|| Failure::Protocol {
            message: "Azure DevOps named no user for this token".to_owned(),
        })?;
        let name = text(&user["providerDisplayName"]);
        let login = text(&user["properties"]["Account"]["$value"])
            .or_else(|| name.clone())
            .unwrap_or_else(|| id.clone());
        Ok(Identity {
            id: collection.user_id(&id),
            login,
            name,
            avatar_url: None,
            scopes: Vec::new(),
        })
    }

    fn owners(&self, account: &AccountContext) -> Result<Vec<Owner>, Failure> {
        repositories::owners(&Collection::of(account)?)
    }

    fn repositories(&self, account: &AccountContext, query: RepositoryQuery) -> Result<Page<Repository>, Failure> {
        repositories::list(&Collection::of(account)?, query)
    }

    fn repository(&self, account: &AccountContext, repo: &RepoRef) -> Result<Repository, Failure> {
        repositories::get(&Collection::of(account)?, repo)
    }

    fn create_repository(&self, account: &AccountContext, new: NewRepository) -> Result<Repository, Failure> {
        repositories::create(&Collection::of(account)?, new)
    }

    fn pulls(&self, account: &AccountContext, repo: &RepoRef, query: PullsQuery) -> Result<Page<PullSummary>, Failure> {
        pulls::list(&Collection::of(account)?, repo, query)
    }

    fn pulls_for_branch(
        &self,
        account: &AccountContext,
        repo: &RepoRef,
        query: BranchQuery,
    ) -> Result<Vec<PullSummary>, Failure> {
        pulls::for_branch(&Collection::of(account)?, repo, query)
    }

    fn pull(&self, account: &AccountContext, repo: &RepoRef, query: PullNumber) -> Result<PullDetails, Failure> {
        pulls::get(&Collection::of(account)?, repo, query)
    }

    fn checks(&self, account: &AccountContext, repo: &RepoRef, query: CommitQuery) -> Result<Vec<Check>, Failure> {
        checks::list(&Collection::of(account)?, repo, query)
    }

    fn conversation(
        &self,
        account: &AccountContext,
        repo: &RepoRef,
        query: ConversationQuery,
    ) -> Result<Page<Comment>, Failure> {
        conversation::list(&Collection::of(account)?, repo, query)
    }

    fn comment(&self, account: &AccountContext, repo: &RepoRef, new: NewComment) -> Result<Comment, Failure> {
        conversation::add(&Collection::of(account)?, repo, new)
    }

    fn create_pull(&self, account: &AccountContext, repo: &RepoRef, new: NewPull) -> Result<PullSummary, Failure> {
        pulls::create(&Collection::of(account)?, repo, new)
    }

    fn merge(&self, account: &AccountContext, repo: &RepoRef, merge: Merge) -> Result<Merged, Failure> {
        pulls::merge(&Collection::of(account)?, repo, merge)
    }
}

export!(AzureDevOps);

pub(crate) fn api(request: Request) -> Request {
    request
        .header("Accept", "application/json")
        .header("X-TFS-FedAuthRedirect", "Suppress")
}
