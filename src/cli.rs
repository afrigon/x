mod build;
mod run;

use std::fmt::Display;
use std::process::ExitCode;

use usage::{Cli, Subcommands};

use build::BuildCommand;
use run::RunCommand;

#[derive(Cli)]
#[usage(
    bin = "x",
    version,
    about = "The x language bootstrap compiler",
    unknown_flags = "error",
    run
)]
pub struct Cli {
    #[usage(subcommand)]
    command: Command,
}

#[derive(Subcommands)]
#[usage(run)]
pub enum Command {
    #[usage(help = "Compile a source file")]
    Build(BuildCommand),
    #[usage(help = "Compile a source file and run the result")]
    Run(RunCommand),
}

fn report(error: impl Display) -> ExitCode {
    eprintln!("error: {error}");
    ExitCode::FAILURE
}
