use std::path::PathBuf;
use std::process::ExitCode;

use usage::{Args, Run, ValueEnum};

use crate::commands::build::{self, BuildArguments};

#[derive(Args)]
pub struct BuildCommand {
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

#[derive(Clone, Copy, ValueEnum)]
enum Emit {
    #[usage(help = "The token stream")]
    Tokens,
    #[usage(help = "The abstract syntax tree")]
    Ast,
    #[usage(help = "Textual LLVM IR")]
    LlvmIr,
    #[usage(help = "Native assembly")]
    Assembly,
    #[usage(help = "A native object file")]
    Object,
    #[usage(help = "A linked executable")]
    Executable,
}

impl From<Emit> for build::Emit {
    fn from(emit: Emit) -> Self {
        match emit {
            Emit::Tokens => build::Emit::Tokens,
            Emit::Ast => build::Emit::Ast,
            Emit::LlvmIr => build::Emit::LlvmIr,
            Emit::Assembly => build::Emit::Assembly,
            Emit::Object => build::Emit::Object,
            Emit::Executable => build::Emit::Executable,
        }
    }
}

impl Run for BuildCommand {
    type Output = ExitCode;

    fn run(self) -> ExitCode {
        let arguments = BuildArguments {
            input: self.file,
            output: self.output,
            emit: self.emit.into_iter().map(Into::into).collect(),
            save_temps: self.save_temps,
        };
        match build::build(&arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => super::report(error),
        }
    }
}
