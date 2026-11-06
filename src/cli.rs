use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::Value;

use crate::{
    accounts,
    connect::{self, CallError, Played, Trial, Usage},
    git::{GIT_CREDENTIAL_FLAG, Repo},
    ui::{counted, thousands},
};

const CONNECTOR_TEST_FLAG: &str = "--connector-test";
const USAGE: &str = "Usage: sternegit [--check] [repository]\n       \
                     sternegit --connector-test <folder> [cassette]";
const HELP: &str = "Brings back the tabs open last time, plus the repository containing the given path.\n\
                    --check reads the history without opening a window and reports how long it took.\n\
                    --connector-test starts the connector in the folder in its sandbox, without a window,\n\
                    replays the cassette's call against it if one is given, and prints the answer, the log,\n\
                    the fuel and the memory used.";
const MEBIBYTE: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Invocation {
    Help,
    Misuse,
    Check(PathBuf),
    Open(Option<PathBuf>),
    GitCredential { account: String, operation: String },
    ConnectorTest { folder: PathBuf, cassette: Option<PathBuf> },
}

pub(crate) fn parse(arguments: impl IntoIterator<Item = OsString>) -> Invocation {
    let mut arguments = arguments.into_iter().peekable();
    if arguments
        .next_if(|first| first == GIT_CREDENTIAL_FLAG)
        .is_some()
    {
        return git_credential(arguments.collect());
    }
    if arguments
        .next_if(|first| first == CONNECTOR_TEST_FLAG)
        .is_some()
    {
        return connector_test(arguments.collect());
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

fn connector_test(rest: Vec<OsString>) -> Invocation {
    let mut rest = rest.into_iter().map(PathBuf::from);
    match (rest.next(), rest.next(), rest.next()) {
        (Some(folder), cassette, None) => Invocation::ConnectorTest { folder, cassette },
        _ => Invocation::Misuse,
    }
}

pub(crate) fn test_connector(folder: &Path, cassette: Option<&Path>) {
    match connect::try_connector(folder, cassette) {
        Ok(trial) => {
            print!("{}", trial_report(&trial));
            if !trial.passed() {
                std::process::exit(1);
            }
        }
        Err(problem) => {
            eprintln!("{problem}");
            std::process::exit(1);
        }
    }
}

fn trial_report(trial: &Trial) -> String {
    let mut lines = vec![format!(
        "{} {}: {}, {} bytes",
        trial.id.0,
        trial.version,
        trial.module.display(),
        thousands(trial.module_bytes)
    )];
    match &trial.start {
        Ok(greeting) => {
            let sdk = greeting
                .hello
                .sdk
                .as_deref()
                .map(|sdk| format!(" through SDK {sdk}"))
                .unwrap_or_default();
            lines.push(format!(
                "  started in {}, speaking protocol {}{sdk}",
                milliseconds(trial.start_time),
                greeting.hello.protocol
            ));
            lines.push(format!("  hello: {}", usage(&greeting.usage)));
        }
        Err(stop) => lines.push(format!("  didn't start: {stop}")),
    }
    lines.extend(indented_log(&trial.start_log));
    if let Some(played) = &trial.played {
        lines.extend(played_report(played));
    }
    lines.push(if trial.passed() { "passed" } else { "failed" }.to_owned());
    lines.join("\n") + "\n"
}

fn played_report(played: &Played) -> Vec<String> {
    let mut lines = vec![
        format!("{} from {}", played.op, played.cassette.display()),
        format!("  {}", usage(&played.outcome.usage)),
    ];
    lines.extend(indented_log(&played.log));
    if let Ok(output) = &played.outcome.result {
        let shown = serde_json::from_slice::<Value>(output)
            .ok()
            .and_then(|value| serde_json::to_string_pretty(&value).ok())
            .unwrap_or_else(|| String::from_utf8_lossy(output).into_owned());
        lines.push("  output:".to_owned());
        lines.extend(shown.lines().map(|line| format!("    {line}")));
    }
    match &played.answer {
        Ok(()) => {}
        Err(CallError::Failed(failure)) => lines.push(format!("  answered with a failure: {failure}")),
        Err(error) => lines.push(format!("  {error}")),
    }
    lines.extend(
        played
            .unplayed
            .iter()
            .map(|exchange| format!("  never asked for {} {}", exchange.method, exchange.url)),
    );
    match played.matches_expected() {
        Some(true) => lines.push("  matches the cassette's output".to_owned()),
        Some(false) => lines.push("  differs from the cassette's output".to_owned()),
        None => {}
    }
    lines
}

fn indented_log(log: &[String]) -> Vec<String> {
    log.iter()
        .map(|line| format!("  log: {}", line.replace('\n', "\n       ")))
        .collect()
}

fn usage(usage: &Usage) -> String {
    format!(
        "{} fuel in {}, {} of memory, {}, {}",
        thousands(usize::try_from(usage.fuel).unwrap_or(usize::MAX)),
        counted(usage.slices as usize, "slice", "slices"),
        size(usage.memory),
        counted(usage.requests, "request", "requests"),
        milliseconds(usage.elapsed)
    )
}

fn size(bytes: usize) -> String {
    if bytes < MEBIBYTE {
        format!("{} KiB", bytes / 1024)
    } else {
        format!("{:.1} MiB", bytes as f64 / MEBIBYTE as f64)
    }
}

fn milliseconds(duration: Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
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
    use std::{ffi::OsString, path::PathBuf, time::Duration};

    use super::{Invocation, indented_log, milliseconds, parse, size};

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

    #[test]
    fn a_connector_test_takes_a_folder_and_maybe_a_cassette() {
        assert_eq!(
            parsed(&["--connector-test", "connectors/github/dist"]),
            Invocation::ConnectorTest {
                folder: PathBuf::from("connectors/github/dist"),
                cassette: None,
            }
        );
        assert_eq!(
            parsed(&["--connector-test", "dist", "tests/list.json"]),
            Invocation::ConnectorTest {
                folder: PathBuf::from("dist"),
                cassette: Some(PathBuf::from("tests/list.json")),
            }
        );
        assert_eq!(parsed(&["--connector-test"]), Invocation::Misuse);
        assert_eq!(parsed(&["--connector-test", "a", "b", "c"]), Invocation::Misuse);
        assert_eq!(parsed(&["repo", "--connector-test"]), Invocation::Misuse);
    }

    #[test]
    fn a_log_line_spanning_lines_stays_under_its_label() {
        assert_eq!(
            indented_log(&["Error panicked at src\\lib.rs:54:9:\nnot written yet".to_owned()]),
            ["  log: Error panicked at src\\lib.rs:54:9:\n       not written yet"]
        );
    }

    #[test]
    fn sizes_and_times_read_at_a_glance() {
        assert_eq!(size(163_840), "160 KiB");
        assert_eq!(size(2_516_582), "2.4 MiB");
        assert_eq!(milliseconds(Duration::from_micros(4_125)), "4.1 ms");
    }
}
