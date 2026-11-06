use std::{fmt, sync::Arc, time::Duration};

use fastgit_protocol::{
    AccountContext, Answer, BranchQuery, Call, Check, Comment, CommitQuery, ConversationQuery,
    Cursor, Failure, Identity, Merge, Merged, NewComment, NewPull, NewReply, NewRepository,
    NewSshKey, Owner, Page, PullDetails, PullNumber, PullSummary, PullsQuery, RepoRef, Repository,
    RepositoryQuery, Resolution, Review, SshKey, StateFilter, Thread,
    ops::{
        self, AddComment, AddReply, AddSshKey, CreatePull, CreateRepository, GetPull,
        GetRepository, ListChecks, ListConversation, ListHostKeys, ListOwners, ListPulls,
        ListPullsForBranch, ListRepositories, ListSshKeys, ListThreads, MergePull, Op, SetResolved,
        SubmitReview, Visit, WhoAmI,
    },
};
use serde::{Serialize, de::DeserializeOwned};

use super::{Budget, Outcome, Started, Stop, Usage};
use crate::{
    accounts::AccountId,
    net::{
        request::NetError,
        services::{AccountView, HostServices, cancel_pair},
    },
};

const OPS_THAT_PAGE_THROUGH_LISTS: [&str; 6] = [
    ListOwners::NAME,
    ListSshKeys::NAME,
    ListHostKeys::NAME,
    ListPullsForBranch::NAME,
    ListChecks::NAME,
    ListThreads::NAME,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CallError {
    Failed(Failure),
    Net(NetError),
    Stopped(Stop),
    Unavailable,
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::Failed(failure) => failure.fmt(f),
            CallError::Net(error) => error.fmt(f),
            CallError::Stopped(stop) => stop.fmt(f),
            CallError::Unavailable => f.write_str("Not available"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct Forge {
    started: Arc<Started>,
    host: Arc<dyn HostServices>,
    account: AccountId,
    repo: Option<RepoRef>,
}

impl Forge {
    pub(crate) fn new(
        started: Arc<Started>,
        host: Arc<dyn HostServices>,
        account: AccountId,
        repo: Option<RepoRef>,
    ) -> Self {
        Self {
            started,
            host,
            account,
            repo,
        }
    }

    pub(crate) fn account(&self) -> &AccountId {
        &self.account
    }

    pub(crate) fn exchange<I: Serialize>(
        &self,
        op: &'static str,
        on_repository: bool,
        input: &I,
    ) -> impl Future<Output = Outcome> + Send + use<I> {
        let prepared = self.prepare(op, on_repository, input);
        let started = Arc::clone(&self.started);
        let host = Arc::clone(&self.host);
        async move {
            match prepared {
                Ok((account, message)) => {
                    let (_cancel_on_drop, cancel) = cancel_pair();
                    let call = started.call(host, Some(account), message, budget_for(op), cancel);
                    call.await
                }
                Err(error) => Outcome {
                    result: Err(error),
                    usage: Usage::default(),
                },
            }
        }
    }

    fn prepare(
        &self,
        op: &'static str,
        on_repository: bool,
        input: &impl Serialize,
    ) -> Result<(AccountView, Vec<u8>), CallError> {
        let account = self
            .host
            .account(&self.account)
            .ok_or(CallError::Net(NetError::NotSignedIn))?;
        let repo = match (on_repository, &self.repo) {
            (false, _) => None,
            (true, Some(repo)) => Some(repo.clone()),
            (true, None) => return Err(CallError::Unavailable),
        };
        let message = envelope(op, Some(context(&account)), repo, input);
        Ok((account, message))
    }
}

#[cfg_attr(not(test), expect(dead_code))]
impl Forge {
    pub(crate) fn who_am_i(&self) -> impl Future<Output = Result<Identity, CallError>> + Send + use<> {
        self.ask::<WhoAmI>(())
    }

    pub(crate) fn owners(&self) -> impl Future<Output = Result<Vec<Owner>, CallError>> + Send + use<> {
        self.ask::<ListOwners>(())
    }

    pub(crate) fn repositories(
        &self,
        query: RepositoryQuery,
    ) -> impl Future<Output = Result<Page<Repository>, CallError>> + Send + use<> {
        self.ask::<ListRepositories>(query)
    }

    pub(crate) fn repository(&self) -> impl Future<Output = Result<Repository, CallError>> + Send + use<> {
        self.ask::<GetRepository>(())
    }

    pub(crate) fn create_repository(
        &self,
        new: NewRepository,
    ) -> impl Future<Output = Result<Repository, CallError>> + Send + use<> {
        self.ask::<CreateRepository>(new)
    }

    pub(crate) fn ssh_keys(&self) -> impl Future<Output = Result<Vec<SshKey>, CallError>> + Send + use<> {
        self.ask::<ListSshKeys>(())
    }

    pub(crate) fn add_ssh_key(
        &self,
        new: NewSshKey,
    ) -> impl Future<Output = Result<SshKey, CallError>> + Send + use<> {
        self.ask::<AddSshKey>(new)
    }

    pub(crate) fn host_keys(&self) -> impl Future<Output = Result<Vec<String>, CallError>> + Send + use<> {
        self.ask::<ListHostKeys>(())
    }

    pub(crate) fn pulls(
        &self,
        state: StateFilter,
        cursor: Option<Cursor>,
    ) -> impl Future<Output = Result<Page<PullSummary>, CallError>> + Send + use<> {
        self.ask::<ListPulls>(PullsQuery { state, cursor })
    }

    pub(crate) fn pulls_for_branch(
        &self,
        branch: String,
    ) -> impl Future<Output = Result<Vec<PullSummary>, CallError>> + Send + use<> {
        self.ask::<ListPullsForBranch>(BranchQuery { branch })
    }

    pub(crate) fn pull(&self, number: u64) -> impl Future<Output = Result<PullDetails, CallError>> + Send + use<> {
        self.ask::<GetPull>(PullNumber { number })
    }

    pub(crate) fn checks(&self, sha: String) -> impl Future<Output = Result<Vec<Check>, CallError>> + Send + use<> {
        self.ask::<ListChecks>(CommitQuery { sha })
    }

    pub(crate) fn conversation(
        &self,
        number: u64,
        cursor: Option<Cursor>,
    ) -> impl Future<Output = Result<Page<Comment>, CallError>> + Send + use<> {
        self.ask::<ListConversation>(ConversationQuery { number, cursor })
    }

    pub(crate) fn comment(
        &self,
        number: u64,
        body: String,
    ) -> impl Future<Output = Result<Comment, CallError>> + Send + use<> {
        self.ask::<AddComment>(NewComment { number, body })
    }

    pub(crate) fn create_pull(&self, new: NewPull) -> impl Future<Output = Result<PullSummary, CallError>> + Send + use<> {
        self.ask::<CreatePull>(new)
    }

    pub(crate) fn threads(&self, number: u64) -> impl Future<Output = Result<Vec<Thread>, CallError>> + Send + use<> {
        self.ask::<ListThreads>(PullNumber { number })
    }

    pub(crate) fn review(&self, review: Review) -> impl Future<Output = Result<(), CallError>> + Send + use<> {
        let submitted = self.ask::<SubmitReview>(review);
        async move { submitted.await.map(drop) }
    }

    pub(crate) fn reply(
        &self,
        number: u64,
        thread: String,
        body: String,
    ) -> impl Future<Output = Result<Comment, CallError>> + Send + use<> {
        self.ask::<AddReply>(NewReply { number, thread, body })
    }

    pub(crate) fn resolve(
        &self,
        number: u64,
        thread: String,
        resolved: bool,
    ) -> impl Future<Output = Result<Thread, CallError>> + Send + use<> {
        self.ask::<SetResolved>(Resolution { number, thread, resolved })
    }

    pub(crate) fn merge(&self, merge: Merge) -> impl Future<Output = Result<Merged, CallError>> + Send + use<> {
        self.ask::<MergePull>(merge)
    }

    fn ask<O: Op>(&self, input: O::Input) -> impl Future<Output = Result<O::Output, CallError>> + Send + use<O> {
        let exchange = self.exchange(O::NAME, O::ON_REPOSITORY, &input);
        let host = Arc::clone(&self.host);
        let account = self.account.clone();
        async move {
            let answer = exchange
                .await
                .result
                .and_then(|output| decode::<O::Output>(O::NAME, &output));
            if let Err(CallError::Failed(Failure::RateLimited { retry_after_seconds })) = &answer {
                host.pause(&account, host.now() + Duration::from_secs(*retry_after_seconds));
            }
            answer
        }
    }
}

pub(crate) fn budget_for(op: &str) -> Budget {
    if OPS_THAT_PAGE_THROUGH_LISTS.contains(&op) {
        Budget::LISTING
    } else {
        Budget::CALL
    }
}

pub(super) fn context(account: &AccountView) -> AccountContext {
    AccountContext {
        id: account.id.as_str().to_owned(),
        login: account.login.clone(),
        endpoints: account
            .endpoints
            .iter()
            .map(|(name, url)| (name.clone(), url.to_string()))
            .collect(),
    }
}

pub(super) fn envelope(
    op: &str,
    account: Option<AccountContext>,
    repo: Option<RepoRef>,
    input: &impl Serialize,
) -> Vec<u8> {
    let call = Call {
        op: op.to_owned(),
        account,
        repo,
        input,
    };
    serde_json::to_vec(&call).unwrap_or_default()
}

pub(crate) fn decode<T: DeserializeOwned>(op: &'static str, output: &[u8]) -> Result<T, CallError> {
    let mut deserializer = serde_json::Deserializer::from_slice(output);
    let answer: Answer<T> = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| bad_output(op, &error.path().to_string(), error.inner()))?;
    deserializer
        .end()
        .map_err(|error| bad_output(op, ".", &error))?;
    answer.into_result().map_err(|failure| match failure {
        Failure::Unsupported => CallError::Unavailable,
        failure => CallError::Failed(failure),
    })
}

pub(crate) fn check_answer(op: &str, output: &[u8]) -> Result<(), CallError> {
    struct Checked<'a>(&'a [u8]);

    impl Visit for Checked<'_> {
        type Result = Result<(), CallError>;

        fn visit<O: Op>(self) -> Self::Result {
            decode::<O::Output>(O::NAME, self.0).map(drop)
        }
    }

    ops::visit(op, Checked(output)).unwrap_or(Err(CallError::Unavailable))
}

fn bad_output(op: &'static str, path: &str, problem: &serde_json::Error) -> CallError {
    CallError::Stopped(Stop::BadOutput {
        op,
        path: inside_answer(path),
        problem: problem.to_string(),
    })
}

fn inside_answer(path: &str) -> String {
    match path.strip_prefix("ok") {
        Some("") => ".".to_owned(),
        Some(rest) if rest.starts_with('[') => rest.to_owned(),
        Some(rest) => rest.strip_prefix('.').unwrap_or(path).to_owned(),
        None => path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{Duration, UNIX_EPOCH},
    };

    use fastgit_protocol::{
        Done, Failure, Identity, Merge, MergeMethod, NewPull, NewRepository, NewSshKey, RepoRef,
        RepositoryQuery, Review, StateFilter, Verdict, Visibility,
    };
    use serde_json::{Value, json};

    use super::{CallError, Forge, budget_for, check_answer, decode, inside_answer};
    use crate::{
        accounts::AccountId,
        connect::{
            Budget, Started, Stop,
            fakes::{FakeHost, NOW_MILLIS, answering, fake_account, fake_connector, recorder},
        },
        net::request::NetError,
    };

    fn forge(wasm: &[u8], host: Arc<FakeHost>, repo: Option<RepoRef>) -> Forge {
        let started = Arc::new(Started::compile(fake_connector(), wasm).unwrap());
        Forge::new(started, host, fake_account().id, repo)
    }

    fn repo() -> RepoRef {
        RepoRef {
            owner: "group/sub".into(),
            name: "tool".into(),
        }
    }

    #[test]
    fn every_typed_call_sends_its_operation_input_and_repository() {
        let host = FakeHost::quiet();
        let forge = forge(&recorder(r#"{"err":{"kind":"not_found"}}"#), host.clone(), Some(repo()));
        let review = Review {
            number: 7,
            commit: "abc".into(),
            verdict: Verdict::Approve,
            body: "Fine".into(),
            drafts: Vec::new(),
        };
        let merge = Merge {
            number: 7,
            method: MergeMethod::Squash,
            title: None,
            message: None,
            expected_head: "abc".into(),
        };
        let new_repository = NewRepository {
            owner: "octo".into(),
            name: "new".into(),
            description: None,
            visibility: Visibility::Private,
            initialize: false,
        };
        let new_pull = NewPull {
            base: "main".into(),
            head: "topic".into(),
            head_owner: None,
            title: "Title".into(),
            body: "Body".into(),
            draft: true,
        };
        let query = RepositoryQuery {
            search: Some("fast".into()),
            ..RepositoryQuery::default()
        };
        let results = smol::block_on(async {
            vec![
                forge.who_am_i().await.map(drop),
                forge.owners().await.map(drop),
                forge.repositories(query).await.map(drop),
                forge.repository().await.map(drop),
                forge.create_repository(new_repository).await.map(drop),
                forge.ssh_keys().await.map(drop),
                forge.add_ssh_key(NewSshKey { title: "FastGit".into(), key: "ssh-ed25519 AAAA".into() }).await.map(drop),
                forge.host_keys().await.map(drop),
                forge.pulls(StateFilter::Open, Some("c2".into())).await.map(drop),
                forge.pulls_for_branch("topic".into()).await.map(drop),
                forge.pull(7).await.map(drop),
                forge.checks("abc".into()).await.map(drop),
                forge.conversation(7, None).await.map(drop),
                forge.comment(7, "Hi".into()).await.map(drop),
                forge.create_pull(new_pull).await.map(drop),
                forge.threads(7).await.map(drop),
                forge.review(review).await,
                forge.reply(7, "t1".into(), "Done".into()).await.map(drop),
                forge.resolve(7, "t1".into(), true).await.map(drop),
                forge.merge(merge).await.map(drop),
            ]
        });
        assert!(results.iter().all(|result| *result == Err(CallError::Failed(Failure::NotFound))));
        let sent: Vec<Value> = host
            .logs()
            .iter()
            .map(|line| {
                let call: Value = serde_json::from_str(line.strip_prefix("Info ").unwrap()).unwrap();
                assert_eq!(call["account"]["login"], "octo");
                json!([call["op"], call["repo"], call["input"]])
            })
            .collect();
        let repo = json!({ "owner": "group/sub", "name": "tool" });
        assert_eq!(
            sent,
            [
                json!(["account.who_am_i", null, null]),
                json!(["repositories.owners", null, null]),
                json!(["repositories.list", null, { "search": "fast", "owner": null, "cursor": null }]),
                json!(["repositories.get", repo, null]),
                json!(["repositories.create", null, { "owner": "octo", "name": "new", "description": null, "visibility": "private", "initialize": false }]),
                json!(["ssh_keys.list", null, null]),
                json!(["ssh_keys.add", null, { "title": "FastGit", "key": "ssh-ed25519 AAAA" }]),
                json!(["ssh_keys.host_keys", null, null]),
                json!(["pulls.list", repo, { "state": "open", "cursor": "c2" }]),
                json!(["pulls.for_branch", repo, { "branch": "topic" }]),
                json!(["pulls.get", repo, { "number": 7 }]),
                json!(["pulls.checks", repo, { "sha": "abc" }]),
                json!(["pulls.conversation", repo, { "number": 7, "cursor": null }]),
                json!(["pulls.comment", repo, { "number": 7, "body": "Hi" }]),
                json!(["pulls.create", repo, { "base": "main", "head": "topic", "head_owner": null, "title": "Title", "body": "Body", "draft": true }]),
                json!(["pulls.threads", repo, { "number": 7 }]),
                json!(["pulls.review", repo, { "number": 7, "commit": "abc", "verdict": "approve", "body": "Fine", "drafts": [] }]),
                json!(["pulls.reply", repo, { "number": 7, "thread": "t1", "body": "Done" }]),
                json!(["pulls.resolve", repo, { "number": 7, "thread": "t1", "resolved": true }]),
                json!(["pulls.merge", repo, { "number": 7, "method": "squash", "title": null, "message": null, "expected_head": "abc" }]),
            ]
        );
    }

    #[test]
    fn answers_decode_into_protocol_types_and_ignore_fields_they_do_not_know() {
        let identity = r#"{"ok":{"id":"7","login":"octo","name":"Octo Cat","avatar_url":null,"scopes":["repo"],"plan":"pro"}}"#;
        let forge = forge(&answering(identity), FakeHost::quiet(), None);
        assert_eq!(
            smol::block_on(forge.who_am_i()),
            Ok(Identity {
                id: "7".into(),
                login: "octo".into(),
                name: Some("Octo Cat".into()),
                avatar_url: None,
                scopes: vec!["repo".into()],
            })
        );
    }

    #[test]
    fn an_answer_of_the_wrong_shape_names_the_path_and_the_problem() {
        let listing = r#"{"ok":{"items":[{"repo":{"owner":"o","name":"n"},"description":null,"visibility":"public","default_branch":"main","clone":{"ssh":null},"web_url":"https://example.com/o/n","updated_at":null,"fork":false,"archived":false}],"next":null}}"#;
        let forge = forge(&answering(listing), FakeHost::quiet(), None);
        match smol::block_on(forge.repositories(RepositoryQuery::default())) {
            Err(CallError::Stopped(Stop::BadOutput { op, path, problem })) => {
                assert_eq!((op, path.as_str()), ("repositories.list", "items[0].clone"));
                assert!(problem.starts_with("missing field `https`"), "{problem}");
            }
            other => panic!("expected bad output, got {other:?}"),
        }
    }

    #[test]
    fn failures_come_back_failed_and_unsupported_reads_as_unavailable() {
        for (answer, expected) in [
            (r#"{"err":{"kind":"not_found"}}"#, CallError::Failed(Failure::NotFound)),
            (
                r#"{"err":{"kind":"forbidden","needs":"write:public_key","message":"Not allowed"}}"#,
                CallError::Failed(Failure::Forbidden {
                    needs: Some("write:public_key".into()),
                    message: "Not allowed".into(),
                }),
            ),
            (r#"{"err":{"kind":"unsupported"}}"#, CallError::Unavailable),
            (r#"{"err":{"kind":"from_the_future","detail":1}}"#, CallError::Failed(Failure::Unknown)),
        ] {
            let forge = forge(&answering(answer), FakeHost::quiet(), None);
            assert_eq!(smol::block_on(forge.ssh_keys()), Err(expected), "{answer}");
        }
    }

    #[test]
    fn a_rate_limited_answer_pauses_the_account() {
        let host = FakeHost::quiet();
        let limited = answering(r#"{"err":{"kind":"rate_limited","retry_after_seconds":60}}"#);
        let forge = forge(&limited, host.clone(), None);
        assert_eq!(
            smol::block_on(forge.owners()),
            Err(CallError::Failed(Failure::RateLimited { retry_after_seconds: 60 }))
        );
        assert_eq!(
            host.pauses(),
            [UNIX_EPOCH + Duration::from_millis(NOW_MILLIS) + Duration::from_secs(60)]
        );
    }

    #[test]
    fn a_repository_call_needs_a_repository_and_every_call_a_signed_in_account() {
        let host = FakeHost::quiet();
        let without_repo = forge(&recorder(r#"{"ok":[]}"#), host.clone(), None);
        assert_eq!(smol::block_on(without_repo.checks("abc".into())), Err(CallError::Unavailable));
        let started = Arc::new(Started::compile(fake_connector(), &recorder(r#"{"ok":[]}"#)).unwrap());
        let stranger = AccountId::parse("fake:example.com:2").unwrap();
        let signed_out = Forge::new(started, host.clone(), stranger, Some(repo()));
        assert_eq!(
            smol::block_on(signed_out.checks("abc".into())),
            Err(CallError::Net(NetError::NotSignedIn))
        );
        assert!(host.logs().is_empty());
    }

    #[test]
    fn anything_after_the_answer_is_bad_output() {
        match decode::<Done>("pulls.review", br#"{"ok":{}} trailing"#) {
            Err(CallError::Stopped(Stop::BadOutput { path, .. })) => assert_eq!(path, "."),
            other => panic!("expected bad output, got {other:?}"),
        }
        assert_eq!(decode::<Done>("pulls.review", br#"{"ok":{}}"#), Ok(Done {}));
    }

    #[test]
    fn answers_are_checked_against_the_operation_they_answer() {
        assert_eq!(check_answer("ssh_keys.host_keys", br#"{"ok":["ssh-ed25519 AAAA"]}"#), Ok(()));
        assert!(matches!(
            check_answer("ssh_keys.host_keys", br#"{"ok":[1]}"#),
            Err(CallError::Stopped(Stop::BadOutput { op: "ssh_keys.host_keys", .. }))
        ));
        assert_eq!(check_answer("teleport", br#"{"ok":{}}"#), Err(CallError::Unavailable));
    }

    #[test]
    fn operations_that_page_through_whole_lists_get_four_times_the_fuel() {
        assert_eq!(budget_for("pulls.threads"), Budget::of(200));
        assert_eq!(budget_for("pulls.list"), Budget::of(50));
        assert_eq!((Budget::LISTING, Budget::CALL), (Budget::of(200), Budget::of(50)));
    }

    #[test]
    fn paths_point_inside_the_answer() {
        assert_eq!(inside_answer("ok.items[0].clone"), "items[0].clone");
        assert_eq!(inside_answer("ok[2].login"), "[2].login");
        assert_eq!(inside_answer("ok"), ".");
        assert_eq!(inside_answer("err.kind"), "err.kind");
        assert_eq!(inside_answer("."), ".");
    }
}
