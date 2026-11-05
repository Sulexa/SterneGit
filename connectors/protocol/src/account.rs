use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountContext {
    pub id: String,
    pub login: String,
    pub endpoints: BTreeMap<String, String>,
}

impl AccountContext {
    pub fn endpoint(&self, name: &str) -> Option<&str> {
        self.endpoints.get(name).map(String::as_str)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub scopes: Vec<String>,
}
