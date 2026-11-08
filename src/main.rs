#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accounts;
mod actions;
mod app;
mod assets;
mod cli;
mod connect;
mod git;
mod hosting;
mod media;
mod net;
mod repo;
mod settings;
#[cfg(test)]
mod test_server;
mod ui;
mod update;

use cli::Invocation;

fn main() {
    match cli::parse(std::env::args_os().skip(1)) {
        Invocation::Help => cli::print_help(),
        Invocation::Misuse => cli::exit_with_usage(),
        Invocation::Check(path) => cli::check(&path),
        Invocation::Open(path, opening) => app::run(path, opening),
        Invocation::GitCredential { account, operation } => {
            cli::answer_git_credential(&account, &operation)
        }
        Invocation::ConnectorTest { folder, cassette } => {
            cli::test_connector(&folder, cassette.as_deref())
        }
    }
}
