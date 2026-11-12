use sternegit_sdk::{
    BranchQuery, Failure, FetchSpec, Merge, MergeMethod, Mergeability, Merged, NewPull, Page,
    Person, PullDetails, PullNumber, PullState, PullSummary, PullsQuery, RepoRef, Request,
    Response, ReviewStanding, StateFilter, Tip, Verdict,
};
use serde_json::{Value, json};

use crate::{
    api,
    moments::moment,
    place::{Collection, PREVIEW, VERSION, branch, encoded, reference, without_user},
    refusal::succeeded,
    repositories::listed,
};

const PAGE: usize = 100;
const MERGE_STRATEGY_POLICY: &str = "fa4e907d-c16b-4a4c-9dfa-4916e5d171ab";
const BUILD_POLICY: &str = "0609b952-1397-4640-95ec-e00a01b2c241";
const STRATEGIES: [(&str, MergeMethod); 4] = [
    ("allowNoFastForward", MergeMethod::Merge),
    ("allowSquash", MergeMethod::Squash),
    ("allowRebase", MergeMethod::Rebase),
    ("allowRebaseMerge", MergeMethod::SemiLinear),
];
const APPROVED: i64 = 5;
const WAITING_FOR_AUTHOR: i64 = -5;
const UNFINISHED_POLICIES: [&str; 2] = ["queued", "running"];
const FAILED_POLICIES: [&str; 2] = ["rejected", "broken"];

pub(crate) fn list(collection: &Collection, repo: &RepoRef, query: PullsQuery) -> Result<Page<PullSummary>, Failure> {
    let skip: usize = match &query.cursor {
        Some(cursor) => cursor.parse().map_err(|_| Failure::Protocol {
            message: format!("{cursor} isn't a page of Azure DevOps pull requests"),
        })?,
        None => 0,
    };
    let status = match query.state {
        StateFilter::Open => "active",
        StateFilter::Closed | StateFilter::All | StateFilter::Unknown => "all",
    };
    let url = format!(
        "{}/pullrequests?searchCriteria.status={status}&$top={PAGE}&$skip={skip}&{VERSION}",
        collection.repository(repo)
    );
    let answer: Value = succeeded(api(Request::get(url)).fetch())?.json()?;
    let found = listed(&answer)?;
    let mut items = found
        .iter()
        .map(|pull| summary(collection, repo, pull))
        .collect::<Result<Vec<_>, _>>()?;
    if query.state == StateFilter::Closed {
        items.retain(|item| item.state != PullState::Open);
    }
    Ok(Page {
        items,
        next: (found.len() >= PAGE).then(|| (skip + found.len()).to_string()),
    })
}

pub(crate) fn for_branch(collection: &Collection, repo: &RepoRef, query: BranchQuery) -> Result<Vec<PullSummary>, Failure> {
    let url = format!(
        "{}/pullrequests?searchCriteria.sourceRefName={}&searchCriteria.status=active&$top={PAGE}&{VERSION}",
        collection.repository(repo),
        encoded(&reference(&query.branch))
    );
    let answer: Value = succeeded(api(Request::get(url)).fetch())?.json()?;
    listed(&answer)?
        .iter()
        .map(|pull| summary(collection, repo, pull))
        .collect()
}

pub(crate) fn get(collection: &Collection, repo: &RepoRef, query: PullNumber) -> Result<PullDetails, Failure> {
    let url = format!("{}/pullrequests/{}?{VERSION}", collection.repository(repo), query.number);
    let pull: Value = succeeded(api(Request::get(url)).fetch())?.json()?;
    let summary = summary(collection, repo, &pull)?;
    let project = encoded(&collection.project(repo));
    let target = pull["targetRefName"].as_str().unwrap_or_default();
    let strategies = Request::get(format!(
        "{project}/_apis/git/policy/configurations?repositoryId={}&refName={}&policyType={MERGE_STRATEGY_POLICY}&{PREVIEW}",
        encoded(pull["repository"]["id"].as_str().unwrap_or_default()),
        encoded(target)
    ));
    let strategies = api(strategies).send();
    let evaluations = api(Request::get(evaluations_url(&project, &pull))).send();
    let strategies = optional(strategies.wait());
    let evaluations = optional(evaluations.wait());
    Ok(PullDetails {
        body: pull["description"].as_str().unwrap_or_default().to_owned(),
        base_sha: summary.base.sha.clone(),
        mergeability: mergeability(&pull, &evaluations),
        merge_methods: merge_methods(&strategies),
        verdicts: vec![Verdict::Comment, Verdict::Approve, Verdict::RequestChanges],
        fetch: FetchSpec {
            url: fork_url(collection, &pull),
            refspec: pull["sourceRefName"].as_str().unwrap_or_default().to_owned(),
            sha: summary.head.sha.clone(),
        },
        summary,
    })
}

pub(crate) fn create(collection: &Collection, repo: &RepoRef, new: NewPull) -> Result<PullSummary, Failure> {
    if new.head_owner.is_some() {
        return Err(Failure::Unsupported);
    }
    let url = format!("{}/pullrequests?{VERSION}", collection.repository(repo));
    let body = json!({
        "sourceRefName": reference(&new.head),
        "targetRefName": reference(&new.base),
        "title": new.title,
        "description": new.body,
        "isDraft": new.draft,
    });
    let pull: Value = succeeded(api(Request::post(url)).json(&body).fetch())?.json()?;
    summary(collection, repo, &pull)
}

pub(crate) fn merge(collection: &Collection, repo: &RepoRef, merge: Merge) -> Result<Merged, Failure> {
    let strategy = match merge.method {
        MergeMethod::Merge => "noFastForward",
        MergeMethod::Squash => "squash",
        MergeMethod::Rebase => "rebase",
        MergeMethod::SemiLinear => "rebaseMerge",
        MergeMethod::FastForward | MergeMethod::Unknown => return Err(Failure::Unsupported),
    };
    let mut options = json!({ "mergeStrategy": strategy, "deleteSourceBranch": false, "transitionWorkItems": false });
    let written: Vec<&str> = [merge.title.as_deref(), merge.message.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if !written.is_empty() {
        options["mergeCommitMessage"] = json!(written.join("\n\n"));
    }
    let body = json!({
        "status": "completed",
        "lastMergeSourceCommit": { "commitId": merge.expected_head },
        "completionOptions": options,
    });
    let url = format!("{}/pullrequests/{}?{VERSION}", collection.repository(repo), merge.number);
    let pull: Value = succeeded(api(Request::patch(url)).json(&body).fetch())?.json()?;
    Ok(match pull["status"].as_str() {
        Some("completed") => Merged {
            sha: pull["lastMergeCommit"]["commitId"].as_str().map(str::to_owned),
            queued: false,
        },
        _ => Merged { sha: None, queued: true },
    })
}

pub(crate) fn summary(collection: &Collection, repo: &RepoRef, pull: &Value) -> Result<PullSummary, Failure> {
    let number = pull["pullRequestId"].as_u64().ok_or_else(|| Failure::Protocol {
        message: "Azure DevOps listed a pull request without its id".to_owned(),
    })?;
    let created_at = moment(&pull["creationDate"]).unwrap_or_default();
    Ok(PullSummary {
        number,
        title: pull["title"].as_str().unwrap_or_default().to_owned(),
        state: match pull["status"].as_str() {
            Some("active") => PullState::Open,
            Some("completed") => PullState::Merged,
            Some("abandoned") => PullState::Closed,
            _ => PullState::Unknown,
        },
        draft: pull["isDraft"].as_bool().unwrap_or(false),
        author: person(&pull["createdBy"]),
        base: Tip {
            branch: branch(&pull["targetRefName"]),
            sha: commit(&pull["lastMergeTargetCommit"]),
            repo: None,
        },
        head: Tip {
            branch: branch(&pull["sourceRefName"]),
            sha: commit(&pull["lastMergeSourceCommit"]),
            repo: collection.repo_ref(&pull["forkSource"]["repository"]),
        },
        review: review(&pull["reviewers"]),
        checks: None,
        updated_at: moment(&pull["closedDate"]).unwrap_or_else(|| created_at.clone()),
        created_at,
        web_url: format!("{}/pullrequest/{number}", collection.web(repo)),
    })
}

pub(crate) fn person(identity: &Value) -> Person {
    let text = |name: &str| identity[name].as_str().filter(|text| !text.trim().is_empty()).map(str::to_owned);
    let name = text("displayName");
    Person {
        login: text("uniqueName").or_else(|| name.clone()).unwrap_or_else(|| "unknown".to_owned()),
        name,
        avatar_url: None,
    }
}

pub(crate) fn evaluations_url(project: &str, pull: &Value) -> String {
    let artifact = format!(
        "vstfs:///CodeReview/CodeReviewId/{}/{}",
        pull["repository"]["project"]["id"].as_str().unwrap_or_default(),
        pull["pullRequestId"].as_u64().unwrap_or_default()
    );
    format!("{project}/_apis/policy/evaluations?artifactId={}&{PREVIEW}", encoded(&artifact))
}

pub(crate) fn blocking(evaluations: &Value) -> impl Iterator<Item = &Value> {
    evaluations["value"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|evaluation| evaluation["configuration"]["isBlocking"].as_bool() == Some(true))
        .filter(|evaluation| evaluation["configuration"]["isEnabled"].as_bool() != Some(false))
}

pub(crate) fn policy_name(evaluation: &Value) -> String {
    let configuration = &evaluation["configuration"];
    configuration["settings"]["displayName"]
        .as_str()
        .or(configuration["type"]["displayName"].as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("A branch policy")
        .to_owned()
}

pub(crate) fn is_build(evaluation: &Value) -> bool {
    evaluation["configuration"]["type"]["id"].as_str() == Some(BUILD_POLICY)
}

pub(crate) fn optional(response: Response) -> Value {
    succeeded(response)
        .ok()
        .and_then(|response| response.json().ok())
        .unwrap_or(Value::Null)
}

fn commit(value: &Value) -> String {
    value["commitId"].as_str().unwrap_or_default().to_owned()
}

fn review(reviewers: &Value) -> Option<ReviewStanding> {
    let reviewers = reviewers.as_array().filter(|reviewers| !reviewers.is_empty())?;
    let vote = |reviewer: &Value| reviewer["vote"].as_i64().unwrap_or(0);
    if reviewers.iter().any(|reviewer| vote(reviewer) <= WAITING_FOR_AUTHOR) {
        return Some(ReviewStanding::ChangesRequested);
    }
    let required_pending = reviewers
        .iter()
        .any(|reviewer| reviewer["isRequired"].as_bool() == Some(true) && vote(reviewer) < APPROVED);
    if required_pending {
        return Some(ReviewStanding::ReviewRequired);
    }
    reviewers
        .iter()
        .any(|reviewer| vote(reviewer) >= APPROVED)
        .then_some(ReviewStanding::Approved)
}

fn mergeability(pull: &Value, evaluations: &Value) -> Mergeability {
    if pull["status"].as_str() != Some("active") {
        return Mergeability::Unknown;
    }
    if pull["isDraft"].as_bool() == Some(true) {
        return Mergeability::Draft;
    }
    match pull["mergeStatus"].as_str() {
        Some("conflicts") => return Mergeability::Conflicts,
        Some("succeeded") => {}
        Some("rejectedByPolicy") => {
            return Mergeability::Blocked {
                reason: "A branch policy refuses this merge.".to_owned(),
            };
        }
        Some("failure") => {
            return Mergeability::Blocked {
                reason: pull["mergeFailureMessage"]
                    .as_str()
                    .unwrap_or("Azure DevOps couldn't merge the branches.")
                    .to_owned(),
            };
        }
        _ => return Mergeability::Checking,
    }
    if pull["completionQueueTime"].is_string() {
        return Mergeability::Checking;
    }
    let status =|evaluation: &Value| evaluation["status"].as_str().unwrap_or_default().to_owned();
    if blocking(evaluations).any(|evaluation| is_build(evaluation) && FAILED_POLICIES.contains(&status(evaluation).as_str())) {
        return Mergeability::FailingChecks;
    }
    let waiting: Vec<String> = blocking(evaluations)
        .filter(|evaluation| {
            let status = status(evaluation);
            UNFINISHED_POLICIES.contains(&status.as_str()) || FAILED_POLICIES.contains(&status.as_str())
        })
        .map(policy_name)
        .collect();
    match waiting.is_empty() {
        true => Mergeability::Ready,
        false => Mergeability::Blocked {
            reason: format!("Waiting for: {}.", waiting.join(", ")),
        },
    }
}

fn merge_methods(strategies: &Value) -> Vec<MergeMethod> {
    let enabled: Vec<&Value> = strategies["value"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|policy| policy["isEnabled"].as_bool() != Some(false))
        .map(|policy| &policy["settings"])
        .collect();
    if enabled.is_empty() {
        return STRATEGIES.map(|(_, method)| method).to_vec();
    }
    let allowed = |setting: &str| {
        enabled.iter().all(|settings| match settings.get(setting) {
            Some(value) => value.as_bool() == Some(true),
            None => setting == "allowSquash" && settings["useSquashMerge"].as_bool() == Some(true),
        })
    };
    STRATEGIES
        .into_iter()
        .filter(|(setting, _)| allowed(setting))
        .map(|(_, method)| method)
        .collect()
}

fn fork_url(collection: &Collection, pull: &Value) -> Option<String> {
    let fork = &pull["forkSource"]["repository"];
    if fork.is_null() {
        return None;
    }
    fork["remoteUrl"]
        .as_str()
        .map(without_user)
        .or_else(|| collection.repo_ref(fork).map(|repo| collection.web(&repo)))
}
