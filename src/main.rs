#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg_attr(not(test), expect(dead_code))]
mod accounts;
mod actions;
mod app;
mod assets;
mod cli;
mod git;
mod media;
#[cfg_attr(not(test), expect(dead_code))]
mod net;
mod repo;
mod settings;
#[cfg(test)]
mod test_server;
mod ui;

use cli::Invocation;

fn main() {
    match cli::parse(std::env::args_os().skip(1)) {
        Invocation::Help => cli::print_help(),
        Invocation::Misuse => cli::exit_with_usage(),
        Invocation::Check(path) => cli::check(&path),
        Invocation::Open(path) => app::run(path),
        Invocation::GitCredential { account, operation } => {
            cli::answer_git_credential(&account, &operation)
        }
    }
}
