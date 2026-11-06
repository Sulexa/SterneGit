use fastgit_sdk::{Failure, FieldError, Response, now};
use serde_json::Value;

const RETRY_AFTER: &str = "retry-after";
const REQUESTS_LEFT: &str = "x-ratelimit-remaining";
const LIMIT_RESETS_AT: &str = "x-ratelimit-reset";
const GRANTED_SCOPES: &str = "x-oauth-scopes";
const ACCEPTED_SCOPES: &str = "x-accepted-oauth-scopes";
const ACCEPTED_PERMISSIONS: &str = "x-accepted-github-permissions";
const WAIT_WHEN_THE_RESET_IS_UNKNOWN_SECONDS: i64 = 60;
const LONGEST_REASON: usize = 200;

pub(crate) fn succeeded(response: Response) -> Result<Response, Failure> {
    match response.succeeded() {
        true => Ok(response),
        false => Err(failure(&response)),
    }
}

pub(crate) fn failure(response: &Response) -> Failure {
    if matches!(response.status, 403 | 429) {
        if let Some(retry_after_seconds) = rate_limit_wait(response) {
            return Failure::RateLimited { retry_after_seconds };
        }
    }
    match response.status {
        401 => Failure::SignedOut,
        403 => Failure::Forbidden {
            needs: missing_permissions(response).or_else(|| missing_scope(response)),
            message: reason(response),
        },
        404 => match missing_scope(response) {
            Some(needs) => Failure::Forbidden {
                needs: Some(needs),
                message: reason(response),
            },
            None => Failure::NotFound,
        },
        405 | 409 => Failure::Conflict {
            message: reason(response),
        },
        422 => Failure::Invalid {
            fields: fields(response),
            message: unattributed_reason(response).unwrap_or_else(|| reason(response)),
        },
        status => Failure::Refused {
            status,
            message: reason(response),
        },
    }
}

pub(crate) fn rate_limit_wait(response: &Response) -> Option<u64> {
    if let Some(seconds) = response
        .header(RETRY_AFTER)
        .and_then(|value| value.trim().parse().ok())
    {
        return Some(seconds);
    }
    if response.header(REQUESTS_LEFT)?.trim() != "0" {
        return None;
    }
    let seconds_left = response
        .header(LIMIT_RESETS_AT)
        .and_then(|value| value.trim().parse::<i64>().ok())
        .map_or(WAIT_WHEN_THE_RESET_IS_UNKNOWN_SECONDS, |resets_at| {
            resets_at - now() / 1000
        });
    Some(seconds_left.max(1).unsigned_abs())
}

fn missing_permissions(response: &Response) -> Option<String> {
    response
        .header(ACCEPTED_PERMISSIONS)
        .map(str::trim)
        .filter(|permissions| !permissions.is_empty())
        .map(str::to_owned)
}

fn missing_scope(response: &Response) -> Option<String> {
    let listed = |name: &str| -> Vec<&str> {
        response
            .header(name)
            .map(|scopes| scopes.split(',').map(str::trim).filter(|scope| !scope.is_empty()).collect())
            .unwrap_or_default()
    };
    let accepted = listed(ACCEPTED_SCOPES);
    let granted = listed(GRANTED_SCOPES);
    let lacks_all = !accepted.is_empty() && !accepted.iter().any(|scope| granted.contains(scope));
    lacks_all.then(|| accepted.join(" or "))
}

fn body(response: &Response) -> Value {
    serde_json::from_slice(&response.body).unwrap_or(Value::Null)
}

fn reason(response: &Response) -> String {
    if let Some(message) = body(response)["message"].as_str() {
        return message.to_owned();
    }
    let text = response.text();
    match text.trim() {
        "" => "The server gave no reason".to_owned(),
        text => text.chars().take(LONGEST_REASON).collect(),
    }
}

fn unattributed_reason(response: &Response) -> Option<String> {
    let body = body(response);
    let messages: Vec<&str> = body["errors"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|error| error["field"].is_null())
        .filter_map(|error| error["message"].as_str())
        .collect();
    (!messages.is_empty()).then(|| messages.join(" "))
}

fn fields(response: &Response) -> Vec<FieldError> {
    let body = body(response);
    let Some(errors) = body["errors"].as_array() else {
        return Vec::new();
    };
    errors
        .iter()
        .filter_map(|error| {
            Some(FieldError {
                field: error["field"].as_str()?.to_owned(),
                message: error["message"]
                    .as_str()
                    .or(error["code"].as_str())
                    .unwrap_or("invalid")
                    .to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use fastgit_sdk::{Failure, FieldError, Response, now};
    use serde_json::json;

    use super::failure;

    fn response(status: u16, headers: &[(&str, &str)], body: serde_json::Value) -> Response {
        Response {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: body.to_string().into_bytes(),
        }
    }

    #[test]
    fn a_rejected_token_means_signed_out() {
        assert_eq!(failure(&response(401, &[], json!({ "message": "Bad credentials" }))), Failure::SignedOut);
    }

    #[test]
    fn an_exhausted_rate_limit_waits_until_its_reset() {
        let resets_at = (now() / 1000 + 90).to_string();
        let limited = response(
            403,
            &[("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", &resets_at)],
            json!({ "message": "API rate limit exceeded" }),
        );
        match failure(&limited) {
            Failure::RateLimited { retry_after_seconds } => {
                assert!((85..=90).contains(&retry_after_seconds), "{retry_after_seconds}")
            }
            other => panic!("expected a rate limit, got {other:?}"),
        }
    }

    #[test]
    fn a_secondary_rate_limit_says_how_long_to_wait() {
        let limited = response(429, &[("retry-after", "30")], json!({}));
        assert_eq!(failure(&limited), Failure::RateLimited { retry_after_seconds: 30 });
    }

    #[test]
    fn a_forbidden_request_names_the_permission_a_fine_grained_token_lacks() {
        let forbidden = response(
            403,
            &[("x-accepted-github-permissions", "pull_requests=write")],
            json!({ "message": "Resource not accessible by personal access token" }),
        );
        assert_eq!(
            failure(&forbidden),
            Failure::Forbidden {
                needs: Some("pull_requests=write".into()),
                message: "Resource not accessible by personal access token".into(),
            }
        );
    }

    #[test]
    fn a_not_found_from_a_missing_scope_is_forbidden_and_names_the_scope() {
        let hidden = response(
            404,
            &[("x-oauth-scopes", "read:org"), ("x-accepted-oauth-scopes", "repo, public_repo")],
            json!({ "message": "Not Found" }),
        );
        assert_eq!(
            failure(&hidden),
            Failure::Forbidden {
                needs: Some("repo or public_repo".into()),
                message: "Not Found".into(),
            }
        );
        let missing = response(
            404,
            &[("x-oauth-scopes", "repo"), ("x-accepted-oauth-scopes", "repo")],
            json!({ "message": "Not Found" }),
        );
        assert_eq!(failure(&missing), Failure::NotFound);
    }

    #[test]
    fn an_invalid_request_lists_each_field() {
        let invalid = response(
            422,
            &[],
            json!({
                "message": "Validation Failed",
                "errors": [
                    { "resource": "Repository", "field": "name", "code": "already_exists", "message": "name already exists on this account" },
                    { "resource": "Repository", "field": "visibility", "code": "invalid" },
                    "a bare string GitHub sometimes sends"
                ]
            }),
        );
        assert_eq!(
            failure(&invalid),
            Failure::Invalid {
                fields: vec![
                    FieldError { field: "name".into(), message: "name already exists on this account".into() },
                    FieldError { field: "visibility".into(), message: "invalid".into() },
                ],
                message: "Validation Failed".into(),
            }
        );
    }

    #[test]
    fn an_invalid_request_without_a_field_says_what_github_found_wrong() {
        let duplicate = response(
            422,
            &[],
            json!({
                "message": "Validation Failed",
                "errors": [
                    { "resource": "PullRequest", "code": "custom", "message": "A pull request already exists for fastgit-org:topic/tabs." }
                ]
            }),
        );
        assert_eq!(
            failure(&duplicate),
            Failure::Invalid {
                fields: Vec::new(),
                message: "A pull request already exists for fastgit-org:topic/tabs.".into(),
            }
        );
    }

    #[test]
    fn conflicts_and_other_refusals_keep_the_servers_reason() {
        assert_eq!(
            failure(&response(409, &[], json!({ "message": "Head branch was modified" }))),
            Failure::Conflict { message: "Head branch was modified".into() }
        );
        let unavailable = Response {
            status: 502,
            headers: Vec::new(),
            body: b"  Bad gateway  ".to_vec(),
        };
        assert_eq!(
            failure(&unavailable),
            Failure::Refused { status: 502, message: "Bad gateway".into() }
        );
    }
}
