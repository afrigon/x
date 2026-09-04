use super::*;

#[test]
fn empty_program_defines_main_returning_zero() {
    let program = Program {
        declarations: Vec::new(),
    };
    assert_eq!(
        emit(&program).unwrap(),
        "define i32 @main() {\nentry:\n  ret i32 0\n}\n"
    );
}
