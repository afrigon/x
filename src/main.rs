mod ast;
mod backend;
mod cli;
mod commands;
mod diagnostic;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
mod token;
mod toolchain;
mod typecheck;
mod types;

use std::process::ExitCode;

use usage::Run;

fn main() -> ExitCode {
    cli::Cli::parse().run()
}
