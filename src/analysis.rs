use crate::context::*;
use crate::syntax::*;

pub struct AnalysisVisitor {}

impl AnalysisVisitor {
    pub fn visit_source_file(&self, node: &SourceFile, context: &mut Context) {
        self.visit_code_block(&node.code_block, context);
    }

    pub fn visit_code_block(&self, node: &CodeBlock, context: &mut Context) {
        for item in node.items {
            self.visit_code_block_item(&item, context);
        }
    }

    pub fn visit_code_block_item(&self, node: &CodeBlockItem, context: &mut Context) {
        match node {
            CodeBlockItem::Declaration(declaration) => {
                self.visit_declaration(&declaration, context)
            }
            _ => return,
        }
    }

    pub fn visit_declaration(&self, node: &Declaration, context: &mut Context) {
        match node {
            Declaration::VariableDeclaration(variable) => {
                self.visit_variable_declaration(&variable, context)
            }
            Declaration::FunctionDeclaration(function) => {
                self.visit_function_declaration(&function, context)
            }
            Declaration::ExternDeclaration(ext) => self.visit_extern_declaration(&ext, context),
        }
    }

    pub fn visit_variable_declaration(&self, node: &VariableDeclaration, context: &mut Context) {
        let symbol = Symbol {
            name: node.identifier.name.clone(),
        };

        context.register_symbol(symbol);
    }

    pub fn visit_function_declaration(&self, node: &FunctionDeclaration, context: &mut Context) {
        let symbol = Symbol {
            name: node.identifier.name.clone(),
        };

        context.register_symbol(symbol);
    }

    pub fn visit_extern_declaration(&self, node: &ExternDeclaration, context: &mut Context) {
        let symbol = Symbol {
            name: node.identifier.name.clone(),
        };

        context.register_symbol(symbol);
    }
}
