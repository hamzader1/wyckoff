//! wyckoff — commit messages written from your staged diff, in your own voice.

mod app;
mod branch;
mod cache;
mod cli;
mod commands;
mod config;
mod context;
mod diff;
mod error;
mod generate;
mod git;
mod hook;
mod init;
mod llm;
mod msg;
mod output;
mod paths;
mod prompt;
mod secrets;
mod style;
mod symbols;
mod tokens;
mod ui;
mod util;
mod validate;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    let code = match app::dispatch(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("wyckoff: {error}");
            1
        }
    };
    std::process::exit(code);
}
