use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context as _, Result, anyhow, bail};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    Success,
    SuccessOrDifference,
}

pub(crate) fn run(location: &Path, args: &[&str], exit: Exit) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    command.arg("--no-pager").arg("-C").arg(location).args(args);
    never_prompt(&mut command);
    without_console_window(&mut command);
    let output = command
        .output()
        .context("Couldn't run git. Is it installed and on PATH?")?;
    let found_difference = exit == Exit::SuccessOrDifference && output.status.code() == Some(1);
    if !output.status.success() && !found_difference {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(output.stdout)
}

pub(crate) fn run_utf8(location: &Path, args: &[&str]) -> Result<String> {
    let output = run(location, args, Exit::Success)?;
    String::from_utf8(output).map_err(|_| anyhow!("Git's output isn't UTF-8."))
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
