mod abi;
mod account;
mod cassette;
mod envelope;
mod failure;
pub mod ops;
mod pulls;
mod repositories;
mod review;
mod ssh;

pub use abi::{ABI, RequestHead, ResponseHead, resolve};
pub use account::{AccountContext, Identity};
pub use cassette::{Cassette, Exchange, Reel};
pub use envelope::{Answer, Call, Done, Hello};
pub use failure::{Failure, FieldError};
pub use pulls::{
    BranchQuery, Check, CheckState, CommitQuery, ConversationQuery, FetchSpec, Merge, MergeMethod,
    Mergeability, Merged, NewComment, NewPull, Person, PullDetails, PullNumber, PullState,
    PullSummary, PullsQuery, ReviewStanding, StateFilter, Tip, Verdict,
};
pub use repositories::{
    CloneUrls, Cursor, NewRepository, Owner, OwnerKind, Page, RepoRef, Repository,
    RepositoryQuery, Timestamp, Visibility,
};
pub use review::{Anchor, Comment, Draft, Line, NewReply, Resolution, Review, Spot, Thread};
pub use ssh::{NewSshKey, SshKey};

pub const PROTOCOL: &str = "1.0";

pub fn major(version: &str) -> Option<u32> {
    version.split('.').next()?.parse().ok()
}

pub fn speaks_same_major(version: &str) -> bool {
    major(version).is_some() && major(version) == major(PROTOCOL)
}
