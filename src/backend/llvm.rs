use crate::ast::{Declaration, Program};

pub fn emit(program: &Program) -> String {
    let mut module: String = program.declarations.iter().map(emit_declaration).collect();
    module.push_str("define i32 @main() {\nentry:\n  ret i32 0\n}\n");
    module
}

fn emit_declaration(declaration: &Declaration) -> String {
    match declaration.kind {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_program_defines_main_returning_zero() {
        let program = Program {
            declarations: Vec::new(),
        };
        assert_eq!(
            emit(&program),
            "define i32 @main() {\nentry:\n  ret i32 0\n}\n"
        );
    }
}
