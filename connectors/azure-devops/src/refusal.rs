use sternegit_sdk::{Failure, Response};
use serde_json::Value;

const RETRY_AFTER: &str = "retry-after";
const SIGN_IN_PAGE: u16 = 203;
const LONGEST_REASON: usize = 200;

pub(crate) fn succeeded(response: Response) -> Result<Response, Failure> {
    match response.succeeded() && response.status != SIGN_IN_PAGE {
        true => Ok(response),
        false => Err(failure(&response)),
    }
}

pub(crate) fn failure(response: &Response) -> Failure {
    if let Some(retry_after_seconds) = retry_after(response) {
        return Failure::RateLimited { retry_after_seconds };
    }
    match response.status {
        401 | SIGN_IN_PAGE => Failure::SignedOut,
        403 => Failure::Forbidden {
            needs: None,
            message: reason(response),
        },
        404 => Failure::NotFound,
        409 => Failure::Conflict {
            message: reason(response),
        },
        400 => Failure::Invalid {
            fields: Vec::new(),
            message: reason(response),
        },
        status => Failure::Refused {
            status,
            message: reason(response),
        },
    }
}

fn retry_after(response: &Response) -> Option<u64> {
    let seconds = response
        .header(RETRY_AFTER)
        .and_then(|value| value.trim().parse::<u64>().ok());
    match response.status {
        429 => Some(seconds.unwrap_or(60).max(1)),
        503 => seconds.map(|seconds| seconds.max(1)),
        _ => None,
    }
}

pub(crate) fn reason(response: &Response) -> String {
    let body: Value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
    if let Some(message) = body["message"].as_str().filter(|message| !message.trim().is_empty()) {
        return message.trim().to_owned();
    }
    let text = response.text();
    match text.trim() {
        "" => "The server gave no reason".to_owned(),
        text if text.starts_with('<') => "The server answered with a web page instead of data".to_owned(),
        text => text.chars().take(LONGEST_REASON).collect(),
    }
}

#[cfg(test)]
mod tests {
    use sternegit_sdk::{Failure, Response};
    use serde_json::json;

    use super::{failure, succeeded};

    fn response(status: u16, headers: &[(&str, &str)], body: &str) -> Response {
        Response {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            body: body.as_bytes().to_vec(),
        }
    }

    fn azure(message: &str) -> String {
        json!({ "$id": "1", "innerException": null, "message": message, "typeKey": "SomeException", "errorCode": 0 }).to_string()
    }

    #[test]
    fn a_rejected_token_or_a_sign_in_page_means_signed_out() {
        assert_eq!(failure(&response(401, &[], "")), Failure::SignedOut);
        let page = response(203, &[("content-type", "text/html")], "<html>Sign in</html>");
        assert_eq!(succeeded(page).unwrap_err(), Failure::SignedOut);
    }

    #[test]
    fn throttling_waits_as_long_as_azure_devops_asks() {
        assert_eq!(failure(&response(429, &[("Retry-After", "12")], "")), Failure::RateLimited { retry_after_seconds: 12 });
        assert_eq!(failure(&response(429, &[], "")), Failure::RateLimited { retry_after_seconds: 60 });
        assert_eq!(failure(&response(503, &[("retry-after", "0")], "")), Failure::RateLimited { retry_after_seconds: 1 });
    }

    #[test]
    fn refusals_keep_azure_devops_own_words() {
        let taken = azure("TF400948: A Git repository with the name app already exists.");
        assert_eq!(
            failure(&response(409, &[], &taken)),
            Failure::Conflict { message: "TF400948: A Git repository with the name app already exists.".into() }
        );
        let bad = azure("TF401398: The pull request cannot be activated because the source and/or the target branch no longer exists.");
        assert!(matches!(failure(&response(400, &[], &bad)), Failure::Invalid { fields, message } if fields.is_empty() && message.starts_with("TF401398")));
        assert_eq!(failure(&response(404, &[], &azure("TF401019"))), Failure::NotFound);
        assert_eq!(
            failure(&response(403, &[], &azure("TF401027: You need the Git 'PullRequestContribute' permission."))),
            Failure::Forbidden { needs: None, message: "TF401027: You need the Git 'PullRequestContribute' permission.".into() }
        );
        assert_eq!(
            failure(&response(500, &[], "<!DOCTYPE html><html>oops</html>")),
            Failure::Refused { status: 500, message: "The server answered with a web page instead of data".into() }
        );
    }
}
