use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use crate::backend::llvm;
use crate::lexer::{self, LexError};
use crate::parser::{self, ParseError};
use crate::token::Token;
use crate::toolchain::{self, CodeFormat, ToolchainError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, usage::ValueEnum)]
pub enum Emit {
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

impl Emit {
    fn extension(self) -> &'static str {
        match self {
            Emit::Tokens => "tokens",
            Emit::Ast => "ast",
            Emit::LlvmIr => "ll",
            Emit::Assembly => "s",
            Emit::Object => "o",
            Emit::Executable => "",
        }
    }
}

pub struct BuildOptions {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub emit: Vec<Emit>,
    pub save_temps: bool,
}

#[derive(Debug)]
pub enum DriverError {
    Read { path: PathBuf, error: io::Error },
    Write { path: PathBuf, error: io::Error },
    AmbiguousOutput,
    Lex(Vec<LexError>),
    Parse(ParseError),
    Toolchain(ToolchainError),
    Execute { path: PathBuf, error: io::Error },
}

impl fmt::Display for DriverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::Read { path, error } => {
                write!(formatter, "cannot read {}: {error}", path.display())
            }
            DriverError::Write { path, error } => {
                write!(formatter, "cannot write {}: {error}", path.display())
            }
            DriverError::AmbiguousOutput => {
                write!(formatter, "--output requires a single --emit kind")
            }
            DriverError::Lex(errors) => {
                for (index, error) in errors.iter().enumerate() {
                    if index > 0 {
                        writeln!(formatter)?;
                    }
                    write!(formatter, "{error}")?;
                }
                Ok(())
            }
            DriverError::Parse(error) => write!(formatter, "{error}"),
            DriverError::Toolchain(error) => write!(formatter, "{error}"),
            DriverError::Execute { path, error } => {
                write!(formatter, "cannot run {}: {error}", path.display())
            }
        }
    }
}

impl From<ToolchainError> for DriverError {
    fn from(error: ToolchainError) -> Self {
        DriverError::Toolchain(error)
    }
}

pub fn build(options: &BuildOptions) -> Result<(), DriverError> {
    let requested: Vec<Emit> = if options.emit.is_empty() {
        vec![Emit::Executable]
    } else {
        options.emit.clone()
    };
    if options.output.is_some() && requested.len() > 1 {
        return Err(DriverError::AmbiguousOutput);
    }
    let furthest = *requested.iter().max().unwrap_or(&Emit::Executable);
    let wants = |emit: Emit| requested.contains(&emit);
    let base = options.output.as_ref().unwrap_or(&options.input);
    let path = |emit: Emit| match &options.output {
        Some(output) if wants(emit) => output.clone(),
        _ => base.with_extension(emit.extension()),
    };
    let mut temporaries = Temporaries::new(options.save_temps);

    let source = fs::read_to_string(&options.input).map_err(|error| DriverError::Read {
        path: options.input.clone(),
        error,
    })?;

    let (tokens, errors) = lexer::tokenize(&source);
    if !errors.is_empty() {
        return Err(DriverError::Lex(errors));
    }
    if wants(Emit::Tokens) {
        write(&path(Emit::Tokens), &token_listing(&tokens))?;
    }
    if furthest == Emit::Tokens {
        return Ok(());
    }

    let program = parser::parse_program(tokens).map_err(DriverError::Parse)?;
    if wants(Emit::Ast) {
        write(&path(Emit::Ast), &program.to_string())?;
    }
    if furthest == Emit::Ast {
        return Ok(());
    }

    let ir_path = path(Emit::LlvmIr);
    write(&ir_path, &llvm::emit(&program))?;
    if !wants(Emit::LlvmIr) {
        temporaries.register(ir_path.clone());
    }
    if furthest == Emit::LlvmIr {
        return Ok(());
    }

    if wants(Emit::Assembly) {
        toolchain::compile(&ir_path, &path(Emit::Assembly), CodeFormat::Assembly)?;
    }
    if furthest == Emit::Assembly {
        return Ok(());
    }

    let object_path = path(Emit::Object);
    toolchain::compile(&ir_path, &object_path, CodeFormat::Object)?;
    if !wants(Emit::Object) {
        temporaries.register(object_path.clone());
    }
    if furthest == Emit::Object {
        return Ok(());
    }

    toolchain::link(&object_path, &path(Emit::Executable))?;
    Ok(())
}

pub fn run(input: &Path) -> Result<ExitStatus, DriverError> {
    let directory = tempfile::tempdir().map_err(|error| DriverError::Write {
        path: std::env::temp_dir(),
        error,
    })?;
    let name = input.with_extension("");
    let executable = directory
        .path()
        .join(name.file_name().unwrap_or(OsStr::new("program")));
    build(&BuildOptions {
        input: input.to_path_buf(),
        output: Some(executable.clone()),
        emit: vec![Emit::Executable],
        save_temps: false,
    })?;
    Command::new(&executable)
        .status()
        .map_err(|error| DriverError::Execute {
            path: executable,
            error,
        })
}

fn write(path: &Path, contents: &str) -> Result<(), DriverError> {
    fs::write(path, contents).map_err(|error| DriverError::Write {
        path: path.to_path_buf(),
        error,
    })
}

fn token_listing(tokens: &[Token]) -> String {
    let mut listing = String::new();
    for token in tokens {
        listing.push_str(&format!(
            "{:>4}:{:<4} {}\n",
            token.span.line, token.span.column, token
        ));
    }
    listing
}

struct Temporaries {
    paths: Vec<PathBuf>,
    keep: bool,
}

impl Temporaries {
    fn new(keep: bool) -> Self {
        Temporaries {
            paths: Vec::new(),
            keep,
        }
    }

    fn register(&mut self, path: PathBuf) {
        self.paths.push(path);
    }
}

impl Drop for Temporaries {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        for path in &self.paths {
            let _ = fs::remove_file(path);
        }
    }
}
