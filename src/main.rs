mod ast;
mod backend;
mod cli;
mod commands;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
mod token;
mod toolchain;

use std::process::ExitCode;

use usage::Run;

fn main() -> ExitCode {
    cli::Cli::parse().run()
}
