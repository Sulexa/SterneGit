use fastgit_sdk::{AccountContext, Comment, ConversationQuery, Failure, NewComment, Page, RepoRef, Request};
use serde_json::{Value, json};

use crate::{
    api,
    listing::{cursor_on_api, id_of, on_api, repo_path, text},
    pulls::{REST_AVATAR, person},
    refusal::succeeded,
};

pub(crate) fn list(account: &AccountContext, repo: &RepoRef, query: ConversationQuery) -> Result<Page<Comment>, Failure> {
    let url = match query.cursor {
        Some(cursor) => cursor_on_api(account, cursor)?,
        None => format!("{}/issues/{}/comments?per_page=100", repo_path(repo), query.number),
    };
    let response = succeeded(api(Request::get(url)).fetch())?;
    let comments: Vec<Value> = response.json()?;
    Ok(Page {
        items: comments.iter().map(comment).collect::<Result<_, _>>()?,
        next: on_api(account, response.next_page()),
    })
}

pub(crate) fn add(repo: &RepoRef, new: NewComment) -> Result<Comment, Failure> {
    let url = format!("{}/issues/{}/comments", repo_path(repo), new.number);
    let body = json!({ "body": new.body });
    comment(&succeeded(api(Request::post(url)).json(&body).fetch())?.json()?)
}

fn comment(item: &Value) -> Result<Comment, Failure> {
    let id = id_of(&item["id"]).ok_or_else(|| Failure::Protocol {
        message: "GitHub listed a comment without its id".to_owned(),
    })?;
    Ok(Comment {
        id,
        author: person(&item["user"], REST_AVATAR),
        body: text(&item["body"]).unwrap_or_default(),
        created_at: text(&item["created_at"]).unwrap_or_default(),
        reply_to: None,
        web_url: text(&item["html_url"]),
    })
}
