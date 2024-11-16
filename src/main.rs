extern crate llvm_sys as llvm;

mod codegen;
mod lexer;
mod parser;
mod syntax;
mod token;

use codegen::LLVMCodeGenVisitor;
use lexer::Lexer;
use parser::Parser;

fn main() {
    // let input = r#"
    //     // Compute the x'th fibonacci number.
    //     fun fib(x) {
    //         if x < 3 {
    //             1
    //         } else {
    //             fib(x - 1) + fib(x - 2)
    //         }
    //     }

    //     // This expression will compute the 40th number.
    //     fib(40)
    // "#;
    let code = r#"
        // Compute the sum of two floating point numbers.
        fun add(x: f64, y: f64) -> f64 {
            x + y
        }

        fun sub(x: f64, y: f64) -> f64 {
            x - y
        }

        10 + 20
    "#;

    let mut input = code.chars().peekable();

    let lexer = Lexer::new();
    let parser = Parser::new(lexer);

    let file = parser.parse(&mut input);
    println!("{:?}", file);

    println!("compiling: \n{:}", code);

    let mut codegen = LLVMCodeGenVisitor::new();
    codegen.visit_source_file(&file);

    println!("");

    codegen.emit_ir();
    codegen.emit_asm();
    codegen.finish();
}
