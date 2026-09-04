mod ast;
mod lexer;
mod parser;
mod token;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::process::ExitCode;

use usage::{Cli, Subcommands};

#[derive(Cli)]
#[usage(
    bin = "x",
    version,
    about = "The x language bootstrap compiler",
    unknown_flags = "error"
)]
struct Cli {
    #[usage(subcommand)]
    command: Command,
}

#[derive(Subcommands)]
enum Command {
    #[usage(help = "Lex a source file and print the resulting token stream")]
    Lex {
        #[usage(help = "Path to the .x source file")]
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
