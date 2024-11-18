extern crate llvm_sys as llvm;

mod analysis;
mod codegen;
mod context;
mod lexer;
mod parser;
mod syntax;
mod token;
mod type_check;

use analysis::AnalysisVisitor;
use codegen::LLVMCodeGenVisitor;
use context::Context;
use lexer::Lexer;
use parser::Parser;
use type_check::TypeCheckVisitor;

use clap::{Parser as ClapParser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(ClapParser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a source file
    Compile { files: Vec<PathBuf> },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Compile { files }) => {
            for source_file in files {
                let code = fs::read_to_string(source_file.clone()).unwrap();
                let mut input = code.chars().peekable();

                let lexer = Lexer::new();
                let parser = Parser::new(lexer);

                let file = parser.parse(&mut input);
                // println!("{:?}", file);

                println!("compiling: \n{:}", code);

                let mut context = Context::new();

                let analysis = AnalysisVisitor {};
                analysis.visit_source_file(&file, &mut context);

                let type_check = TypeCheckVisitor::new();
                type_check.visit_source_file(&file, &mut context);

                let mut codegen = LLVMCodeGenVisitor::new();
                codegen.visit_source_file(&file); // TODO: maybe pass and use context here

                let output_file = source_file.with_extension("o");

                codegen.emit_ir();
                codegen.emit_asm(output_file);
                codegen.finish();
            }
        }
        None => {
            println!("no command given");
        }
    }
}
