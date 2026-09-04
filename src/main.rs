mod ast;
mod backend;
mod driver;
mod lexer;
mod parser;
#[cfg(test)]
mod tests;
mod token;
mod toolchain;

use std::path::PathBuf;
use std::process::ExitCode;

use usage::{Args, Cli, Subcommands};

use driver::{BuildOptions, Emit};

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
    #[usage(help = "Compile a source file")]
    Build(Build),
    #[usage(help = "Compile a source file and run the result")]
    Run(Run),
}

#[derive(Args)]
struct Build {
    #[usage(help = "Path to the .x source file")]
    file: PathBuf,
    #[usage(
        short = 'o',
        long,
        help = "Where to write the output; requires a single --emit kind"
    )]
    output: Option<PathBuf>,
    #[usage(
        long,
        var,
        value_enum,
        help = "What to produce, repeatable; defaults to executable"
    )]
    emit: Vec<Emit>,
    #[usage(long, help = "Keep the intermediate .ll and .o files")]
    save_temps: bool,
}

#[derive(Args)]
struct Run {
    #[usage(help = "Path to the .x source file")]
    file: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Build(build) => build_command(build),
        Command::Run(run) => run_command(run),
    }
}

fn build_command(build: Build) -> ExitCode {
    let options = BuildOptions {
        input: build.file,
        output: build.output,
        emit: build.emit,
        save_temps: build.save_temps,
    };
    match driver::build(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(error),
    }
}

fn run_command(run: Run) -> ExitCode {
    match driver::run(&run.file) {
        Ok(status) => match status.code() {
            Some(code) => ExitCode::from(code as u8),
            None => {
                eprintln!("error: program terminated by a signal");
                ExitCode::FAILURE
            }
        },
        Err(error) => report(error),
    }
}

fn report(error: driver::DriverError) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}
