use std::fmt::{self, Write as _};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::backend::llvm::{self, EmitError};
use crate::diagnostic::Diagnostic;
use crate::lexer::{self, LexError};
use crate::parser::{self, ParseError};
use crate::token::{Span, Token};
use crate::toolchain::{self, CodeFormat, ToolchainError};
use crate::typecheck::{self, TypedProgram};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Emit {
    Tokens,
    Ast,
    LlvmIr,
    Assembly,
    Object,
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

    fn artifact(self) -> Emit {
        match self {
            Emit::Assembly => Emit::LlvmIr,
            other => other,
        }
    }
}

pub struct BuildArguments {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub emit: Vec<Emit>,
    pub save_temps: bool,
}

#[derive(Debug)]
pub enum BuildError {
    Read {
        path: PathBuf,
        error: io::Error,
    },
    Write {
        path: PathBuf,
        error: io::Error,
    },
    AmbiguousOutput,
    Lex {
        path: PathBuf,
        errors: Vec<LexError>,
    },
    Parse {
        path: PathBuf,
        error: ParseError,
    },
    Check {
        path: PathBuf,
        diagnostics: Vec<Diagnostic>,
    },
    Emit {
        path: PathBuf,
        error: EmitError,
    },
    Toolchain(ToolchainError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Read { path, error } => {
                write!(formatter, "error: cannot read {}: {error}", path.display())
            }
            BuildError::Write { path, error } => {
                write!(formatter, "error: cannot write {}: {error}", path.display())
            }
            BuildError::AmbiguousOutput => {
                write!(formatter, "error: --output requires a single --emit kind")
            }
            BuildError::Lex { path, errors } => {
                let lines = errors.iter().map(|error| (error.span, error.message()));
                positioned(formatter, path, lines)
            }
            BuildError::Parse { path, error } => {
                positioned(formatter, path, [(error.span, error.message.clone())])
            }
            BuildError::Check { path, diagnostics } => {
                let lines = diagnostics
                    .iter()
                    .map(|diagnostic| (diagnostic.span, diagnostic.message.clone()));
                positioned(formatter, path, lines)
            }
            BuildError::Emit { path, error } => {
                positioned(formatter, path, [(error.span(), error.message())])
            }
            BuildError::Toolchain(error) => write!(formatter, "error: {error}"),
        }
    }
}

fn positioned(
    formatter: &mut fmt::Formatter<'_>,
    path: &Path,
    lines: impl IntoIterator<Item = (Span, String)>,
) -> fmt::Result {
    for (index, (span, message)) in lines.into_iter().enumerate() {
        if index > 0 {
            writeln!(formatter)?;
        }
        write!(
            formatter,
            "{}:{}:{}: error: {message}",
            path.display(),
            span.line,
            span.column
        )?;
    }
    Ok(())
}

impl From<ToolchainError> for BuildError {
    fn from(error: ToolchainError) -> Self {
        BuildError::Toolchain(error)
    }
}

pub fn build(arguments: &BuildArguments) -> Result<(), BuildError> {
    let mut context = BuildContext::new(arguments)?;
    let last = context.furthest().artifact();
    let source = fs::read_to_string(&arguments.input).map_err(|error| BuildError::Read {
        path: arguments.input.clone(),
        error,
    })?;
    let mut artifact = Artifact::Source(source);
    while artifact.kind() != Some(last) {
        artifact = artifact.advance(&context)?;
        context.emit(&artifact)?;
    }
    Ok(())
}

enum Artifact {
    Source(String),
    Tokens(Vec<Token>),
    Program(Program),
    Typed(TypedProgram),
    LlvmIr(PathBuf),
    Object(PathBuf),
    Executable,
}

impl Artifact {
    fn kind(&self) -> Option<Emit> {
        match self {
            Artifact::Source(_) => None,
            Artifact::Tokens(_) => Some(Emit::Tokens),
            Artifact::Program(_) => Some(Emit::Ast),
            Artifact::Typed(_) => None,
            Artifact::LlvmIr(_) => Some(Emit::LlvmIr),
            Artifact::Object(_) => Some(Emit::Object),
            Artifact::Executable => Some(Emit::Executable),
        }
    }

    fn advance(self, context: &BuildContext) -> Result<Artifact, BuildError> {
        let input = || context.input.clone();
        Ok(match self {
            Artifact::Source(source) => {
                let (tokens, errors) = lexer::tokenize(&source);
                if !errors.is_empty() {
                    return Err(BuildError::Lex {
                        path: input(),
                        errors,
                    });
                }
                Artifact::Tokens(tokens)
            }
            Artifact::Tokens(tokens) => {
                let program = parser::parse_program(tokens).map_err(|error| BuildError::Parse {
                    path: input(),
                    error,
                })?;
                Artifact::Program(program)
            }
            Artifact::Program(program) => {
                let typed = typecheck::check(program).map_err(|diagnostics| BuildError::Check {
                    path: input(),
                    diagnostics,
                })?;
                Artifact::Typed(typed)
            }
            Artifact::Typed(typed) => {
                let path = context.path(Emit::LlvmIr);
                let ir = llvm::emit(&typed.program).map_err(|error| BuildError::Emit {
                    path: input(),
                    error,
                })?;
                write(&path, &ir)?;
                Artifact::LlvmIr(path)
            }
            Artifact::LlvmIr(ir) => {
                let path = context.path(Emit::Object);
                toolchain::compile(&ir, &path, CodeFormat::Object)?;
                Artifact::Object(path)
            }
            Artifact::Object(object) => {
                let path = context.path(Emit::Executable);
                toolchain::link(&object, &path)?;
                Artifact::Executable
            }
            Artifact::Executable => unreachable!("an executable is the last artifact"),
        })
    }
}

struct BuildContext {
    input: PathBuf,
    requested: Vec<Emit>,
    output: Option<PathBuf>,
    base: PathBuf,
    temporaries: Vec<PathBuf>,
    save_temps: bool,
}

impl BuildContext {
    fn new(arguments: &BuildArguments) -> Result<Self, BuildError> {
        let requested = if arguments.emit.is_empty() {
            vec![Emit::Executable]
        } else {
            arguments.emit.clone()
        };
        if arguments.output.is_some() && requested.len() > 1 {
            return Err(BuildError::AmbiguousOutput);
        }
        Ok(BuildContext {
            input: arguments.input.clone(),
            requested,
            output: arguments.output.clone(),
            base: arguments
                .output
                .clone()
                .unwrap_or_else(|| arguments.input.clone()),
            temporaries: Vec::new(),
            save_temps: arguments.save_temps,
        })
    }

    fn furthest(&self) -> Emit {
        *self
            .requested
            .iter()
            .max()
            .expect("at least one kind is requested")
    }

    fn wants(&self, emit: Emit) -> bool {
        self.requested.contains(&emit)
    }

    fn path(&self, emit: Emit) -> PathBuf {
        match &self.output {
            Some(output) if self.wants(emit) => output.clone(),
            _ => self.base.with_extension(emit.extension()),
        }
    }

    fn emit(&mut self, artifact: &Artifact) -> Result<(), BuildError> {
        match artifact {
            Artifact::Source(_) | Artifact::Typed(_) | Artifact::Executable => {}
            Artifact::Tokens(tokens) => {
                if self.wants(Emit::Tokens) {
                    write(&self.path(Emit::Tokens), &token_listing(tokens))?;
                }
            }
            Artifact::Program(program) => {
                if self.wants(Emit::Ast) {
                    write(&self.path(Emit::Ast), &program.to_string())?;
                }
            }
            Artifact::LlvmIr(ir) => {
                if self.wants(Emit::Assembly) {
                    toolchain::compile(ir, &self.path(Emit::Assembly), CodeFormat::Assembly)?;
                }
                if !self.wants(Emit::LlvmIr) {
                    self.temporaries.push(ir.clone());
                }
            }
            Artifact::Object(object) => {
                if !self.wants(Emit::Object) {
                    self.temporaries.push(object.clone());
                }
            }
        }
        Ok(())
    }
}

impl Drop for BuildContext {
    fn drop(&mut self) {
        if self.save_temps {
            return;
        }
        for path in &self.temporaries {
            let _ = fs::remove_file(path);
        }
    }
}

fn write(path: &Path, contents: &str) -> Result<(), BuildError> {
    fs::write(path, contents).map_err(|error| BuildError::Write {
        path: path.to_path_buf(),
        error,
    })
}

fn token_listing(tokens: &[Token]) -> String {
    let mut listing = String::new();
    for token in tokens {
        let _ = writeln!(
            listing,
            "{:>4}:{:<4} {}",
            token.span.line, token.span.column, token
        );
    }
    listing
}
