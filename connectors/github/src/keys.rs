use fastgit_sdk::{AccountContext, Failure, NewSshKey, Request, SshKey};
use serde_json::{Value, json};

use crate::{
    api,
    listing::{every_page, id_of},
    refusal::succeeded,
};

pub(crate) fn list(account: &AccountContext) -> Result<Vec<SshKey>, Failure> {
    every_page(account, "user/keys?per_page=100")?
        .iter()
        .map(key)
        .collect()
}

pub(crate) fn add(new: NewSshKey) -> Result<SshKey, Failure> {
    let body = json!({ "title": new.title, "key": new.key });
    key(&succeeded(api(Request::post("user/keys")).json(&body).fetch())?.json()?)
}

pub(crate) fn host_keys() -> Result<Vec<String>, Failure> {
    let meta: Value = succeeded(api(Request::get("meta")).fetch())?.json()?;
    Ok(meta["ssh_keys"]
        .as_array()
        .map(|keys| keys.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default())
}

fn key(item: &Value) -> Result<SshKey, Failure> {
    let text = |name: &str| item[name].as_str().map(str::to_owned);
    let (Some(id), Some(key)) = (id_of(&item["id"]), text("key")) else {
        return Err(Failure::Protocol {
            message: "GitHub listed an SSH key without its id or key".to_owned(),
        });
    };
    Ok(SshKey {
        id,
        title: text("title").unwrap_or_default(),
        key,
        created_at: text("created_at"),
        read_only: item["read_only"].as_bool().unwrap_or(false),
    })
}
