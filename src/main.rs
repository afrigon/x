//! The `x` bootstrap compiler (written in Rust).
//!
//! Currently just the lexer, exposed through an `x lex <file>` command that
//! dumps the token stream — useful for eyeballing the lexer on real `.x`
//! source while the rest of the frontend is built out.

mod ast;
mod lexer;
mod parser;
mod token;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "x", version, about = "The x language bootstrap compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lex a source file and print the resulting token stream.
    Lex {
        /// Path to the `.x` source file.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Lex { file } => lex_command(&file),
    }
}

fn lex_command(file: &PathBuf) -> ExitCode {
    let source = match std::fs::read_to_string(file) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", file.display());
            return ExitCode::FAILURE;
        }
    };

    let (tokens, errors) = lexer::tokenize(&source);

    for token in &tokens {
        println!("{:>4}:{:<4} {}", token.span.line, token.span.column, token);
    }

    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("\n{} lex error(s):", errors.len());
        for error in &errors {
            eprintln!("  {error}");
        }
        ExitCode::FAILURE
    }
}
