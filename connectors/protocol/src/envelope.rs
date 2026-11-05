use serde::{Deserialize, Serialize};

use crate::{AccountContext, Failure, RepoRef};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Call<T> {
    pub op: String,
    pub account: Option<AccountContext>,
    pub repo: Option<RepoRef>,
    pub input: T,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer<T> {
    Ok(T),
    Err(Failure),
}

impl<T> Answer<T> {
    pub fn into_result(self) -> Result<T, Failure> {
        match self {
            Answer::Ok(value) => Ok(value),
            Answer::Err(failure) => Err(failure),
        }
    }
}

impl<T> From<Result<T, Failure>> for Answer<T> {
    fn from(result: Result<T, Failure>) -> Self {
        match result {
            Ok(value) => Answer::Ok(value),
            Err(failure) => Answer::Err(failure),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: String,
    pub sdk: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Done {}
