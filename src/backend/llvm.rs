use crate::ast::Program;

pub fn emit(program: &Program) -> String {
    let mut module = String::new();
    for declaration in &program.declarations {
        match declaration.kind {}
    }
    module.push_str("define i32 @main() {\nentry:\n  ret i32 0\n}\n");
    module
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
