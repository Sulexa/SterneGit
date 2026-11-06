use std::fs;

use fastgit_sdk::{
    AccountContext, Connector, Failure, Identity, Level, Page, Person, PullState, PullSummary,
    PullsQuery, RepoRef, Request, SDK_VERSION, Tip, check, dispatch, export, log,
    protocol::{
        Cassette,
        ops::{self, Op, Visit},
    },
    replay,
};
use serde_json::{Value, json};

struct Toy;

struct Bare;

impl Connector for Bare {}

impl Connector for Toy {
    fn who_am_i(&self, account: &AccountContext) -> Result<Identity, Failure> {
        log(Level::Info, "asking who I am");
        let response = Request::get("/user")
            .header("Accept", "application/json")
            .fetch();
        if response.status == 401 {
            return Err(Failure::SignedOut);
        }
        let user: Value = response.json()?;
        Ok(Identity {
            id: user["id"].to_string(),
            login: user["login"].as_str().unwrap_or(&account.login).to_owned(),
            name: user["name"].as_str().map(str::to_owned),
            avatar_url: None,
            scopes: response
                .header("x-oauth-scopes")
                .map(|scopes| scopes.split(", ").map(str::to_owned).collect())
                .unwrap_or_default(),
        })
    }

    fn pulls(
        &self,
        _: &AccountContext,
        repo: &RepoRef,
        query: PullsQuery,
    ) -> Result<Page<PullSummary>, Failure> {
        let first = format!("repos/{}/{}/pulls?state=open", repo.owner, repo.name);
        let listing = Request::get(query.cursor.unwrap_or(first)).send();
        let limits = Request::get("rate_limit").send();
        let remaining = limits
            .wait()
            .header("x-ratelimit-remaining")
            .unwrap_or("?")
            .to_owned();
        let listing = listing.wait();
        log(Level::Debug, &format!("{remaining} requests left"));
        let pulls: Vec<Value> = listing.json()?;
        Ok(Page {
            items: pulls.iter().map(|pull| summary(repo, pull)).collect(),
            next: listing.next_page(),
        })
    }
}

fn summary(repo: &RepoRef, pull: &Value) -> PullSummary {
    let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
    let tip = |branch: &str| Tip {
        branch: branch.to_owned(),
        sha: "0".repeat(40),
        repo: Some(repo.clone()),
    };
    PullSummary {
        number: pull["number"].as_u64().unwrap_or_default(),
        title: text(&pull["title"]),
        state: PullState::Open,
        draft: false,
        author: Person {
            login: text(&pull["user"]["login"]),
            name: None,
            avatar_url: None,
        },
        base: tip("main"),
        head: tip("topic"),
        review: None,
        checks: None,
        created_at: text(&pull["created_at"]),
        updated_at: text(&pull["created_at"]),
        web_url: text(&pull["html_url"]),
    }
}

export!(Toy);

fn cassette(name: &str) -> Cassette {
    let text = fs::read_to_string(format!("tests/cassettes/{name}.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn answered(connector: &impl Connector, call: &str) -> Value {
    serde_json::from_slice(&dispatch(connector, call.as_bytes())).unwrap()
}

#[test]
fn a_cassette_replays_through_the_connector_and_keeps_its_log() {
    let replayed = check(&Toy, "tests/cassettes/who_am_i.json");
    assert_eq!(replayed.log, ["Info asking who I am"]);
}

#[test]
fn requests_sent_together_are_answered_whatever_order_they_are_read_in() {
    let replayed = check(&Toy, "tests/cassettes/pulls_list.json");
    assert_eq!(replayed.log, ["Debug 4999 requests left"]);
}

#[test]
fn a_failure_is_answered_as_err() {
    let mut signed_out = cassette("who_am_i");
    signed_out.exchanges[0].status = 401;
    let replayed = replay(&Toy, &signed_out);
    assert_eq!(replayed.output, json!({ "err": { "kind": "signed_out" } }));
    assert!(replayed.unplayed.is_empty());
}

#[test]
#[should_panic(expected = "The cassette holds no answer to GET https://api.github.com/user")]
fn a_request_the_cassette_lacks_fails_loudly() {
    let mut empty = cassette("who_am_i");
    empty.exchanges.clear();
    replay(&Toy, &empty);
}

#[test]
#[should_panic(expected = "the connector never asked for GET https://api.github.com/rate_limit")]
fn an_exchange_the_connector_never_asked_for_fails_the_check() {
    let mut extra = cassette("who_am_i");
    extra.exchanges.push(cassette("pulls_list").exchanges[0].clone());
    let folder = std::env::temp_dir().join(format!("fastgit-sdk-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("extra.json");
    fs::write(&path, serde_json::to_string(&extra).unwrap()).unwrap();
    check(&Toy, &path);
}

#[test]
fn hello_names_the_protocol_and_the_sdk() {
    assert_eq!(
        answered(&Bare, r#"{"op":"hello","account":null,"repo":null,"input":{"protocol":"1.0","sdk":null}}"#),
        json!({ "ok": { "protocol": "1.0", "sdk": SDK_VERSION } })
    );
}

#[test]
fn operations_the_connector_lacks_are_unsupported() {
    for op in ["ssh_keys.list", "pulls.teleport"] {
        let call = format!(
            r#"{{"op":"{op}","account":null,"repo":{{"owner":"o","name":"n"}},"input":null}}"#
        );
        assert_eq!(answered(&Toy, &call), json!({ "err": { "kind": "unsupported" } }), "{op}");
    }
}

#[test]
fn repository_operations_need_a_repository() {
    struct OnRepository;

    impl Visit for OnRepository {
        type Result = bool;

        fn visit<O: Op>(self) -> bool {
            O::ON_REPOSITORY
        }
    }

    for op in ops::NAMES {
        let answer = answered(&Bare, &format!(
            r#"{{"op":"{op}","account":null,"repo":null,"input":null}}"#
        ));
        let needs_repository = answer["err"]["message"] == format!("{op} needs a repository");
        assert_eq!(Some(needs_repository), ops::visit(op, OnRepository), "{op}: {answer}");
    }
}

#[test]
fn a_call_or_input_of_the_wrong_shape_is_misunderstood() {
    let wrong_input = answered(
        &Toy,
        r#"{"op":"pulls.list","account":null,"repo":{"owner":"o","name":"n"},"input":{"state":7}}"#,
    );
    assert_eq!(wrong_input["err"]["kind"], "protocol");
    assert!(
        wrong_input["err"]["message"]
            .as_str()
            .unwrap()
            .starts_with("FastGit's input to pulls.list can't be read")
    );
    let garbage = answered(&Toy, "not json");
    assert!(
        garbage["err"]["message"]
            .as_str()
            .unwrap()
            .starts_with("FastGit's call can't be read")
    );
}
