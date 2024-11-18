use llvm::analysis::*;
use llvm::core::*;
use llvm::prelude::*;
use llvm::target::*;
use llvm::target_machine::*;
use llvm::*;

use std::collections::HashMap;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr;

use crate::syntax::*;

pub struct FunctionRef {
    pub name: String,
    pub function_type: LLVMTypeRef,
    pub function_ref: LLVMValueRef,
}

pub struct LLVMCodeGenVisitor {
    context: *mut LLVMContext,
    module: *mut LLVMModule,
    builder: *mut LLVMBuilder,
    named_values: HashMap<String, LLVMValueRef>,
    function_table: HashMap<String, FunctionRef>,
}

impl LLVMCodeGenVisitor {
    pub fn new() -> Self {
        unsafe {
            let context = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext("main\0".as_ptr() as *const _, context);
            let builder = LLVMCreateBuilderInContext(context);
            let named_values = HashMap::<String, LLVMValueRef>::new();
            let function_table = HashMap::<String, FunctionRef>::new();

            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargets();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllAsmParsers();
            LLVM_InitializeAllAsmPrinters();

            Self {
                context,
                module,
                builder,
                named_values,
                function_table,
            }
        }
    }

    pub fn emit_ir(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    pub fn emit_asm(&self, output_file: PathBuf) {
        unsafe {
            let triple = LLVMGetDefaultTargetTriple();
            let mut error: *mut i8 = ptr::null_mut();
            let mut target: LLVMTargetRef = ptr::null_mut();
            LLVMGetTargetFromTriple(triple, &mut target, &mut error);

            let target_machine = LLVMCreateTargetMachine(
                target,
                triple,
                "generic\0".as_ptr() as *const _,
                "\0".as_ptr() as *const _,
                LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );

            let data_layout = LLVMCreateTargetDataLayout(target_machine);
            LLVMSetModuleDataLayout(self.module, data_layout);
            LLVMSetTarget(self.module, triple);

            let filename = CString::new(output_file.as_os_str().as_bytes()).unwrap();

            LLVMTargetMachineEmitToFile(
                target_machine,
                self.module,
                filename.as_ptr() as *mut _,
                LLVMCodeGenFileType::LLVMObjectFile,
                &mut error,
            );
        };
    }

    pub fn finish(&self) {
        unsafe {
            LLVMDisposeBuilder(self.builder);
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.context);
        }
    }

    pub fn visit_source_file(&mut self, node: &SourceFile) {
        self.visit_code_block(&node.code_block);
    }

    pub fn visit_code_block(&mut self, node: &CodeBlock) {
        for item in &node.items {
            self.visit_code_block_item(item);
        }
    }

    pub fn visit_code_block_container(&mut self, node: &CodeBlockContainer, bb: LLVMBasicBlockRef) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.builder, bb);

            for item in &node.code_block.items {
                self.visit_code_block_item(item);
            }
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
            Declaration::ExternDeclaration(e) => self.visit_extern_declaration(e),
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

            self.function_table.insert(
                node.identifier.name.clone(),
                FunctionRef {
                    name: node.identifier.name.clone(),
                    function_type,
                    function_ref: function,
                },
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

    pub fn visit_extern_declaration(&mut self, node: &ExternDeclaration) {
        let function_type = self.visit_function_signature(&node.signature);

        unsafe {
            let function = LLVMAddFunction(
                self.module,
                node.identifier.name.as_ptr() as *const _,
                function_type,
            );

            self.function_table.insert(
                node.identifier.name.clone(),
                FunctionRef {
                    name: node.identifier.name.clone(),
                    function_type,
                    function_ref: function,
                },
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
            let entry =
                LLVMAppendBasicBlockInContext(self.context, function, "\0".as_ptr() as *const _);
            LLVMPositionBuilderAtEnd(self.builder, entry);

            self.named_values.clear();
            for i in 0..signature.parameters.parameters.len() {
                let param = &signature.parameters.parameters[i];
                let value = LLVMGetParam(function, i as u32);
                self.named_values.insert(param.name.name.clone(), value);
            }

            let mut terminated = false;

            for i in 0..node.code_block.items.len() {
                if i != node.code_block.items.len() - 1 {
                    self.visit_code_block_item(&node.code_block.items[i]);
                } else {
                    if let CodeBlockItem::Expression(expr) = &node.code_block.items[i] {
                        let value = self.visit_expression(expr);

                        if signature.return_clause.is_some() {
                            LLVMBuildRet(self.builder, value);
                            terminated = true;
                        }
                    } else {
                        self.visit_code_block_item(&node.code_block.items[i]);
                    }
                }
            }

            if !terminated {
                LLVMBuildRetVoid(self.builder);
            }
        }
    }

    pub fn visit_statement(&mut self, node: &Statement) {}

    pub fn visit_expression(&mut self, node: &Expression) -> LLVMValueRef {
        match node {
            Expression::BooleanLiteral(value) => self.visit_boolean_literal(*value),
            Expression::FloatNumberLiteral(value) => self.visit_float_number_literal(*value),
            Expression::Identifier(identifier) => self.visit_identifier(identifier),
            Expression::Tuple(tuple) => self.visit_tuple(tuple),
            Expression::BinaryOperator(op) => self.visit_binary_operator_expression(op),
            Expression::FunctionCall(function_call) => self.visit_function_call(function_call),
            Expression::If(if_expression) => self.visit_if_expression(if_expression),
        }
    }

    pub fn visit_if_expression(&mut self, node: &IfExpression) -> LLVMValueRef {
        unsafe {
            let condition = self.visit_expression(&node.condition);
            let bb = LLVMGetBasicBlockParent(LLVMGetInsertBlock(self.builder));

            let mut then_bb =
                LLVMAppendBasicBlockInContext(self.context, bb, "\0".as_ptr() as *const _);
            let mut else_bb =
                LLVMAppendBasicBlockInContext(self.context, bb, "\0".as_ptr() as *const _);
            let merge_bb =
                LLVMAppendBasicBlockInContext(self.context, bb, "\0".as_ptr() as *const _);

            LLVMBuildCondBr(self.builder, condition, then_bb, else_bb);

            LLVMPositionBuilderAtEnd(self.builder, then_bb);
            let mut then_value = self.visit_expression(&node.then_expression);
            LLVMBuildBr(self.builder, merge_bb);

            LLVMPositionBuilderAtEnd(self.builder, else_bb);
            let mut else_value = self.visit_expression(&node.else_expression);
            LLVMBuildBr(self.builder, merge_bb);

            LLVMPositionBuilderAtEnd(self.builder, merge_bb);
            let phi = LLVMBuildPhi(
                self.builder,
                LLVMDoubleTypeInContext(self.context), // TODO: double for now but should be type checked
                "\0".as_ptr() as *const _,
            );

            LLVMAddIncoming(phi, &mut then_value, &mut then_bb, 1);
            LLVMAddIncoming(phi, &mut else_value, &mut else_bb, 1);

            phi
        }
    }

    pub fn visit_function_call(&mut self, node: &FunctionCallExpression) -> LLVMValueRef {
        unsafe {
            let function_ref = self
                .function_table
                .get(&node.function.name)
                .expect(format!("Function {:?} not registered", node.function.name).as_str());

            let function_type = function_ref.function_type;
            let function = function_ref.function_ref;

            let param_count = LLVMCountParams(function_ref.function_ref);

            let mut args: Vec<LLVMValueRef> = node
                .arguments
                .expressions
                .items
                .iter()
                .map(|arg| self.visit_expression(arg))
                .collect::<Vec<_>>();

            let llvm_name = CString::new("").unwrap();

            LLVMDumpValue(function);
            LLVMDumpType(function_type);
            dbg!(param_count);

            LLVMBuildCall2(
                self.builder,
                function_type,
                function,
                args.as_mut_ptr(),
                param_count,
                llvm_name.as_ptr() as *mut _,
            )
        }
    }

    pub fn visit_boolean_literal(&self, value: bool) -> LLVMValueRef {
        unsafe {
            LLVMConstInt(
                LLVMInt1TypeInContext(self.context),
                if value { 1 } else { 0 },
                0,
            )
        }
    }

    pub fn visit_float_number_literal(&self, value: f64) -> LLVMValueRef {
        println!("creating ssa var: {}", value);
        unsafe { LLVMConstReal(LLVMDoubleTypeInContext(self.context), value) }
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
                    LLVMBuildFAdd(self.builder, lhs, rhs, "\0".as_ptr() as *const _)
                }
                BinaryOperator::Subtract => {
                    LLVMBuildFSub(self.builder, lhs, rhs, "\0".as_ptr() as *const _)
                }
                BinaryOperator::Multiply => {
                    LLVMBuildFMul(self.builder, lhs, rhs, "\0".as_ptr() as *const _)
                }
                BinaryOperator::Divide => {
                    LLVMBuildFDiv(self.builder, lhs, rhs, "\0".as_ptr() as *const _)
                }
            }
        }
    }
}
