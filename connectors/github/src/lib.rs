mod refusal;

use fastgit_sdk::{AccountContext, Connector, Failure, Identity, Request, Response, export};
use serde_json::Value;

use refusal::succeeded;

const MEDIA_TYPE: &str = "application/vnd.github+json";
const API_VERSION: &str = "2022-11-28";
const GRANTED_SCOPES: &str = "x-oauth-scopes";

pub struct GitHub;

impl Connector for GitHub {
    fn who_am_i(&self, _account: &AccountContext) -> Result<Identity, Failure> {
        let response = succeeded(api(Request::get("user")).fetch())?;
        let user: Value = response.json()?;
        let text = |name: &str| user[name].as_str().map(str::to_owned);
        Ok(Identity {
            login: text("login").ok_or_else(|| Failure::Protocol {
                message: "GitHub named no login for this token".to_owned(),
            })?,
            name: text("name"),
            avatar_url: text("avatar_url"),
            scopes: granted_scopes(&response),
        })
    }
}

export!(GitHub);

fn api(request: Request) -> Request {
    request
        .header("Accept", MEDIA_TYPE)
        .header("X-GitHub-Api-Version", API_VERSION)
}

fn granted_scopes(response: &Response) -> Vec<String> {
    response
        .header(GRANTED_SCOPES)
        .map(|scopes| {
            scopes
                .split(',')
                .map(str::trim)
                .filter(|scope| !scope.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
