use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Person, Timestamp, Verdict};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub old: Option<u32>,
    pub new: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spot {
    pub path: String,
    pub old_path: Option<String>,
    pub first: Option<Line>,
    pub last: Option<Line>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub spot: Spot,
    pub commit: String,
    pub extra: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: Person,
    pub body: String,
    pub created_at: Timestamp,
    pub reply_to: Option<String>,
    pub web_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub anchor: Anchor,
    pub outdated: bool,
    pub resolved: bool,
    pub can_resolve: bool,
    pub can_reply: bool,
    pub comments: Vec<Comment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub spot: Spot,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub number: u64,
    pub commit: String,
    pub verdict: Verdict,
    pub body: String,
    pub drafts: Vec<Draft>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewReply {
    pub number: u64,
    pub thread: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub number: u64,
    pub thread: String,
    pub resolved: bool,
}
