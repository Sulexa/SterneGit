#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod assets;
mod cli;
mod git;
mod media;
mod repo;
mod ui;

use cli::Invocation;

fn main() {
    match cli::parse(std::env::args_os().skip(1)) {
        Invocation::Help => cli::print_help(),
        Invocation::Misuse => cli::exit_with_usage(),
        Invocation::Check(path) => cli::check(&path),
        Invocation::Open(path) => app::run(path),
    }
}
