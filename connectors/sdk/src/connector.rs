use sternegit_protocol::{
    AccountContext, BranchQuery, Check, Comment, CommitQuery, ConversationQuery, Failure,
    Identity, Merge, Merged, NewComment, NewPull, NewReply, NewRepository, NewSshKey, Owner, Page,
    PullDetails, PullNumber, PullSummary, PullsQuery, RepoRef, Repository, RepositoryQuery,
    Resolution, Review, SshKey, Thread,
};

pub trait Connector {
    fn who_am_i(&self, _account: &AccountContext) -> Result<Identity, Failure> {
        Err(Failure::Unsupported)
    }

    fn owners(&self, _account: &AccountContext) -> Result<Vec<Owner>, Failure> {
        Err(Failure::Unsupported)
    }

    fn repositories(
        &self,
        _account: &AccountContext,
        _query: RepositoryQuery,
    ) -> Result<Page<Repository>, Failure> {
        Err(Failure::Unsupported)
    }

    fn repository(&self, _account: &AccountContext, _repo: &RepoRef) -> Result<Repository, Failure> {
        Err(Failure::Unsupported)
    }

    fn create_repository(
        &self,
        _account: &AccountContext,
        _new: NewRepository,
    ) -> Result<Repository, Failure> {
        Err(Failure::Unsupported)
    }

    fn ssh_keys(&self, _account: &AccountContext) -> Result<Vec<SshKey>, Failure> {
        Err(Failure::Unsupported)
    }

    fn add_ssh_key(&self, _account: &AccountContext, _new: NewSshKey) -> Result<SshKey, Failure> {
        Err(Failure::Unsupported)
    }

    fn host_keys(&self, _account: &AccountContext) -> Result<Vec<String>, Failure> {
        Err(Failure::Unsupported)
    }

    fn pulls(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _query: PullsQuery,
    ) -> Result<Page<PullSummary>, Failure> {
        Err(Failure::Unsupported)
    }

    fn pulls_for_branch(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _query: BranchQuery,
    ) -> Result<Vec<PullSummary>, Failure> {
        Err(Failure::Unsupported)
    }

    fn pull(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _query: PullNumber,
    ) -> Result<PullDetails, Failure> {
        Err(Failure::Unsupported)
    }

    fn checks(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _query: CommitQuery,
    ) -> Result<Vec<Check>, Failure> {
        Err(Failure::Unsupported)
    }

    fn conversation(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _query: ConversationQuery,
    ) -> Result<Page<Comment>, Failure> {
        Err(Failure::Unsupported)
    }

    fn comment(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _new: NewComment,
    ) -> Result<Comment, Failure> {
        Err(Failure::Unsupported)
    }

    fn create_pull(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _new: NewPull,
    ) -> Result<PullSummary, Failure> {
        Err(Failure::Unsupported)
    }

    fn threads(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _query: PullNumber,
    ) -> Result<Vec<Thread>, Failure> {
        Err(Failure::Unsupported)
    }

    fn review(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _review: Review,
    ) -> Result<(), Failure> {
        Err(Failure::Unsupported)
    }

    fn reply(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _reply: NewReply,
    ) -> Result<Comment, Failure> {
        Err(Failure::Unsupported)
    }

    fn resolve(
        &self,
        _account: &AccountContext,
        _repo: &RepoRef,
        _resolution: Resolution,
    ) -> Result<Thread, Failure> {
        Err(Failure::Unsupported)
    }

    fn merge(&self, _account: &AccountContext, _repo: &RepoRef, _merge: Merge) -> Result<Merged, Failure> {
        Err(Failure::Unsupported)
    }
}
