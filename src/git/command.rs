use std::{
    ffi::OsStr,
    io::{Read as _, Write as _},
    path::Path,
    process::{Command, Output, Stdio},
};

use anyhow::{Context as _, Result, bail};

use super::{access::Access, text::decode};

pub(crate) const READ_ONLY: &str = "--no-optional-locks";
pub(crate) const UNIQUE_LINES_FIRST: &str = "--histogram";
const NO_EDITOR: &str = ":";
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

pub(crate) fn run_with_access<A: AsRef<OsStr>>(
    location: &Path,
    access: &Access,
    args: &[A],
    exit: Exit,
) -> Result<Vec<u8>> {
    access.renew_tokens();
    let mut all = access.config_args();
    all.extend(args.iter().map(|arg| arg.as_ref().to_owned()));
    let mut command = git(location, &all);
    command.envs(access.environment());
    checked(command.output().context(GIT_MISSING)?, exit)
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

pub(crate) fn first_bytes<A: AsRef<OsStr>>(location: &Path, args: &[A], count: usize) -> Result<Vec<u8>> {
    let mut child = git(location, args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context(GIT_MISSING)?;
    let mut start = Vec::with_capacity(count);
    if let Some(stdout) = child.stdout.take() {
        stdout.take(count as u64).read_to_end(&mut start)?;
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(start)
}

fn git<A: AsRef<OsStr>>(location: &Path, args: &[A]) -> Command {
    let mut command = Command::new("git");
    command.arg("--no-pager").arg("-C").arg(location).args(args);
    never_prompt(&mut command);
    without_console_window(&mut command);
    ignoring_the_users_config(&mut command);
    command
}

fn checked(output: Output, exit: Exit) -> Result<Vec<u8>> {
    let found_difference = exit == Exit::SuccessOrDifference && output.status.code() == Some(1);
    if !output.status.success() && !found_difference {
        bail!("{}", what_git_said(&output));
    }
    Ok(output.stdout)
}

fn what_git_said(output: &Output) -> String {
    [&output.stderr, &output.stdout]
        .into_iter()
        .map(|said| decode(said).trim().to_owned())
        .find(|said| !said.is_empty())
        .unwrap_or_else(|| match output.status.code() {
            Some(code) => format!("git stopped with exit code {code} and said nothing"),
            None => "git was stopped before it finished".to_owned(),
        })
}

fn never_prompt(command: &mut Command) {
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", NO_EDITOR)
        .stdin(Stdio::null());
}

#[cfg(test)]
pub(crate) fn ignoring_the_users_config(command: &mut Command) {
    command.env("GIT_CONFIG_NOSYSTEM", "1").env(
        "GIT_CONFIG_GLOBAL",
        std::env::temp_dir().join("sternegit-tests-ignore-the-users-git-config"),
    );
}

#[cfg(not(test))]
pub(crate) fn ignoring_the_users_config(_: &mut Command) {}

#[cfg(windows)]
pub(crate) fn without_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(crate) fn without_console_window(_: &mut Command) {}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Exit, run};
    use crate::git::scratch::ScratchRepo;

    #[test]
    fn a_failure_with_nothing_on_stderr_keeps_what_git_printed_or_its_exit_code() -> Result<()> {
        let scratch = ScratchRepo::new("silent-failure")?;
        std::fs::write(scratch.dir.join("before"), "Lune
")?;
        std::fs::write(scratch.dir.join("after"), "Astro
")?;
        let printed = run(&scratch.dir, &["diff", "--no-index", "--exit-code", "before", "after"], Exit::Success)
            .expect_err("the files differ");
        assert!(printed.to_string().contains("+Astro"));
        let silent = run(&scratch.dir, &["config", "--get", "sternegit.missing"], Exit::Success)
            .expect_err("the key is missing");
        assert_eq!(silent.to_string(), "git stopped with exit code 1 and said nothing");
        Ok(())
    }
}
