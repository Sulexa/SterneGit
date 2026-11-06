use sternegit_sdk::{
    AccountContext, BranchQuery, CheckState, Failure, FetchSpec, Merge, MergeMethod, Mergeability,
    Merged, NewPull, Page, Person, PullDetails, PullNumber, PullState, PullSummary, PullsQuery,
    RepoRef, Request, ReviewStanding, StateFilter, Tip, Verdict,
};
use serde_json::{Value, json};

use crate::{
    api,
    graphql::{self, found},
    listing::{encoded, repo_path, text, written},
    refusal::succeeded,
};

const SUMMARY: &str = "fragment Summary on PullRequest { \
    number title state isDraft url createdAt updatedAt reviewDecision \
    author { login avatarUrl ... on User { name } } \
    baseRefName baseRefOid headRefName headRefOid headRepository { name owner { login } } \
    commits(last: 1) { nodes { commit { statusCheckRollup { state } } } } }";

const LIST: &str = "query($owner: String!, $name: String!, $states: [PullRequestState!], $cursor: String) { \
    repository(owner: $owner, name: $name) { \
    pullRequests(states: $states, first: 100, after: $cursor, orderBy: {field: UPDATED_AT, direction: DESC}) { \
    pageInfo { hasNextPage endCursor } nodes { ...Summary } } } } ";

const DETAILS: &str = "query($owner: String!, $name: String!, $number: Int!) { \
    repository(owner: $owner, name: $name) { \
    mergeCommitAllowed squashMergeAllowed rebaseMergeAllowed \
    pullRequest(number: $number) { ...Summary body mergeable mergeStateStatus viewerDidAuthor } } } ";

const GHOST: &str = "ghost";
const GRAPH_AVATAR: &str = "avatarUrl";
pub(crate) const REST_AVATAR: &str = "avatar_url";
const ALLOWED_METHODS: [(&str, MergeMethod); 3] = [
    ("mergeCommitAllowed", MergeMethod::Merge),
    ("squashMergeAllowed", MergeMethod::Squash),
    ("rebaseMergeAllowed", MergeMethod::Rebase),
];
const NEEDS_APPROVAL: &str = "At least one approving review is required.";
const CHANGES_REQUESTED: &str = "Changes were requested in a review.";
const RULES_BLOCK: &str = "A required check hasn't passed, or a branch rule blocks it.";

pub(crate) fn list(account: &AccountContext, repo: &RepoRef, query: PullsQuery) -> Result<Page<PullSummary>, Failure> {
    let mut variables = json!({ "owner": repo.owner, "name": repo.name, "cursor": query.cursor });
    if let Some(states) = states(query.state) {
        variables["states"] = json!(states);
    }
    let data = graphql::query(account, &[LIST, SUMMARY].concat(), variables)?;
    let pulls = &found(&data["repository"])?["pullRequests"];
    let nodes = pulls["nodes"].as_array().ok_or_else(|| Failure::Protocol {
        message: "GitHub's answer lists no pull requests".to_owned(),
    })?;
    let page = &pulls["pageInfo"];
    Ok(Page {
        items: nodes.iter().map(|node| summary(repo, node)).collect::<Result<_, _>>()?,
        next: match page["hasNextPage"].as_bool() {
            Some(true) => text(&page["endCursor"]),
            _ => None,
        },
    })
}

pub(crate) fn for_branch(repo: &RepoRef, query: BranchQuery) -> Result<Vec<PullSummary>, Failure> {
    let head = encoded(&format!("{}:{}", repo.owner, query.branch));
    let url = format!("{}/pulls?head={head}&state=open&per_page=100", repo_path(repo));
    let pulls: Vec<Value> = succeeded(api(Request::get(url)).fetch())?.json()?;
    pulls.iter().map(listed_summary).collect()
}

pub(crate) fn get(account: &AccountContext, repo: &RepoRef, query: PullNumber) -> Result<PullDetails, Failure> {
    let variables = json!({ "owner": repo.owner, "name": repo.name, "number": query.number });
    let data = graphql::query(account, &[DETAILS, SUMMARY].concat(), variables)?;
    let repository = found(&data["repository"])?;
    let pull = found(&repository["pullRequest"])?;
    let summary = summary(repo, pull)?;
    Ok(PullDetails {
        body: text(&pull["body"]).unwrap_or_default(),
        base_sha: summary.base.sha.clone(),
        mergeability: mergeability(pull),
        merge_methods: ALLOWED_METHODS
            .into_iter()
            .filter(|(setting, _)| repository[*setting].as_bool() == Some(true))
            .map(|(_, method)| method)
            .collect(),
        verdicts: match pull["viewerDidAuthor"].as_bool() {
            Some(true) => vec![Verdict::Comment],
            _ => vec![Verdict::Comment, Verdict::Approve, Verdict::RequestChanges],
        },
        fetch: FetchSpec {
            url: None,
            refspec: format!("refs/pull/{}/head", summary.number),
            sha: summary.head.sha.clone(),
        },
        summary,
    })
}

pub(crate) fn create(repo: &RepoRef, new: NewPull) -> Result<PullSummary, Failure> {
    let url = format!("{}/pulls", repo_path(repo));
    listed_summary(&succeeded(api(Request::post(url)).json(&opening(new)).fetch())?.json()?)
}

pub(crate) fn merge(repo: &RepoRef, merge: Merge) -> Result<Merged, Failure> {
    let url = format!("{}/pulls/{}/merge", repo_path(repo), merge.number);
    let body = merging(merge)?;
    let merged: Value = succeeded(api(Request::put(url)).json(&body).fetch())?.json()?;
    Ok(Merged {
        sha: text(&merged["sha"]),
        queued: false,
    })
}

pub(crate) fn person(user: &Value, avatar: &str) -> Person {
    match text(&user["login"]) {
        Some(login) => Person {
            login,
            name: written(&user["name"]),
            avatar_url: text(&user[avatar]),
        },
        None => Person {
            login: GHOST.to_owned(),
            name: None,
            avatar_url: None,
        },
    }
}

fn opening(new: NewPull) -> Value {
    let head = match new.head_owner {
        Some(owner) => format!("{owner}:{}", new.head),
        None => new.head,
    };
    json!({ "title": new.title, "head": head, "base": new.base, "body": new.body, "draft": new.draft })
}

fn merging(merge: Merge) -> Result<Value, Failure> {
    let method = match merge.method {
        MergeMethod::Merge => "merge",
        MergeMethod::Squash => "squash",
        MergeMethod::Rebase => "rebase",
        MergeMethod::SemiLinear | MergeMethod::FastForward | MergeMethod::Unknown => {
            return Err(Failure::Unsupported);
        }
    };
    let mut body = json!({ "merge_method": method, "sha": merge.expected_head });
    if let Some(title) = merge.title {
        body["commit_title"] = title.into();
    }
    if let Some(message) = merge.message {
        body["commit_message"] = message.into();
    }
    Ok(body)
}

fn states(filter: StateFilter) -> Option<&'static [&'static str]> {
    match filter {
        StateFilter::Open => Some(&["OPEN"]),
        StateFilter::Closed => Some(&["CLOSED", "MERGED"]),
        StateFilter::All | StateFilter::Unknown => None,
    }
}

fn summary(repo: &RepoRef, node: &Value) -> Result<PullSummary, Failure> {
    let field = |name: &str| text(&node[name]);
    let (Some(number), Some(web_url), Some(base_branch), Some(base_sha), Some(head_branch), Some(head_sha)) = (
        node["number"].as_u64(),
        field("url"),
        field("baseRefName"),
        field("baseRefOid"),
        field("headRefName"),
        field("headRefOid"),
    ) else {
        return Err(undescribed());
    };
    Ok(PullSummary {
        number,
        title: field("title").unwrap_or_default(),
        state: match node["state"].as_str() {
            Some("OPEN") => PullState::Open,
            Some("CLOSED") => PullState::Closed,
            Some("MERGED") => PullState::Merged,
            _ => PullState::Unknown,
        },
        draft: node["isDraft"].as_bool().unwrap_or(false),
        author: person(&node["author"], GRAPH_AVATAR),
        base: Tip {
            branch: base_branch,
            sha: base_sha,
            repo: Some(repo.clone()),
        },
        head: Tip {
            branch: head_branch,
            sha: head_sha,
            repo: repo_ref(&node["headRepository"]),
        },
        review: standing(node["reviewDecision"].as_str()),
        checks: rollup(node["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["state"].as_str()),
        created_at: field("createdAt").unwrap_or_default(),
        updated_at: field("updatedAt").unwrap_or_default(),
        web_url,
    })
}

fn listed_summary(pull: &Value) -> Result<PullSummary, Failure> {
    let field = |name: &str| text(&pull[name]);
    let (Some(number), Some(web_url), Some(base), Some(head)) = (
        pull["number"].as_u64(),
        field("html_url"),
        listed_tip(&pull["base"]),
        listed_tip(&pull["head"]),
    ) else {
        return Err(undescribed());
    };
    Ok(PullSummary {
        number,
        title: field("title").unwrap_or_default(),
        state: match pull["state"].as_str() {
            Some("open") => PullState::Open,
            Some("closed") if !pull["merged_at"].is_null() => PullState::Merged,
            Some("closed") => PullState::Closed,
            _ => PullState::Unknown,
        },
        draft: pull["draft"].as_bool().unwrap_or(false),
        author: person(&pull["user"], REST_AVATAR),
        base,
        head,
        review: None,
        checks: None,
        created_at: field("created_at").unwrap_or_default(),
        updated_at: field("updated_at").unwrap_or_default(),
        web_url,
    })
}

fn listed_tip(tip: &Value) -> Option<Tip> {
    Some(Tip {
        branch: text(&tip["ref"])?,
        sha: text(&tip["sha"])?,
        repo: repo_ref(&tip["repo"]),
    })
}

fn repo_ref(repository: &Value) -> Option<RepoRef> {
    Some(RepoRef {
        owner: text(&repository["owner"]["login"])?,
        name: text(&repository["name"])?,
    })
}

fn standing(decision: Option<&str>) -> Option<ReviewStanding> {
    match decision? {
        "APPROVED" => Some(ReviewStanding::Approved),
        "CHANGES_REQUESTED" => Some(ReviewStanding::ChangesRequested),
        "REVIEW_REQUIRED" => Some(ReviewStanding::ReviewRequired),
        _ => Some(ReviewStanding::Unknown),
    }
}

fn rollup(state: Option<&str>) -> Option<CheckState> {
    match state? {
        "SUCCESS" => Some(CheckState::Success),
        "FAILURE" | "ERROR" => Some(CheckState::Failure),
        "PENDING" | "EXPECTED" => Some(CheckState::Pending),
        _ => Some(CheckState::Unknown),
    }
}

fn mergeability(pull: &Value) -> Mergeability {
    let open = pull["state"].as_str() == Some("OPEN");
    let draft = pull["isDraft"].as_bool() == Some(true);
    let mergeable = pull["mergeable"].as_str();
    let status = pull["mergeStateStatus"].as_str();
    match (open, draft, mergeable, status) {
        (false, ..) => Mergeability::Unknown,
        (_, true, ..) | (.., Some("DRAFT")) => Mergeability::Draft,
        (_, _, Some("UNKNOWN"), _) | (.., Some("UNKNOWN")) => Mergeability::Checking,
        (_, _, Some("CONFLICTING"), _) | (.., Some("DIRTY")) => Mergeability::Conflicts,
        (.., Some("BEHIND")) => Mergeability::Behind,
        (.., Some("BLOCKED")) => Mergeability::Blocked {
            reason: blocked_because(pull["reviewDecision"].as_str()).to_owned(),
        },
        (.., Some("UNSTABLE")) => Mergeability::FailingChecks,
        (.., Some("CLEAN" | "HAS_HOOKS")) => Mergeability::Ready,
        _ => Mergeability::Unknown,
    }
}

fn blocked_because(decision: Option<&str>) -> &'static str {
    match decision {
        Some("REVIEW_REQUIRED") => NEEDS_APPROVAL,
        Some("CHANGES_REQUESTED") => CHANGES_REQUESTED,
        _ => RULES_BLOCK,
    }
}

fn undescribed() -> Failure {
    Failure::Protocol {
        message: "GitHub described a pull request without its number, address or branches".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use sternegit_sdk::{Failure, Merge, MergeMethod, Mergeability, NewPull, PullState};
    use serde_json::{Value, json};

    use super::{listed_summary, merging, mergeability, opening};

    fn open(mergeable: &str, status: &str, decision: Value) -> Mergeability {
        mergeability(&json!({
            "state": "OPEN",
            "isDraft": false,
            "mergeable": mergeable,
            "mergeStateStatus": status,
            "reviewDecision": decision
        }))
    }

    #[test]
    fn only_an_open_pull_request_has_a_mergeability() {
        let merged = json!({ "state": "MERGED", "isDraft": false, "mergeable": "UNKNOWN", "mergeStateStatus": "UNKNOWN" });
        assert_eq!(mergeability(&merged), Mergeability::Unknown);
    }

    #[test]
    fn a_draft_is_a_draft_before_anything_else() {
        let draft = json!({ "state": "OPEN", "isDraft": true, "mergeable": "CONFLICTING", "mergeStateStatus": "DIRTY" });
        assert_eq!(mergeability(&draft), Mergeability::Draft);
        assert_eq!(open("MERGEABLE", "DRAFT", json!(null)), Mergeability::Draft);
    }

    #[test]
    fn github_still_computing_either_answer_means_checking() {
        assert_eq!(open("UNKNOWN", "CLEAN", json!(null)), Mergeability::Checking);
        assert_eq!(open("CONFLICTING", "UNKNOWN", json!(null)), Mergeability::Checking);
    }

    #[test]
    fn conflicts_come_before_what_the_merge_state_says() {
        assert_eq!(open("CONFLICTING", "BLOCKED", json!("REVIEW_REQUIRED")), Mergeability::Conflicts);
        assert_eq!(open("MERGEABLE", "DIRTY", json!(null)), Mergeability::Conflicts);
    }

    #[test]
    fn each_merge_state_has_its_mergeability() {
        assert_eq!(open("MERGEABLE", "BEHIND", json!(null)), Mergeability::Behind);
        assert_eq!(open("MERGEABLE", "UNSTABLE", json!(null)), Mergeability::FailingChecks);
        assert_eq!(open("MERGEABLE", "CLEAN", json!("APPROVED")), Mergeability::Ready);
        assert_eq!(open("MERGEABLE", "HAS_HOOKS", json!(null)), Mergeability::Ready);
        assert_eq!(open("MERGEABLE", "QUEUED", json!(null)), Mergeability::Unknown);
    }

    #[test]
    fn a_blocked_merge_says_why_from_the_review_decision() {
        let reason = |decision: Value| match open("MERGEABLE", "BLOCKED", decision) {
            Mergeability::Blocked { reason } => reason,
            other => panic!("expected blocked, got {other:?}"),
        };
        assert_eq!(reason(json!("REVIEW_REQUIRED")), "At least one approving review is required.");
        assert_eq!(reason(json!("CHANGES_REQUESTED")), "Changes were requested in a review.");
        assert_eq!(reason(json!("APPROVED")), "A required check hasn't passed, or a branch rule blocks it.");
    }

    #[test]
    fn a_closed_pull_with_a_merge_time_was_merged() {
        let state = |state: &str, merged_at: Value| {
            let pull = json!({
                "number": 136,
                "html_url": "https://github.com/sternegit-org/sternegit/pull/136",
                "state": state,
                "merged_at": merged_at,
                "user": null,
                "base": { "ref": "main", "sha": "7d3f9b1e", "repo": { "name": "sternegit", "owner": { "login": "sternegit-org" } } },
                "head": { "ref": "quiet-fetch", "sha": "e5a9c3f7", "repo": null }
            });
            listed_summary(&pull).unwrap().state
        };
        assert_eq!(state("closed", json!("2026-10-24T17:03:39Z")), PullState::Merged);
        assert_eq!(state("closed", json!(null)), PullState::Closed);
        assert_eq!(state("open", json!(null)), PullState::Open);
    }

    #[test]
    fn a_pull_from_another_owner_names_its_head_as_owner_colon_branch() {
        let new = |head_owner: Option<&str>| NewPull {
            base: "main".into(),
            head: "topic/tabs".into(),
            head_owner: head_owner.map(str::to_owned),
            title: "Open files in tabs".into(),
            body: String::new(),
            draft: true,
        };
        assert_eq!(opening(new(Some("sam-lee")))["head"], "sam-lee:topic/tabs");
        assert_eq!(
            opening(new(None)),
            json!({ "title": "Open files in tabs", "head": "topic/tabs", "base": "main", "body": "", "draft": true })
        );
    }

    #[test]
    fn a_merge_guards_the_head_and_sends_only_the_title_and_message_given() {
        let merge = |method: MergeMethod, title: Option<&str>| Merge {
            number: 142,
            method,
            title: title.map(str::to_owned),
            message: None,
            expected_head: "4f1d2c9".into(),
        };
        assert_eq!(
            merging(merge(MergeMethod::Squash, Some("Show pull requests (#142)"))).unwrap(),
            json!({ "merge_method": "squash", "sha": "4f1d2c9", "commit_title": "Show pull requests (#142)" })
        );
        assert_eq!(merging(merge(MergeMethod::Rebase, None)).unwrap(), json!({ "merge_method": "rebase", "sha": "4f1d2c9" }));
        assert_eq!(merging(merge(MergeMethod::FastForward, None)), Err(Failure::Unsupported));
    }
}
