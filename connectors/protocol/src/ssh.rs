use serde::{Deserialize, Serialize};

use crate::Timestamp;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshKey {
    pub id: String,
    pub title: String,
    pub key: String,
    pub created_at: Option<Timestamp>,
    pub read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewSshKey {
    pub title: String,
    pub key: String,
}
