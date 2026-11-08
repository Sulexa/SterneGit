use std::{
    ffi::OsString,
    io,
    path::Path,
    process::{Command, Stdio},
};

const SILENT_SETUP: [&str; 5] = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-", "/NOCANCEL"];
const RELAUNCH_AFTER_SETUP: &str = "/relaunch=yes";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Then {
    Relaunch,
    Stay,
}

pub(crate) fn setup_arguments(then: Then) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = SILENT_SETUP.iter().map(OsString::from).collect();
    if then == Then::Relaunch {
        arguments.push(RELAUNCH_AFTER_SETUP.into());
    }
    arguments
}

pub(crate) fn run_setup(setup: &Path, then: Then) -> io::Result<()> {
    start(setup, &setup_arguments(then))
}

pub(crate) fn start(program: &Path, arguments: &[OsString]) -> io::Result<()> {
    let command = || {
        let mut command = Command::new(program);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(folder) = program.parent() {
            command.current_dir(folder);
        }
        command
    };
    let mut outliving_this_process = command();
    outlive_this_process(&mut outliving_this_process);
    match outliving_this_process.spawn() {
        Ok(_) => Ok(()),
        Err(_) => command().spawn().map(|_| ()),
    }
}

#[cfg(windows)]
fn outlive_this_process(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    command.creation_flags(CREATE_BREAKAWAY_FROM_JOB);
}

#[cfg(not(windows))]
fn outlive_this_process(_: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::{Then, setup_arguments};

    #[test]
    fn the_setup_runs_without_any_window_and_relaunches_only_when_asked() {
        let restart = setup_arguments(Then::Relaunch);
        let quit = setup_arguments(Then::Stay);
        assert!(restart.iter().any(|argument| argument == "/VERYSILENT"));
        assert!(restart.iter().any(|argument| argument == "/SUPPRESSMSGBOXES"));
        assert!(restart.iter().any(|argument| argument == "/relaunch=yes"));
        assert!(!quit.iter().any(|argument| argument == "/relaunch=yes"));
    }
}
