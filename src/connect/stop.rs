use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Refused(String),
    Trapped { message: String, log: Vec<String> },
    OutOfFuel,
    OutOfMemory,
    BadOutput {
        op: &'static str,
        path: String,
        problem: String,
    },
    Cancelled,
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stop::Refused(reason) => f.write_str(reason),
            Stop::Trapped { message, .. } => write!(f, "The connector stopped: {message}"),
            Stop::OutOfFuel => f.write_str("The connector ran too long and was stopped"),
            Stop::OutOfMemory => {
                f.write_str("The connector needed more than 64 MB of memory and was stopped")
            }
            Stop::BadOutput { op, path, problem } => {
                write!(f, "The connector's answer to {op} can't be read at {path}: {problem}")
            }
            Stop::Cancelled => f.write_str("Cancelled"),
        }
    }
}
