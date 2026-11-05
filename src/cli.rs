use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    accounts,
    git::{GIT_CREDENTIAL_FLAG, Repo},
};

const USAGE: &str = "Usage: fastgit [--check] [repository]";
const HELP: &str = "Brings back the tabs open last time, plus the repository containing the given path.\n\
                    --check reads the history without opening a window and reports how long it took.";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Invocation {
    Help,
    Misuse,
    Check(PathBuf),
    Open(Option<PathBuf>),
    GitCredential { account: String, operation: String },
}

pub(crate) fn parse(arguments: impl IntoIterator<Item = OsString>) -> Invocation {
    let mut arguments = arguments.into_iter().peekable();
    if arguments
        .next_if(|first| first == GIT_CREDENTIAL_FLAG)
        .is_some()
    {
        return git_credential(arguments.collect());
    }
    let mut check = false;
    let mut path = None;
    for argument in arguments {
        if argument == "--help" || argument == "-h" {
            return Invocation::Help;
        } else if argument == "--check" {
            check = true;
        } else if path.is_none() && !argument.to_string_lossy().starts_with('-') {
            path = Some(PathBuf::from(argument));
        } else {
            return Invocation::Misuse;
        }
    }
    if check {
        Invocation::Check(path.unwrap_or_else(|| PathBuf::from(".")))
    } else {
        Invocation::Open(path)
    }
}

fn git_credential(rest: Vec<OsString>) -> Invocation {
    let Ok([account, operation]) = <[OsString; 2]>::try_from(rest) else {
        return Invocation::Misuse;
    };
    match (account.into_string(), operation.into_string()) {
        (Ok(account), Ok(operation)) => Invocation::GitCredential { account, operation },
        _ => Invocation::Misuse,
    }
}

pub(crate) fn answer_git_credential(account: &str, operation: &str) {
    let answered = accounts::answer_git(
        account,
        operation,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
    );
    if answered.is_err() {
        std::process::exit(1);
    }
}

pub(crate) fn print_help() {
    println!("{USAGE}\n{HELP}");
}

pub(crate) fn exit_with_usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2)
}

pub(crate) fn check(path: &Path) {
    match Repo::open(path) {
        Ok((repo, history)) => println!(
            "{}: {} commits{}, {} refs, {} lanes wide, read in {:.1?}",
            repo.name(),
            history.commit_count(),
            if history.truncated {
                " (newest only)"
            } else {
                ""
            },
            history.refs.len(),
            history.graph.max_lanes,
            history.elapsed
        ),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{Invocation, parse};

    fn parsed(arguments: &[&str]) -> Invocation {
        parse(arguments.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_open_no_repository_beyond_the_saved_tabs() {
        assert_eq!(parsed(&[]), Invocation::Open(None));
    }

    #[test]
    fn a_path_opens_that_repository() {
        assert_eq!(
            parsed(&["repo"]),
            Invocation::Open(Some(PathBuf::from("repo")))
        );
    }

    #[test]
    fn check_reads_the_current_directory_unless_given_a_path() {
        assert_eq!(parsed(&["--check"]), Invocation::Check(PathBuf::from(".")));
        assert_eq!(
            parsed(&["repo", "--check"]),
            Invocation::Check(PathBuf::from("repo"))
        );
    }

    #[test]
    fn help_wins_over_everything_before_it() {
        assert_eq!(parsed(&["--check", "-h"]), Invocation::Help);
        assert_eq!(parsed(&["repo", "--help"]), Invocation::Help);
    }

    #[test]
    fn a_second_path_or_an_unknown_flag_is_misuse() {
        assert_eq!(parsed(&["one", "two"]), Invocation::Misuse);
        assert_eq!(parsed(&["--verbose"]), Invocation::Misuse);
    }

    #[test]
    fn git_reaches_the_credential_helper_through_a_flag_that_must_come_first() {
        assert_eq!(
            parsed(&["--git-credential", "github:github.com:1", "get"]),
            Invocation::GitCredential {
                account: "github:github.com:1".into(),
                operation: "get".into(),
            }
        );
        assert_eq!(parsed(&["--git-credential", "github:github.com:1"]), Invocation::Misuse);
        assert_eq!(
            parsed(&["--git-credential", "a", "get", "extra"]),
            Invocation::Misuse
        );
        assert_eq!(parsed(&["credential", "--git-credential"]), Invocation::Misuse);
    }
}
