use fastgit_protocol::{
    AccountContext, Answer, Call, Done, Failure, Hello, PROTOCOL, RepoRef,
    ops::{
        AddComment, AddReply, AddSshKey, CreatePull, CreateRepository, GetPull, GetRepository,
        Greet, ListChecks, ListConversation, ListHostKeys, ListOwners, ListPulls,
        ListPullsForBranch, ListRepositories, ListSshKeys, ListThreads, MergePull, Op, SetResolved,
        SubmitReview, WhoAmI,
    },
};
use serde::Serialize;
use serde_json::Value;

use crate::{Connector, SDK_VERSION};

pub fn dispatch(connector: &impl Connector, input: &[u8]) -> Vec<u8> {
    match serde_json::from_slice::<Call<Value>>(input) {
        Ok(call) => route(connector, call),
        Err(error) => answer::<Done>(Err(misunderstood(format!(
            "FastGit's call can't be read: {error}"
        )))),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn serve(connector: &impl Connector, input_len: i32) -> i32 {
    crate::log::report_panics();
    let input = crate::host::take(input_len as u32 as usize);
    let output = dispatch(connector, &input);
    crate::host::finish(&output);
    0
}

struct Route {
    account: AccountContext,
    repo: Option<RepoRef>,
    input: Value,
}

impl Route {
    fn on_account<O: Op>(
        self,
        run: impl FnOnce(&AccountContext, O::Input) -> Result<O::Output, Failure>,
    ) -> Vec<u8> {
        let Route { account, input, .. } = self;
        answer(decoded::<O>(input).and_then(|input| run(&account, input)))
    }

    fn on_repository<O: Op>(
        self,
        run: impl FnOnce(&AccountContext, &RepoRef, O::Input) -> Result<O::Output, Failure>,
    ) -> Vec<u8> {
        let Route {
            account,
            repo,
            input,
        } = self;
        answer(match repo {
            Some(repo) => decoded::<O>(input).and_then(|input| run(&account, &repo, input)),
            None => Err(misunderstood(format!("{} needs a repository", O::NAME))),
        })
    }
}

fn route(connector: &impl Connector, call: Call<Value>) -> Vec<u8> {
    let route = Route {
        account: call.account.unwrap_or_default(),
        repo: call.repo,
        input: call.input,
    };
    let c = connector;
    match call.op.as_str() {
        Greet::NAME => route.on_account::<Greet>(|_, _| Ok(hello())),
        WhoAmI::NAME => route.on_account::<WhoAmI>(|account, ()| c.who_am_i(account)),
        ListOwners::NAME => route.on_account::<ListOwners>(|account, ()| c.owners(account)),
        ListRepositories::NAME => {
            route.on_account::<ListRepositories>(|account, query| c.repositories(account, query))
        }
        GetRepository::NAME => {
            route.on_repository::<GetRepository>(|account, repo, ()| c.repository(account, repo))
        }
        CreateRepository::NAME => {
            route.on_account::<CreateRepository>(|account, new| c.create_repository(account, new))
        }
        ListSshKeys::NAME => route.on_account::<ListSshKeys>(|account, ()| c.ssh_keys(account)),
        AddSshKey::NAME => route.on_account::<AddSshKey>(|account, new| c.add_ssh_key(account, new)),
        ListHostKeys::NAME => route.on_account::<ListHostKeys>(|account, ()| c.host_keys(account)),
        ListPulls::NAME => {
            route.on_repository::<ListPulls>(|account, repo, query| c.pulls(account, repo, query))
        }
        ListPullsForBranch::NAME => route.on_repository::<ListPullsForBranch>(|account, repo, query| {
            c.pulls_for_branch(account, repo, query)
        }),
        GetPull::NAME => {
            route.on_repository::<GetPull>(|account, repo, query| c.pull(account, repo, query))
        }
        ListChecks::NAME => {
            route.on_repository::<ListChecks>(|account, repo, query| c.checks(account, repo, query))
        }
        ListConversation::NAME => route.on_repository::<ListConversation>(|account, repo, query| {
            c.conversation(account, repo, query)
        }),
        AddComment::NAME => {
            route.on_repository::<AddComment>(|account, repo, new| c.comment(account, repo, new))
        }
        CreatePull::NAME => {
            route.on_repository::<CreatePull>(|account, repo, new| c.create_pull(account, repo, new))
        }
        ListThreads::NAME => {
            route.on_repository::<ListThreads>(|account, repo, query| c.threads(account, repo, query))
        }
        SubmitReview::NAME => route.on_repository::<SubmitReview>(|account, repo, review| {
            c.review(account, repo, review).map(|()| Done {})
        }),
        AddReply::NAME => {
            route.on_repository::<AddReply>(|account, repo, reply| c.reply(account, repo, reply))
        }
        SetResolved::NAME => route.on_repository::<SetResolved>(|account, repo, resolution| {
            c.resolve(account, repo, resolution)
        }),
        MergePull::NAME => {
            route.on_repository::<MergePull>(|account, repo, merge| c.merge(account, repo, merge))
        }
        _ => answer::<Done>(Err(Failure::Unsupported)),
    }
}

fn hello() -> Hello {
    Hello {
        protocol: PROTOCOL.to_owned(),
        sdk: Some(SDK_VERSION.to_owned()),
    }
}

fn decoded<O: Op>(input: Value) -> Result<O::Input, Failure> {
    serde_json::from_value(input).map_err(|error| {
        misunderstood(format!("FastGit's input to {} can't be read: {error}", O::NAME))
    })
}

fn answer<T: Serialize>(result: Result<T, Failure>) -> Vec<u8> {
    serde_json::to_vec(&Answer::from(result)).unwrap_or_else(|error| {
        let failure = misunderstood(format!("the answer can't be written as JSON: {error}"));
        serde_json::to_vec(&Answer::<Done>::Err(failure)).unwrap_or_default()
    })
}

fn misunderstood(message: String) -> Failure {
    Failure::Protocol { message }
}
