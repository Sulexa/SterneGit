#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Refused(String),
    Trapped { message: String, log: Vec<String> },
    OutOfFuel,
    OutOfMemory { limit_bytes: usize },
    BadOutput {
        op: &'static str,
        path: String,
        problem: String,
    },
    Cancelled,
}

impl Stop {
    pub(crate) fn told_of(&self, plugin: &str) -> String {
        match self {
            Stop::Refused(reason) => reason.clone(),
            Stop::Trapped { message, .. } => format!("The {plugin} stopped: {message}"),
            Stop::OutOfFuel => format!("The {plugin} ran too long and was stopped"),
            Stop::OutOfMemory { limit_bytes } => format!(
                "The {plugin} needed more than {} MB of memory and was stopped",
                limit_bytes / (1024 * 1024)
            ),
            Stop::BadOutput { op, path, problem } => {
                format!("The {plugin}'s answer to {op} can't be read at {path}: {problem}")
            }
            Stop::Cancelled => "Cancelled".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stop;

    #[test]
    fn a_stop_names_the_kind_of_plugin_and_its_memory_limit() {
        assert_eq!(
            Stop::OutOfMemory { limit_bytes: 64 * 1024 * 1024 }.told_of("connector"),
            "The connector needed more than 64 MB of memory and was stopped"
        );
        assert_eq!(Stop::OutOfFuel.told_of("reader"), "The reader ran too long and was stopped");
        assert_eq!(Stop::Refused("No module".into()).told_of("reader"), "No module");
    }
}
