use std::fs;

use fastgit_github::GitHub;
use fastgit_sdk::check;

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
