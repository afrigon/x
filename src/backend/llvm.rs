use std::fmt;

use crate::ast::{Declaration, DeclarationKind, Program};
use crate::token::Span;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum EmitError {
    Unsupported { what: &'static str, span: Span },
}

impl fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::Unsupported { what, span } => write!(
                formatter,
                "line {}, column {}: cannot lower {what}",
                span.line, span.column
            ),
        }
    }
}

pub fn emit(program: &Program) -> Result<String, EmitError> {
    let mut module = String::new();
    for declaration in &program.declarations {
        module.push_str(&emit_declaration(declaration)?);
    }
    module.push_str("define i32 @main() {\nentry:\n  ret i32 0\n}\n");
    Ok(module)
}

fn emit_declaration(declaration: &Declaration) -> Result<String, EmitError> {
    match &declaration.kind {
        DeclarationKind::Function(_) => Err(EmitError::Unsupported {
            what: "function declarations",
            span: declaration.span,
        }),
    }
}
