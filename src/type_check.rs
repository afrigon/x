use crate::context::*;
use crate::syntax::*;

pub struct TypeCheckVisitor {}

impl TypeCheckVisitor {
    pub fn new() -> TypeCheckVisitor {
        Self {}
    }

    pub fn visit_source_file(&self, node: &SourceFile, context: &mut Context) {
        self.visit_code_block(&node.code_block, context);
    }

    pub fn visit_code_block(&self, node: &CodeBlock, context: &mut Context) {
        for item in &node.items {
            self.visit_code_block_item(&item, context);
        }
    }

    pub fn visit_code_block_container(&self, node: &CodeBlockContainer, context: &mut Context) {
        self.visit_code_block(&node.code_block, context);
    }

    pub fn visit_code_block_item(&self, code_block_item: &CodeBlockItem, context: &mut Context) {
        match code_block_item {
            CodeBlockItem::Declaration(decl) => self.visit_declaration(&decl, context),
            CodeBlockItem::Expression(expr) => {
                self.visit_expression(&expr, context);
            }
            _ => return,
        };
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
            Declaration::EnumDeclaration(enumeration) => {
                self.visit_enum_declaration(&enumeration, context)
            }
            Declaration::TypeDeclaration(_type) => self.visit_type_declaration(&_type, context),
        }
    }

    pub fn visit_variable_declaration(&self, node: &VariableDeclaration, context: &mut Context) {}

    pub fn visit_function_declaration(&self, node: &FunctionDeclaration, context: &mut Context) {
        self.visit_code_block_container(&node.body, context);
    }

    pub fn visit_extern_declaration(&self, node: &ExternDeclaration, context: &mut Context) {}

    pub fn visit_enum_declaration(&self, node: &EnumDeclaration, context: &mut Context) {}

    pub fn visit_type_declaration(&self, node: &TypeDeclaration, context: &mut Context) {}

    pub fn visit_expression(&self, expression: &Expression, context: &mut Context) -> Type {
        match expression {
            Expression::BinaryOperator(binary_expr) => {
                self.visit_binary_operator_expression(&binary_expr, context)
            }
            Expression::Identifier(identifier) => {
                self.visit_identifier_expression(&identifier, context)
            }
            Expression::BooleanLiteral(_) => Type::Identifier("bool".to_string()),
            Expression::FloatNumberLiteral(_) => Type::Identifier("float".to_string()),
            Expression::FunctionCall(call) => self.visit_function_call_expression(&call, context),
            _ => return Type::Void,
        }
    }

    pub fn visit_identifier_expression(
        &self,
        identifier: &Identifier,
        context: &mut Context,
    ) -> Type {
        let Some(symbol) = context.lookup(identifier.name.clone()) else {
            println!("undeclared identifier {:?}", identifier.name);
            return Type::Void; // TODO: should we return unknown(String) instead?
        };

        return symbol.symbol_type;
    }

    pub fn visit_function_call_expression(
        &self,
        node: &FunctionCallExpression,
        context: &mut Context,
    ) -> Type {
        let Some(symbol) = context.lookup(node.function.name.clone()) else {
            println!("undeclared function {:?}", node.function.name);
            return Type::Void;
        };

        match symbol.symbol_type {
            Type::Function(function) => {
                // TODO: check arguments
                return function.return_type.as_ref().clone();
            }
            _ => {
                println!(
                    "trying to use {:?} as a function but it is a {:?}",
                    symbol.name, symbol.symbol_type
                );
                return Type::Void;
            }
        }
    }

    pub fn visit_binary_operator_expression(
        &self,
        node: &BinaryOperatorExpression,
        context: &mut Context,
    ) -> Type {
        let left = self.visit_expression(&node.left, context);
        let right = self.visit_expression(&node.right, context);

        if left != right {
            println!(
                "incompatible types for {:?} {:?} {:?}",
                left, node.operator, right
            );
        }

        return left; // TODO: rework this without assuming left op right result in the same type
    }
}
