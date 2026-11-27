mod files;
mod model;

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

pub use files::{Found, Search};
pub use model::{Field, Icon, Model, Object, Part, Reference, Role, Scope, Value};

pub const ABI: &str = "sternegit:1";
pub const PROTOCOL: &str = "1.0";

pub fn speaks_same_major(protocol: &str) -> bool {
    let major = |version: &str| version.split('.').next().map(str::to_owned);
    major(protocol) == major(PROTOCOL)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "input", rename_all = "snake_case")]
pub enum Call {
    Hello(Hello),
    Read(ReadInput),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: String,
    pub sdk: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Before,
    After,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadInput {
    pub kind: String,
    pub path: String,
    pub side: Side,
    #[serde(default)]
    pub options: BTreeMap<String, bool>,
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Failure {
    NotThisFormat { message: String },
    Missing { path: String },
    Unsupported,
    Protocol { message: String },
    #[serde(other)]
    Unknown,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::NotThisFormat { message } => f.write_str(message),
            Failure::Missing { path } => write!(f, "{path} isn't there"),
            Failure::Unsupported => f.write_str("Not available"),
            Failure::Protocol { message } => write!(f, "Misunderstood: {message}"),
            Failure::Unknown => f.write_str("Failed for a reason this SterneGit doesn't know"),
        }
    }
}

impl std::error::Error for Failure {}

#[cfg(test)]
mod tests {
    use super::{Answer, Call, Failure, ReadInput, Side, speaks_same_major};

    #[test]
    fn a_read_call_names_its_op_and_carries_its_input() {
        let call = Call::Read(ReadInput {
            kind: "prefab".into(),
            path: "Assets/Player.prefab".into(),
            side: Side::After,
            options: Default::default(),
        });
        let json = serde_json::to_value(&call).unwrap();
        assert_eq!(json["op"], "read");
        assert_eq!(json["input"]["side"], "after");
        assert_eq!(serde_json::from_value::<Call>(json).unwrap(), call);
    }

    #[test]
    fn an_unknown_failure_still_reads() {
        let answer: Answer<()> = serde_json::from_str(r#"{"err":{"kind":"from_the_future"}}"#).unwrap();
        assert_eq!(answer.into_result(), Err(Failure::Unknown));
    }

    #[test]
    fn minor_versions_speak_together() {
        assert!(speaks_same_major("1.4"));
        assert!(!speaks_same_major("2.0"));
    }
}
