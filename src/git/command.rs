use std::{
    ffi::OsStr,
    io::Write as _,
    path::Path,
    process::{Command, Output, Stdio},
};

use anyhow::{Context as _, Result, bail};

use super::text::decode;

pub(crate) const READ_ONLY: &str = "--no-optional-locks";
const GIT_MISSING: &str = "Couldn't run git. Is it installed and on PATH?";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    Success,
    SuccessOrDifference,
}

pub(crate) fn run<A: AsRef<OsStr>>(location: &Path, args: &[A], exit: Exit) -> Result<Vec<u8>> {
    let output = git(location, args).output().context(GIT_MISSING)?;
    checked(output, exit)
}

pub(crate) fn run_text<A: AsRef<OsStr>>(location: &Path, args: &[A], exit: Exit) -> Result<String> {
    Ok(decode(&run(location, args, exit)?))
}

pub(crate) fn run_with_input<A: AsRef<OsStr>>(
    location: &Path,
    args: &[A],
    input: &[u8],
) -> Result<Vec<u8>> {
    let mut child = git(location, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(GIT_MISSING)?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input)?;
    }
    checked(child.wait_with_output()?, Exit::Success)
}

fn git<A: AsRef<OsStr>>(location: &Path, args: &[A]) -> Command {
    let mut command = Command::new("git");
    command.arg("--no-pager").arg("-C").arg(location).args(args);
    never_prompt(&mut command);
    without_console_window(&mut command);
    command
}

fn checked(output: Output, exit: Exit) -> Result<Vec<u8>> {
    let found_difference = exit == Exit::SuccessOrDifference && output.status.code() == Some(1);
    if !output.status.success() && !found_difference {
        bail!("{}", decode(&output.stderr).trim());
    }
    Ok(output.stdout)
}

fn never_prompt(command: &mut Command) {
    command.env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null());
}

#[cfg(windows)]
fn without_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn without_console_window(_: &mut Command) {}
