use std::fs;

use fastgit_github::GitHub;
use fastgit_sdk::check;
use serde_json::{Value, json};

const CASSETTES: &str = "tests/cassettes";

#[test]
fn every_cassette_replays_to_its_recorded_answer() {
    let mut played = 0;
    for entry in fs::read_dir(CASSETTES).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "json") {
            check(&GitHub, &path);
            played += 1;
        }
    }
    assert!(played >= 3, "only {played} cassettes found in {CASSETTES}");
}

#[test]
fn who_am_i_reads_the_user_and_the_scopes_the_token_holds() {
    let replayed = check(&GitHub, format!("{CASSETTES}/who_am_i.json"));
    assert_eq!(replayed.output["ok"]["scopes"][1], "repo");
}

fn answer(cassette: &str) -> Value {
    check(&GitHub, format!("{CASSETTES}/{cassette}.json")).output
}

#[test]
fn each_pull_request_carries_the_rollup_of_its_head_commits_checks() {
    let open = answer("pulls_open");
    let rollups: Vec<&Value> = (0..4).map(|item| &open["ok"]["items"][item]["checks"]).collect();
    assert_eq!(rollups, [&json!("success"), &json!("pending"), &json!("failure"), &Value::Null]);
    let closed = answer("pulls_closed_next_page");
    assert_eq!(closed["ok"]["items"][0]["checks"], "failure");
    assert_eq!(closed["ok"]["items"][1]["checks"], "pending");
}

#[test]
fn a_pull_request_from_a_fork_names_the_fork_as_its_head_and_a_deleted_fork_as_none() {
    let fork = json!({ "owner": "sam-lee", "name": "fastgit" });
    assert_eq!(answer("pulls_open")["ok"]["items"][1]["head"]["repo"], fork);
    assert_eq!(answer("pull_checking")["ok"]["summary"]["head"]["repo"], fork);
    assert_eq!(answer("create_pull_from_a_fork")["ok"]["head"]["repo"], fork);
    let closed = answer("pulls_closed_next_page");
    assert_eq!(closed["ok"]["items"][1]["head"]["repo"], Value::Null);
    assert_eq!(closed["ok"]["items"][1]["author"]["login"], "ghost");
}

#[test]
fn mergeability_follows_githubs_merge_state() {
    let mergeability = |cassette: &str| answer(cassette)["ok"]["mergeability"].clone();
    assert_eq!(mergeability("pull_ready"), json!({ "kind": "ready" }));
    assert_eq!(mergeability("pull_conflicting"), json!({ "kind": "conflicts" }));
    assert_eq!(mergeability("pull_checking"), json!({ "kind": "checking" }));
    assert_eq!(mergeability("pull_merged"), json!({ "kind": "unknown" }));
    assert_eq!(
        mergeability("pull_blocked"),
        json!({ "kind": "blocked", "reason": "At least one approving review is required." })
    );
}

#[test]
fn the_author_may_only_comment_and_merges_only_by_the_methods_the_repository_allows() {
    let ready = answer("pull_ready");
    assert_eq!(ready["ok"]["verdicts"], json!(["comment"]));
    assert_eq!(ready["ok"]["merge_methods"], json!(["merge", "squash"]));
    assert_eq!(ready["ok"]["fetch"]["refspec"], "refs/pull/142/head");
    let other = answer("pull_conflicting");
    assert_eq!(other["ok"]["verdicts"], json!(["comment", "approve", "request_changes"]));
}

#[test]
fn a_check_run_without_a_title_says_how_long_it_ran() {
    let checks = answer("checks");
    let described: Vec<&Value> = (0..9).map(|run| &checks["ok"][run]["description"]).collect();
    assert_eq!(
        described,
        [
            &json!("Successful in 4m 12s"),
            &json!("Failing after 1m 12s"),
            &json!("No warnings"),
            &Value::Null,
            &Value::Null,
            &json!("Cancelled after 30s"),
            &Value::Null,
            &json!("Failing after 1h 5m"),
            &json!("Authorization required to deploy."),
        ]
    );
    assert_eq!(checks["ok"][9]["name"], "ci/circleci: test");
}

#[test]
fn github_refusals_keep_githubs_own_words() {
    assert_eq!(
        answer("create_pull_already_open")["err"]["message"],
        "A pull request already exists for fastgit-org:topic/tabs."
    );
    assert_eq!(
        answer("merge_after_the_head_moved")["err"],
        json!({ "kind": "conflict", "message": "Head branch was modified. Review and try the merge again." })
    );
    assert_eq!(answer("pulls_of_a_missing_repository")["err"], json!({ "kind": "not_found" }));
}

#[test]
fn a_merge_method_github_lacks_is_unsupported_before_any_request() {
    let replayed = check(&GitHub, format!("{CASSETTES}/merge_with_an_unsupported_method.json"));
    assert_eq!(replayed.output["err"], json!({ "kind": "unsupported" }));
    assert!(replayed.log.is_empty());
}
