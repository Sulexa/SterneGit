use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::git::{self, Repo};

const USAGE: &str = "Usage: fastgit [--check] [repository]";
const HELP: &str = "Opens the repository containing the given path, or the current directory.\n\
                    --check reads the history without opening a window and reports how long it took.";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Invocation {
    Help,
    Misuse,
    Check(PathBuf),
    Open(Option<PathBuf>),
}

pub(crate) fn parse(arguments: impl IntoIterator<Item = OsString>) -> Invocation {
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
            history.commits.len(),
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

pub(crate) fn current_repository() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .filter(|dir| git::is_inside_repository(dir))
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{Invocation, parse};

    fn parsed(arguments: &[&str]) -> Invocation {
        parse(arguments.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_open_the_current_directory_when_it_is_a_repository() {
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
}
