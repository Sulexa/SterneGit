use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Failure {
    SignedOut,
    Forbidden {
        needs: Option<String>,
        message: String,
    },
    RateLimited {
        retry_after_seconds: u64,
    },
    NotFound,
    Invalid {
        fields: Vec<FieldError>,
        message: String,
    },
    Conflict {
        message: String,
    },
    Refused {
        status: u16,
        message: String,
    },
    Unsupported,
    Unfinished {
        posted: u32,
        failure: Box<Failure>,
    },
    Protocol {
        message: String,
    },
    #[serde(other)]
    Unknown,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::SignedOut => f.write_str("Signed out"),
            Failure::Forbidden { needs: Some(needs), message } => {
                write!(f, "Not allowed without {needs}: {message}")
            }
            Failure::Forbidden { needs: None, message } => write!(f, "Not allowed: {message}"),
            Failure::RateLimited { retry_after_seconds } => {
                write!(f, "Too many requests; try again in {retry_after_seconds} s")
            }
            Failure::NotFound => f.write_str("Not found"),
            Failure::Invalid { fields, message } => {
                f.write_str(message)?;
                fields
                    .iter()
                    .try_for_each(|field| write!(f, "; {}: {}", field.field, field.message))
            }
            Failure::Conflict { message } => f.write_str(message),
            Failure::Refused { status, message } => write!(f, "Refused with {status}: {message}"),
            Failure::Unsupported => f.write_str("Not available"),
            Failure::Unfinished { posted, failure } => {
                write!(f, "Stopped after posting {posted}: {failure}")
            }
            Failure::Protocol { message } => write!(f, "Misunderstood: {message}"),
            Failure::Unknown => f.write_str("Failed for a reason this FastGit doesn't know"),
        }
    }
}

impl std::error::Error for Failure {}
