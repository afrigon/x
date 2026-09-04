use crate::ast::{Declaration, Program};

#[cfg(test)]
mod tests;

pub fn emit(program: &Program) -> String {
    let mut module: String = program.declarations.iter().map(emit_declaration).collect();
    module.push_str("define i32 @main() {\nentry:\n  ret i32 0\n}\n");
    module
}

fn emit_declaration(declaration: &Declaration) -> String {
    match declaration.kind {}
}
