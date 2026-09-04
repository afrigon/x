use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::ExitCode;

use usage::{Args, Run};

use crate::commands::build::{self, BuildArguments, Emit};
use crate::commands::run::{self, RunArguments};

#[derive(Args)]
pub struct RunCommand {
    #[usage(help = "Path to the .x source file")]
    file: PathBuf,
}

impl Run for RunCommand {
    type Output = ExitCode;

    fn run(self) -> ExitCode {
        let directory = match tempfile::tempdir() {
            Ok(directory) => directory,
            Err(error) => {
                return super::report(format_args!("cannot create a temporary directory: {error}"));
            }
        };
        let name = self.file.with_extension("");
        let executable = directory
            .path()
            .join(name.file_name().unwrap_or(OsStr::new("program")));

        let build = BuildArguments {
            input: self.file,
            output: Some(executable.clone()),
            emit: vec![Emit::Executable],
            save_temps: false,
        };
        if let Err(error) = build::build(&build) {
            return super::report(error);
        }

        match run::run(&RunArguments { executable }) {
            Ok(status) => match status.code() {
                Some(code) => ExitCode::from(code as u8),
                None => super::report("program terminated by a signal"),
            },
            Err(error) => super::report(error),
        }
    }
}
