use sternegit_sdk::{Comment, ConversationQuery, Failure, NewComment, Page, RepoRef, Request};
use serde_json::{Value, json};

use crate::{
    api,
    moments::moment,
    place::{Collection, VERSION},
    pulls::person,
    refusal::succeeded,
    repositories::listed,
};

const TEXT: &str = "text";
const ACTIVE: i64 = 1;

pub(crate) fn list(collection: &Collection, repo: &RepoRef, query: ConversationQuery) -> Result<Page<Comment>, Failure> {
    let url = format!("{}/pullRequests/{}/threads?{VERSION}", collection.repository(repo), query.number);
    let answer: Value = succeeded(api(Request::get(url)).fetch())?.json()?;
    let mut comments: Vec<Comment> = listed(&answer)?
        .iter()
        .filter(|thread| thread["threadContext"].is_null() && thread["isDeleted"].as_bool() != Some(true))
        .flat_map(|thread| {
            thread["comments"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|comment| comment["commentType"].as_str() == Some(TEXT))
                .filter(|comment| comment["isDeleted"].as_bool() != Some(true))
                .filter_map(move |comment| written(thread, comment))
        })
        .collect();
    comments.sort_by(|one, other| one.created_at.cmp(&other.created_at));
    Ok(Page { items: comments, next: None })
}

pub(crate) fn add(collection: &Collection, repo: &RepoRef, new: NewComment) -> Result<Comment, Failure> {
    let url = format!("{}/pullRequests/{}/threads?{VERSION}", collection.repository(repo), new.number);
    let body = json!({
        "comments": [{ "parentCommentId": 0, "content": new.body, "commentType": 1 }],
        "status": ACTIVE,
    });
    let thread: Value = succeeded(api(Request::post(url)).json(&body).fetch())?.json()?;
    thread["comments"]
        .as_array()
        .and_then(|comments| comments.first())
        .and_then(|comment| written(&thread, comment))
        .ok_or_else(|| Failure::Protocol {
            message: "Azure DevOps kept the comment but didn't return it".to_owned(),
        })
}

fn written(thread: &Value, comment: &Value) -> Option<Comment> {
    let thread_id = thread["id"].as_u64()?;
    let id = comment["id"].as_u64()?;
    Some(Comment {
        id: format!("{thread_id}.{id}"),
        author: person(&comment["author"]),
        body: comment["content"].as_str().unwrap_or_default().to_owned(),
        created_at: moment(&comment["publishedDate"]).unwrap_or_default(),
        reply_to: comment["parentCommentId"]
            .as_u64()
            .filter(|parent| *parent != 0)
            .map(|parent| format!("{thread_id}.{parent}")),
        web_url: None,
    })
}
