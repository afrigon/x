use std::fmt;
use std::io;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CodeFormat {
    Assembly,
    Object,
}

#[derive(Debug)]
pub enum ToolchainError {
    NotFound {
        tool: &'static str,
    },
    Failed {
        tool: &'static str,
        stderr: String,
    },
    Spawn {
        tool: &'static str,
        error: io::Error,
    },
}

impl fmt::Display for ToolchainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolchainError::NotFound { tool } => write!(formatter, "{tool} not found in PATH"),
            ToolchainError::Failed { tool, stderr } => {
                write!(formatter, "{tool} failed:\n{}", stderr.trim_end())
            }
            ToolchainError::Spawn { tool, error } => {
                write!(formatter, "cannot run {tool}: {error}")
            }
        }
    }
}

pub fn compile(ir: &Path, output: &Path, format: CodeFormat) -> Result<(), ToolchainError> {
    let filetype = match format {
        CodeFormat::Assembly => "asm",
        CodeFormat::Object => "obj",
    };
    let mut command = Command::new("llc");
    command
        .arg("-relocation-model=pic")
        .arg(format!("-filetype={filetype}"))
        .arg(ir)
        .arg("-o")
        .arg(output);
    invoke("llc", command)
}

pub fn link(object: &Path, executable: &Path) -> Result<(), ToolchainError> {
    let mut command = Command::new("clang");
    command.arg(object).arg("-o").arg(executable);
    invoke("clang", command)
}

fn invoke(tool: &'static str, mut command: Command) -> Result<(), ToolchainError> {
    let output = command.output().map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ToolchainError::NotFound { tool },
        _ => ToolchainError::Spawn { tool, error },
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(ToolchainError::Failed {
            tool,
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
