use sternegit_sdk::{Check, CheckState, CommitQuery, Failure, RepoRef, Request};
use serde_json::{Value, json};

use crate::{
    api,
    place::{Collection, PREVIEW, VERSION, encoded},
    pulls::{evaluations_url, is_build, optional, policy_name},
    refusal::succeeded,
    repositories::listed,
};

const NOT_APPLICABLE: &str = "notApplicable";

pub(crate) fn list(collection: &Collection, repo: &RepoRef, query: CommitQuery) -> Result<Vec<Check>, Failure> {
    let repository = collection.repository(repo);
    let statuses = Request::get(format!(
        "{repository}/commits/{}/statuses?latestOnly=true&{VERSION}",
        encoded(&query.sha)
    ));
    let statuses = api(statuses).send();
    let search = json!({ "queries": [{ "type": "lastMergeSourceCommit", "items": [query.sha] }] });
    let search = api(Request::post(format!("{repository}/pullrequestquery?{VERSION}")).json(&search)).send();
    let statuses: Value = succeeded(statuses.wait())?.json()?;
    let found = optional(search.wait());
    let mut checks: Vec<Check> = latest(listed(&statuses)?);
    let Some(pull) = pull_for(&found, &query.sha) else {
        return Ok(checks);
    };
    let number = pull["pullRequestId"].as_u64().unwrap_or_default();
    let project = encoded(&collection.project(repo));
    let pull_statuses = api(Request::get(format!("{repository}/pullRequests/{number}/statuses?{PREVIEW}"))).send();
    let evaluations = pull["repository"]["project"]["id"]
        .as_str()
        .map(|_| api(Request::get(evaluations_url(&project, pull))).send());
    let pull_statuses = optional(pull_statuses.wait());
    checks.extend(latest(pull_statuses["value"].as_array().map(Vec::as_slice).unwrap_or_default()));
    if let Some(evaluations) = evaluations {
        let evaluations = optional(evaluations.wait());
        checks.extend(
            evaluations["value"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|evaluation| evaluation["configuration"]["isEnabled"].as_bool() != Some(false))
                .filter(|evaluation| evaluation["status"].as_str() != Some(NOT_APPLICABLE))
                .map(|evaluation| policy_check(collection, repo, evaluation)),
        );
    }
    Ok(checks)
}

fn pull_for<'a>(found: &'a Value, sha: &str) -> Option<&'a Value> {
    let pulls = found["results"]
        .as_array()?
        .iter()
        .find_map(|result| result[sha].as_array())?;
    pulls
        .iter()
        .find(|pull| pull["status"].as_str() == Some("active"))
        .or_else(|| pulls.first())
}

fn latest(statuses: &[Value]) -> Vec<Check> {
    let mut checks: Vec<Check> = Vec::new();
    for status in statuses {
        let check = status_check(status);
        match checks.iter_mut().find(|known| known.name == check.name) {
            Some(known) => *known = check,
            None => checks.push(check),
        }
    }
    checks
}

fn status_check(status: &Value) -> Check {
    let context = &status["context"];
    let name = context["name"].as_str().unwrap_or("status");
    let text = |value: &Value| value.as_str().filter(|text| !text.trim().is_empty()).map(str::to_owned);
    Check {
        name: match context["genre"].as_str().filter(|genre| !genre.trim().is_empty()) {
            Some(genre) => format!("{genre}/{name}"),
            None => name.to_owned(),
        },
        state: match status["state"].as_str() {
            Some("succeeded") => CheckState::Success,
            Some("failed" | "error") => CheckState::Failure,
            Some("pending") => CheckState::Pending,
            Some(NOT_APPLICABLE) => CheckState::Skipped,
            _ => CheckState::Unknown,
        },
        description: text(&status["description"]),
        web_url: text(&status["targetUrl"]),
    }
}

fn policy_check(collection: &Collection, repo: &RepoRef, evaluation: &Value) -> Check {
    let blocking = evaluation["configuration"]["isBlocking"].as_bool() == Some(true);
    let build = evaluation["context"]["buildId"].as_u64().filter(|_| is_build(evaluation));
    Check {
        name: policy_name(evaluation),
        state: match evaluation["status"].as_str() {
            Some("approved") => CheckState::Success,
            Some("rejected" | "broken") => CheckState::Failure,
            Some("running") => CheckState::Running,
            Some("queued") => CheckState::Pending,
            _ => CheckState::Unknown,
        },
        description: Some(match blocking {
            true => "Required".to_owned(),
            false => "Optional".to_owned(),
        }),
        web_url: build.map(|id| collection.in_project(repo, &format!("_build/results?buildId={id}"))),
    }
}
