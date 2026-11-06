use sternegit_sdk::{Check, CheckState, CommitQuery, Failure, RepoRef, Request};
use serde_json::Value;

use crate::{
    api,
    listing::{encoded, repo_path, text, written},
    moments::{epoch_seconds, spoken},
    refusal::succeeded,
};

pub(crate) fn list(repo: &RepoRef, query: CommitQuery) -> Result<Vec<Check>, Failure> {
    let commit = format!("{}/commits/{}", repo_path(repo), encoded(&query.sha));
    let runs = api(Request::get(format!("{commit}/check-runs?per_page=100"))).send();
    let statuses = api(Request::get(format!("{commit}/status?per_page=100"))).send();
    let (runs, statuses) = (runs.wait(), statuses.wait());
    let runs: Value = succeeded(runs)?.json()?;
    let statuses: Value = succeeded(statuses)?.json()?;
    Ok(listed(&runs, "check_runs")
        .iter()
        .map(check_run)
        .chain(listed(&statuses, "statuses").iter().map(commit_status))
        .collect())
}

fn listed<'a>(answer: &'a Value, name: &str) -> &'a [Value] {
    answer[name].as_array().map_or(&[], Vec::as_slice)
}

fn check_run(run: &Value) -> Check {
    let state = run_state(run["status"].as_str(), run["conclusion"].as_str());
    Check {
        name: text(&run["name"]).unwrap_or_default(),
        state,
        description: written(&run["output"]["title"]).or_else(|| took(state, run)),
        web_url: text(&run["html_url"]),
    }
}

fn commit_status(status: &Value) -> Check {
    Check {
        name: text(&status["context"]).unwrap_or_default(),
        state: match status["state"].as_str() {
            Some("error" | "failure") => CheckState::Failure,
            Some("pending") => CheckState::Pending,
            Some("success") => CheckState::Success,
            _ => CheckState::Unknown,
        },
        description: written(&status["description"]),
        web_url: written(&status["target_url"]),
    }
}

fn run_state(status: Option<&str>, conclusion: Option<&str>) -> CheckState {
    match (status, conclusion) {
        (Some("queued" | "waiting" | "requested" | "pending"), _) => CheckState::Pending,
        (Some("in_progress"), _) => CheckState::Running,
        (Some("completed"), Some("success")) => CheckState::Success,
        (Some("completed"), Some("failure" | "timed_out" | "startup_failure")) => CheckState::Failure,
        (Some("completed"), Some("neutral")) => CheckState::Neutral,
        (Some("completed"), Some("cancelled")) => CheckState::Cancelled,
        (Some("completed"), Some("skipped" | "stale")) => CheckState::Skipped,
        (Some("completed"), Some("action_required")) => CheckState::ActionRequired,
        _ => CheckState::Unknown,
    }
}

fn took(state: CheckState, run: &Value) -> Option<String> {
    let started = epoch_seconds(run["started_at"].as_str()?)?;
    let completed = epoch_seconds(run["completed_at"].as_str()?)?;
    let lasted = spoken(Some(completed - started).filter(|seconds| *seconds >= 0)?);
    match state {
        CheckState::Success => Some(format!("Successful in {lasted}")),
        CheckState::Failure => Some(format!("Failing after {lasted}")),
        CheckState::Cancelled => Some(format!("Cancelled after {lasted}")),
        _ => None,
    }
}
