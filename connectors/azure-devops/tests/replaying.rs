use std::fs;

use sternegit_azure_devops::AzureDevOps;
use sternegit_sdk::check;
use serde_json::{Value, json};

const CASSETTES: &str = "tests/cassettes";

fn answer(cassette: &str) -> Value {
    check(&AzureDevOps, format!("{CASSETTES}/{cassette}.json")).output
}

#[test]
fn every_cassette_replays_to_its_recorded_answer() {
    let mut played = 0;
    for entry in fs::read_dir(CASSETTES).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "json") {
            check(&AzureDevOps, &path);
            played += 1;
        }
    }
    assert!(played >= 30, "only {played} cassettes found in {CASSETTES}");
}

#[test]
fn the_user_id_carries_the_organization_so_one_person_in_two_organizations_is_two_accounts() {
    assert_eq!(answer("who_am_i")["ok"]["id"], "contoso.6c8f1a2b-4d3e-4f5a-9b8c-7d6e5f4a3b2c");
    assert_eq!(answer("who_am_i_on_a_server")["ok"]["id"], "tfs.defaultcollection.0e2f7a1c-5b6d-4e8f-a1b2-c3d4e5f60718");
}

#[test]
fn a_bad_token_an_anonymous_answer_and_a_sign_in_page_all_mean_signed_out() {
    for cassette in ["who_am_i_signed_out", "who_am_i_anonymous", "who_am_i_sign_in_page"] {
        assert_eq!(answer(cassette)["err"], json!({ "kind": "signed_out" }), "{cassette}");
    }
}

#[test]
fn projects_are_the_owners_and_their_visibility_is_the_only_one_offered() {
    let owners = answer("owners");
    let logins: Vec<&Value> = (0..3).map(|owner| &owners["ok"][owner]["login"]).collect();
    assert_eq!(logins, [&json!("contoso/Archive"), &json!("contoso/Game Tools"), &json!("contoso/Web")]);
    assert_eq!(owners["ok"][0]["visibilities"], json!(["public"]));
    assert_eq!(owners["ok"][1]["can_initialize"], false);
}

#[test]
fn repository_names_come_back_readable_and_clone_addresses_without_a_user() {
    let found = answer("repository");
    assert_eq!(found["ok"]["repo"], json!({ "owner": "contoso/Game Tools", "name": "build scripts" }));
    assert_eq!(found["ok"]["clone"]["https"], "https://dev.azure.com/contoso/Game%20Tools/_git/build%20scripts");
    let all = answer("repositories");
    assert_eq!(all["ok"]["items"][2]["default_branch"], Value::Null);
    assert_eq!(all["ok"]["items"][3]["archived"], true);
    let searched = answer("repositories_of_a_project");
    assert_eq!(searched["ok"]["items"].as_array().map(Vec::len), Some(1));
}

#[test]
fn a_taken_repository_name_is_an_error_on_the_name_field() {
    assert_eq!(answer("create_repository_with_a_taken_name")["err"]["fields"][0]["field"], "name");
}

#[test]
fn votes_become_a_review_standing() {
    let open = answer("pulls_open");
    let standings: Vec<&Value> = (0..3).map(|pull| &open["ok"]["items"][pull]["review"]).collect();
    assert_eq!(standings, [&json!("approved"), &json!("changes_requested"), &json!("review_required")]);
    assert_eq!(open["ok"]["items"][1]["head"]["repo"], json!({ "owner": "contoso/Sandbox", "name": "app" }));
}

#[test]
fn closed_lists_completed_and_abandoned_pull_requests_and_pages_by_skipping() {
    let closed = answer("pulls_closed_next_page");
    let states: Vec<&Value> = (0..2).map(|pull| &closed["ok"]["items"][pull]["state"]).collect();
    assert_eq!(states, [&json!("merged"), &json!("closed")]);
    assert_eq!(closed["ok"]["items"].as_array().map(Vec::len), Some(2));
}

#[test]
fn mergeability_follows_the_merge_status_and_the_blocking_policies() {
    let mergeability = |cassette: &str| answer(cassette)["ok"]["mergeability"].clone();
    assert_eq!(mergeability("pull_ready"), json!({ "kind": "ready" }));
    assert_eq!(mergeability("pull_conflicting"), json!({ "kind": "conflicts" }));
    assert_eq!(mergeability("pull_failing_build"), json!({ "kind": "failing_checks" }));
    assert_eq!(mergeability("pull_from_a_fork"), json!({ "kind": "draft" }));
    assert_eq!(mergeability("pull_merged"), json!({ "kind": "unknown" }));
    assert_eq!(mergeability("pull_completing"), json!({ "kind": "checking" }));
    assert_eq!(
        mergeability("pull_blocked"),
        json!({ "kind": "blocked", "reason": "Waiting for: Minimum number of reviewers, Comment requirements." })
    );
}

#[test]
fn the_merge_strategy_policy_limits_the_merge_methods() {
    let methods = |cassette: &str| answer(cassette)["ok"]["merge_methods"].clone();
    assert_eq!(methods("pull_ready"), json!(["merge", "squash", "rebase", "semi_linear"]));
    assert_eq!(methods("pull_blocked"), json!(["squash"]));
    assert_eq!(methods("pull_failing_build"), json!(["merge", "squash"]));
    assert_eq!(methods("pull_from_a_fork"), json!(["squash"]));
}

#[test]
fn the_head_is_fetched_from_its_branch_and_a_forks_from_the_fork() {
    let ready = answer("pull_ready");
    assert_eq!(ready["ok"]["fetch"], json!({ "url": null, "refspec": "refs/heads/topic/tabs", "sha": "9f1c2b3a4d5e6f708192a3b4c5d6e7f809102132" }));
    assert_eq!(answer("pull_from_a_fork")["ok"]["fetch"]["url"], "https://dev.azure.com/contoso/Sandbox/_git/app");
}

#[test]
fn checks_join_commit_statuses_pull_request_statuses_and_branch_policies() {
    let checks = answer("checks");
    let names: Vec<&str> = checks["ok"].as_array().unwrap().iter().map(|check| check["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["continuous-integration/build", "deploy", "sternegit-ci/lint", "security-scan", "CI", "Minimum number of reviewers", "Comment requirements"]
    );
    assert_eq!(checks["ok"][2]["state"], "success", "the latest lint status wins");
    assert_eq!(checks["ok"][4]["web_url"], "https://dev.azure.com/contoso/Web/_build/results?buildId=812");
}

#[test]
fn the_conversation_keeps_written_comments_outside_files_and_drops_system_notes() {
    let conversation = answer("conversation");
    let ids: Vec<&str> = conversation["ok"]["items"].as_array().unwrap().iter().map(|comment| comment["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["102.1", "102.2", "105.2"]);
    assert_eq!(conversation["ok"]["items"][1]["reply_to"], "102.1");
}

#[test]
fn a_merge_is_done_or_queued_and_a_fast_forward_is_unsupported_before_any_request() {
    assert_eq!(answer("merge")["ok"], json!({ "sha": "7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b", "queued": false }));
    assert_eq!(answer("merge_queued")["ok"], json!({ "sha": null, "queued": true }));
    let replayed = check(&AzureDevOps, format!("{CASSETTES}/merge_with_an_unsupported_method.json"));
    assert_eq!(replayed.output["err"], json!({ "kind": "unsupported" }));
    assert_eq!(answer("create_pull_from_a_fork")["err"], json!({ "kind": "unsupported" }));
}

#[test]
fn a_collection_on_a_server_is_reached_under_its_own_path() {
    let pulls = answer("pulls_on_a_server");
    assert_eq!(pulls["ok"]["items"][0]["web_url"], "http://devops.example.com:8080/tfs/DefaultCollection/Game/_git/client/pullrequest/7");
}
