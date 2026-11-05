use serde::{Serialize, de::DeserializeOwned};

use crate::{
    BranchQuery, Check, Comment, CommitQuery, ConversationQuery, Done, Hello, Identity, Merge,
    Merged, NewComment, NewPull, NewReply, NewRepository, NewSshKey, Owner, Page, PullDetails,
    PullNumber, PullSummary, PullsQuery, Repository, RepositoryQuery, Resolution, Review, SshKey,
    Thread,
};

pub trait Op {
    const NAME: &'static str;
    const ON_REPOSITORY: bool;
    type Input: Serialize + DeserializeOwned + Send + 'static;
    type Output: Serialize + DeserializeOwned + Send + 'static;
}

pub trait Visit {
    type Result;

    fn visit<O: Op>(self) -> Self::Result;
}

macro_rules! ops {
    (@on account) => {
        false
    };
    (@on repository) => {
        true
    };
    ($($op:ident $name:literal $on:ident $input:ty => $output:ty;)*) => {
        $(
            pub struct $op;

            impl Op for $op {
                const NAME: &'static str = $name;
                const ON_REPOSITORY: bool = ops!(@on $on);
                type Input = $input;
                type Output = $output;
            }
        )*

        pub const NAMES: &[&str] = &[$($name),*];

        pub fn visit<V: Visit>(name: &str, visitor: V) -> Option<V::Result> {
            match name {
                $($name => Some(visitor.visit::<$op>()),)*
                _ => None,
            }
        }
    };
}

ops! {
    Greet "hello" account Hello => Hello;
    WhoAmI "account.who_am_i" account () => Identity;
    ListOwners "repositories.owners" account () => Vec<Owner>;
    ListRepositories "repositories.list" account RepositoryQuery => Page<Repository>;
    GetRepository "repositories.get" repository () => Repository;
    CreateRepository "repositories.create" account NewRepository => Repository;
    ListSshKeys "ssh_keys.list" account () => Vec<SshKey>;
    AddSshKey "ssh_keys.add" account NewSshKey => SshKey;
    ListHostKeys "ssh_keys.host_keys" account () => Vec<String>;
    ListPulls "pulls.list" repository PullsQuery => Page<PullSummary>;
    ListPullsForBranch "pulls.for_branch" repository BranchQuery => Vec<PullSummary>;
    GetPull "pulls.get" repository PullNumber => PullDetails;
    ListChecks "pulls.checks" repository CommitQuery => Vec<Check>;
    ListConversation "pulls.conversation" repository ConversationQuery => Page<Comment>;
    AddComment "pulls.comment" repository NewComment => Comment;
    CreatePull "pulls.create" repository NewPull => PullSummary;
    ListThreads "pulls.threads" repository PullNumber => Vec<Thread>;
    SubmitReview "pulls.review" repository Review => Done;
    AddReply "pulls.reply" repository NewReply => Comment;
    SetResolved "pulls.resolve" repository Resolution => Thread;
    MergePull "pulls.merge" repository Merge => Merged;
}

pub fn named(name: &str) -> Option<&'static str> {
    NAMES.iter().copied().find(|known| *known == name)
}
