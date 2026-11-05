#[cfg(not(target_arch = "wasm32"))]
pub mod build;
mod connector;
mod dispatch;
mod host;
mod log;
#[cfg(not(target_arch = "wasm32"))]
mod replay;
mod request;

pub use connector::Connector;
pub use dispatch::dispatch;
#[cfg(target_arch = "wasm32")]
pub use dispatch::serve;
pub use fastgit_protocol as protocol;
pub use fastgit_protocol::{
    AccountContext, Anchor, BranchQuery, Check, CheckState, CloneUrls, Comment, CommitQuery,
    ConversationQuery, Cursor, Draft, FetchSpec, Failure, FieldError, Identity, Line, Merge,
    MergeMethod, Mergeability, Merged, NewComment, NewPull, NewReply, NewRepository, NewSshKey,
    Owner, OwnerKind, Page, Person, PullDetails, PullNumber, PullState, PullSummary, PullsQuery,
    RepoRef, Repository, RepositoryQuery, Resolution, Review, ReviewStanding, Spot, SshKey,
    StateFilter, Thread, Timestamp, Tip, Verdict, Visibility,
};
pub use log::{Level, log, now};
#[cfg(not(target_arch = "wasm32"))]
pub use replay::{Replayed, check, replay};
pub use request::{Request, Response, Sent};

pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_export]
macro_rules! export {
    ($connector:expr) => {
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn fastgit_call(input_len: i32) -> i32 {
            $crate::serve(&$connector, input_len)
        }

        #[cfg(not(target_arch = "wasm32"))]
        const _: fn() = || {
            let _: &dyn $crate::Connector = &$connector;
        };
    };
}
