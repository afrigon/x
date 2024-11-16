use llvm::analysis::*;
use llvm::core::*;
use llvm::prelude::*;
use llvm::*;

use std::collections::HashMap;

use crate::syntax::*;

pub struct LLVMCodeGenVisitor {
    context: *mut LLVMContext,
    module: *mut LLVMModule,
    builder: *mut LLVMBuilder,
    named_values: HashMap<String, LLVMValueRef>,
}

impl LLVMCodeGenVisitor {
    pub fn new() -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext("main\0".as_ptr() as *const _, context);
            let builder = LLVMCreateBuilderInContext(context);
            let named_values = HashMap::<String, LLVMValueRef>::new();

            Self {
                context,
                module,
                builder,
                named_values,
            }
        }
    }

    pub fn emit_ir(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    pub fn finish(&self) {
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }

    pub fn visit_source_file(&mut self, node: &SourceFile) {
        for item in &node.code_block.items {
            self.visit_code_block_item(item);
        }
    }

    pub fn visit_code_block_item(&mut self, node: &CodeBlockItem) {
        match node {
            CodeBlockItem::Declaration(decl) => {
                self.visit_declaration(decl);
            }
            CodeBlockItem::Statement(stmt) => {
                self.visit_statement(stmt);
            }
            CodeBlockItem::Expression(expr) => {
                self.visit_expression(expr);
            }
        }
    }

    pub fn visit_declaration(&mut self, node: &Declaration) {
        match node {
            Declaration::VariableDeclaration(variable) => self.visit_variable_declaration(variable),
            Declaration::FunctionDeclaration(function) => self.visit_function_declaration(function),
        }
    }

    pub fn visit_variable_declaration(&mut self, node: &VariableDeclaration) {
        let value = self.visit_expression(&node.expression);

        self.named_values
            .insert(node.identifier.name.clone(), value);
    }

    pub fn visit_function_declaration(&mut self, node: &FunctionDeclaration) {
        let function_type = self.visit_function_signature(&node.signature);

        unsafe {
            let function = LLVMAddFunction(
                self.module,
                node.identifier.name.as_ptr() as *const _,
                function_type,
            );

            for i in 0..node.signature.parameters.parameters.len() {
                let param = LLVMGetParam(function, i as u32);
                let name = node.signature.parameters.parameters[i].name.name.as_ptr() as *const _;
                LLVMSetValueName2(
                    param,
                    name,
                    node.signature.parameters.parameters[i].name.name.len() as usize,
                );
            }

            self.visit_function_body(&node.body, &node.signature, function);
            LLVMVerifyFunction(function, LLVMVerifierFailureAction::LLVMPrintMessageAction);
        }
    }

    pub fn visit_function_signature(&mut self, node: &FunctionSignature) -> LLVMTypeRef {
        unsafe {
            let mut types = self.visit_function_parameters(&node.parameters);
            let return_type = self.visit_return_clause(&node.return_clause);

            // TODO: what is isVarArg, setting to no for now
            LLVMFunctionType(return_type, types.as_mut_ptr(), types.len() as u32, 0)
        }
    }

    pub fn visit_function_parameters(&mut self, node: &FunctionParameters) -> Vec<LLVMTypeRef> {
        (0..node.parameters.len())
            .map(|_| unsafe { LLVMDoubleTypeInContext(self.context) })
            .collect()
    }

    pub fn visit_return_clause(&mut self, node: &Option<ReturnClause>) -> LLVMTypeRef {
        unsafe {
            match node {
                Some(_) => LLVMDoubleTypeInContext(self.context),
                None => LLVMVoidTypeInContext(self.context),
            }
        }
    }

    pub fn visit_function_body(
        &mut self,
        node: &CodeBlockContainer,
        signature: &FunctionSignature,
        function: LLVMValueRef,
    ) {
        unsafe {
            let entry = LLVMAppendBasicBlockInContext(
                self.context,
                function,
                "entry\0".as_ptr() as *const _,
            );
            LLVMPositionBuilderAtEnd(self.builder, entry);

            self.named_values.clear();
            for i in 0..signature.parameters.parameters.len() {
                let param = &signature.parameters.parameters[i];
                let value = LLVMGetParam(function, i as u32);
                self.named_values.insert(param.name.name.clone(), value);
            }

            for i in 0..node.code_block.items.len() {
                if i != node.code_block.items.len() - 1 {
                    self.visit_code_block_item(&node.code_block.items[i]);
                } else {
                    if let CodeBlockItem::Expression(expr) = &node.code_block.items[i] {
                        let value = self.visit_expression(expr);

                        if signature.return_clause.is_some() {
                            LLVMBuildRet(self.builder, value);
                        } else {
                            LLVMBuildRetVoid(self.builder);
                        }
                    } else {
                        self.visit_code_block_item(&node.code_block.items[i]);
                        LLVMBuildRetVoid(self.builder);
                    }
                }
            }
        }
    }

    pub fn visit_code_block_container(&mut self, node: &CodeBlockContainer) {}

    pub fn visit_statement(&mut self, node: &Statement) {}

    pub fn visit_expression(&mut self, node: &Expression) -> LLVMValueRef {
        match node {
            Expression::FloatNumberLiteral(value) => self.visit_float_number_literal(value),
            Expression::Identifier(identifier) => self.visit_identifier(identifier),
            Expression::Tuple(tuple) => self.visit_tuple(tuple),
            Expression::BinaryOperator(op) => self.visit_binary_operator_expression(op),
        }
    }

    pub fn visit_float_number_literal(&self, value: &f64) -> LLVMValueRef {
        unsafe { LLVMConstReal(LLVMDoubleTypeInContext(self.context), *value) }
    }

    pub fn visit_identifier(&mut self, identifier: &Identifier) -> LLVMValueRef {
        if let Some(value) = self.named_values.get_mut(&identifier.name) {
            *value
        } else {
            panic!("Unknown identifier: {}", identifier.name);
        }
    }

    pub fn visit_tuple(&mut self, tuple: &TupleExpression) -> LLVMValueRef {
        self.visit_expression(&tuple.expressions.items[0])
    }

    pub fn visit_binary_operator_expression(
        &mut self,
        node: &BinaryOperatorExpression,
    ) -> LLVMValueRef {
        let lhs = self.visit_expression(&node.left);
        let rhs = self.visit_expression(&node.right);

        unsafe {
            match node.operator {
                BinaryOperator::Add => {
                    LLVMBuildFAdd(self.builder, lhs, rhs, "add\0".as_ptr() as *const _)
                }
                BinaryOperator::Subtract => {
                    LLVMBuildFSub(self.builder, lhs, rhs, "sub\0".as_ptr() as *const _)
                }
                BinaryOperator::Multiply => {
                    LLVMBuildFMul(self.builder, lhs, rhs, "mul\0".as_ptr() as *const _)
                }
                BinaryOperator::Divide => {
                    LLVMBuildFDiv(self.builder, lhs, rhs, "div\0".as_ptr() as *const _)
                }
            }
        }
    }
}
