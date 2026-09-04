use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

pub struct RunArguments {
    pub executable: PathBuf,
}

#[derive(Debug)]
pub enum RunError {
    Execute { path: PathBuf, error: io::Error },
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Execute { path, error } => {
                write!(formatter, "cannot run {}: {error}", path.display())
            }
        }
    }
}

pub fn run(arguments: &RunArguments) -> Result<ExitStatus, RunError> {
    Command::new(&arguments.executable)
        .status()
        .map_err(|error| RunError::Execute {
            path: arguments.executable.clone(),
            error,
        })
}
