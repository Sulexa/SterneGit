use sternegit_sdk::{AccountContext, Failure, Request, Response};
use serde_json::{Value, json};

use crate::refusal::{rate_limit_wait, succeeded};

const GRAPHQL_ENDPOINT: &str = "graphql";
const MERGE_INFO_PREVIEW: &str = "application/vnd.github.merge-info-preview+json";
const NOT_FOUND: &str = "NOT_FOUND";
const FORBIDDEN: &str = "FORBIDDEN";
const RATE_LIMITED: &str = "RATE_LIMITED";
const WAIT_WHEN_THE_RESET_IS_UNKNOWN_SECONDS: u64 = 60;

pub(crate) fn query(account: &AccountContext, document: &str, variables: Value) -> Result<Value, Failure> {
    let url = account.endpoint(GRAPHQL_ENDPOINT).ok_or_else(|| Failure::Protocol {
        message: "This account names no GraphQL endpoint".to_owned(),
    })?;
    let body = json!({ "query": document, "variables": variables });
    let response = succeeded(Request::post(url).header("Accept", MERGE_INFO_PREVIEW).json(&body).fetch())?;
    let mut answer: Value = response.json()?;
    match answer["errors"].as_array() {
        Some(errors) if !errors.is_empty() => Err(failure(&response, errors)),
        _ => Ok(answer.get_mut("data").map(Value::take).unwrap_or_default()),
    }
}

pub(crate) fn found(value: &Value) -> Result<&Value, Failure> {
    match value {
        Value::Null => Err(Failure::NotFound),
        found => Ok(found),
    }
}

fn failure(response: &Response, errors: &[Value]) -> Failure {
    let typed = |kind: &str| errors.iter().find(|error| error["type"].as_str() == Some(kind));
    let message = |error: &Value| {
        error["message"]
            .as_str()
            .unwrap_or("GitHub gave no reason")
            .to_owned()
    };
    if typed(NOT_FOUND).is_some() {
        return Failure::NotFound;
    }
    if let Some(error) = typed(FORBIDDEN) {
        return Failure::Forbidden {
            needs: None,
            message: message(error),
        };
    }
    if typed(RATE_LIMITED).is_some() {
        return Failure::RateLimited {
            retry_after_seconds: rate_limit_wait(response).unwrap_or(WAIT_WHEN_THE_RESET_IS_UNKNOWN_SECONDS),
        };
    }
    Failure::Protocol {
        message: errors.first().map(message).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use sternegit_sdk::{Failure, Response};
    use serde_json::{Value, json};

    use super::failure;

    fn answered(errors: Value) -> Failure {
        let response = Response {
            status: 200,
            headers: Vec::new(),
            body: Vec::new(),
        };
        failure(&response, errors.as_array().unwrap())
    }

    #[test]
    fn a_missing_object_anywhere_in_the_answer_is_not_found() {
        let errors = json!([
            { "type": "FORBIDDEN", "message": "Resource not accessible by integration" },
            { "type": "NOT_FOUND", "message": "Could not resolve to a PullRequest with the number of 9999." }
        ]);
        assert_eq!(answered(errors), Failure::NotFound);
    }

    #[test]
    fn a_forbidden_field_keeps_githubs_message() {
        let errors = json!([{ "type": "FORBIDDEN", "message": "Resource not accessible by personal access token" }]);
        assert_eq!(
            answered(errors),
            Failure::Forbidden {
                needs: None,
                message: "Resource not accessible by personal access token".into(),
            }
        );
    }

    #[test]
    fn an_exhausted_graphql_budget_is_rate_limited() {
        let errors = json!([{ "type": "RATE_LIMITED", "message": "API rate limit exceeded for user ID 4120511." }]);
        assert_eq!(answered(errors), Failure::RateLimited { retry_after_seconds: 60 });
    }

    #[test]
    fn any_other_error_is_a_misunderstanding_in_githubs_words() {
        let errors = json!([{ "message": "Field 'mergeQueue' doesn't exist on type 'Repository'" }]);
        assert_eq!(
            answered(errors),
            Failure::Protocol {
                message: "Field 'mergeQueue' doesn't exist on type 'Repository'".into(),
            }
        );
    }
}
