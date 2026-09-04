use std::collections::HashMap;

use crate::ast::{
    Argument, AssignmentOperator, Attribute, BinaryOperator, DeclarationKind, Expression,
    ExpressionId, ExpressionKind, Function, Program, Statement, StatementKind, TypeKind,
    UnaryOperator,
};
use crate::diagnostic::Diagnostic;
use crate::token::Span;
use crate::types::Type;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub name: String,
    pub labels: Vec<Option<String>>,
    pub parameters: Vec<Type>,
    pub variadic: bool,
    pub result: Type,
    pub foreign: Option<Foreign>,
    pub requires_unsafe: bool,
    pub declaration: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Foreign {
    pub symbol: String,
    pub link: Option<String>,
}

#[derive(Debug)]
pub struct TypedProgram {
    pub program: Program,
    pub functions: Vec<Signature>,
    pub expression_types: HashMap<ExpressionId, Type>,
    pub callees: HashMap<ExpressionId, FunctionId>,
}

pub fn check(program: Program) -> Result<TypedProgram, Vec<Diagnostic>> {
    let mut checker = Checker::new(&program);
    checker.collect_signatures();
    checker.check_bodies();
    let Checker {
        functions,
        mut diagnostics,
        expression_types,
        callees,
        ..
    } = checker;
    if diagnostics.is_empty() {
        Ok(TypedProgram {
            program,
            functions,
            expression_types,
            callees,
        })
    } else {
        diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
        Err(diagnostics)
    }
}

#[derive(Clone)]
struct Local {
    declared: Type,
    mutable: bool,
}

struct Checker<'a> {
    program: &'a Program,
    functions: Vec<Signature>,
    by_name: HashMap<String, FunctionId>,
    diagnostics: Vec<Diagnostic>,
    expression_types: HashMap<ExpressionId, Type>,
    callees: HashMap<ExpressionId, FunctionId>,
    scopes: Vec<HashMap<String, Local>>,
    unsafe_depth: usize,
    result: Type,
}

impl<'a> Checker<'a> {
    fn new(program: &'a Program) -> Self {
        Checker {
            program,
            functions: Vec::new(),
            by_name: HashMap::new(),
            diagnostics: Vec::new(),
            expression_types: HashMap::new(),
            callees: HashMap::new(),
            scopes: Vec::new(),
            unsafe_depth: 0,
            result: Type::Unit,
        }
    }

    fn report(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::new(span, message));
    }

    fn unsupported(&mut self, span: Span, what: &str) -> Type {
        self.report(span, format!("{what} are not supported"));
        Type::Error
    }

    fn collect_signatures(&mut self) {
        for (index, declaration) in self.program.declarations.iter().enumerate() {
            let DeclarationKind::Function(function) = &declaration.kind;
            let (foreign, requires_unsafe) =
                self.declaration_attributes(&declaration.attributes, function);
            let labels = function
                .parameters
                .iter()
                .map(|parameter| parameter.label.clone())
                .collect();
            let parameters: Vec<Type> = function
                .parameters
                .iter()
                .map(|parameter| self.resolve_type(&parameter.annotation))
                .collect();
            let result = function
                .result
                .as_ref()
                .map_or(Type::Unit, |result| self.resolve_type(result));

            for parameter in &function.parameters {
                if let Some(default) = &parameter.default {
                    self.unsupported(default.span, "default arguments");
                }
            }
            if foreign.is_some() {
                for (parameter, declared) in function.parameters.iter().zip(&parameters) {
                    if !declared.is_representable_in_c() && !declared.is_unknown() {
                        self.report(
                            parameter.annotation.span,
                            format!("`{declared}` has no C representation"),
                        );
                    }
                }
                if let Some(annotation) = &function.result
                    && !result.is_representable_in_c()
                    && !result.is_unknown()
                {
                    self.report(
                        annotation.span,
                        format!("`{result}` has no C representation"),
                    );
                }
            }
            if function.name == "main" {
                if !function.parameters.is_empty() {
                    self.report(declaration.span, "`main` takes no parameters");
                }
                if result != Type::Unit {
                    self.report(declaration.span, "`main` returns unit");
                }
                if foreign.is_some() {
                    self.report(declaration.span, "`main` cannot be `@extern`");
                }
            }

            let id = FunctionId(self.functions.len());
            if self.by_name.insert(function.name.clone(), id).is_some() {
                self.report(
                    declaration.span,
                    format!("`{}` is already declared", function.name),
                );
            }
            self.functions.push(Signature {
                name: function.name.clone(),
                labels,
                parameters,
                variadic: function.variadic,
                result,
                foreign,
                requires_unsafe,
                declaration: index,
            });
        }
    }

    fn declaration_attributes(
        &mut self,
        attributes: &[Attribute],
        function: &Function,
    ) -> (Option<Foreign>, bool) {
        let mut foreign = None;
        let mut requires_unsafe = false;
        for attribute in attributes {
            match attribute.name.as_str() {
                "extern" => foreign = Some(self.foreign_attribute(attribute, &function.name)),
                "unsafe" => {
                    if !attribute.arguments.is_empty() {
                        self.report(attribute.span, "`@unsafe` takes no arguments");
                    }
                    requires_unsafe = true;
                }
                other => self.report(
                    attribute.span,
                    format!("attribute `@{other}` is not supported"),
                ),
            }
        }
        (foreign, requires_unsafe)
    }

    fn foreign_attribute(&mut self, attribute: &Attribute, function_name: &str) -> Foreign {
        let mut arguments = attribute.arguments.iter();
        match arguments.next() {
            Some(Argument {
                label: None, value, ..
            }) if matches!(&value.kind, ExpressionKind::ImplicitMember(abi) if abi == "c") => {}
            Some(Argument {
                label: None, value, ..
            }) => self.report(value.span, "`@extern` ABI must be `.c`"),
            _ => self.report(
                attribute.span,
                "`@extern` needs an ABI as its first argument, such as `.c`",
            ),
        }
        let mut symbol = None;
        let mut link = None;
        for argument in arguments {
            match argument.label.as_deref() {
                Some("symbol") => symbol = self.string_argument(argument, "symbol"),
                Some("link") => link = self.string_argument(argument, "link"),
                Some("callconv") => self.report(argument.span, "`callconv:` is not supported"),
                Some(other) => self.report(
                    argument.span,
                    format!("unknown `@extern` argument `{other}:`"),
                ),
                None => self.report(
                    argument.span,
                    "`@extern` takes labelled arguments after the ABI",
                ),
            }
        }
        Foreign {
            symbol: symbol.unwrap_or_else(|| function_name.to_string()),
            link,
        }
    }

    fn string_argument(&mut self, argument: &Argument, label: &str) -> Option<String> {
        match &argument.value.kind {
            ExpressionKind::String(text) => Some(text.clone()),
            _ => {
                self.report(
                    argument.value.span,
                    format!("`{label}:` takes a string literal"),
                );
                None
            }
        }
    }

    fn resolve_type(&mut self, annotation: &crate::ast::Type) -> Type {
        match &annotation.kind {
            TypeKind::Named { name, arguments } => {
                if !arguments.is_empty() {
                    return self.unsupported(annotation.span, "generic types");
                }
                match name.as_str() {
                    "string" => self.unsupported(annotation.span, "`string` values"),
                    "void" => {
                        self.report(annotation.span, "`void` is only allowed behind a pointer");
                        Type::Error
                    }
                    _ => Type::primitive(name).unwrap_or_else(|| {
                        self.report(annotation.span, format!("unknown type `{name}`"));
                        Type::Error
                    }),
                }
            }
            TypeKind::Pointer { mutable, pointee } => {
                let pointee = match &pointee.kind {
                    TypeKind::Named { name, arguments }
                        if name == "void" && arguments.is_empty() =>
                    {
                        Type::Void
                    }
                    _ => self.resolve_type(pointee),
                };
                Type::Pointer {
                    mutable: *mutable,
                    pointee: Box::new(pointee),
                }
            }
            TypeKind::Function {
                parameters,
                variadic,
                result,
            } => Type::Function {
                parameters: parameters
                    .iter()
                    .map(|parameter| self.resolve_type(parameter))
                    .collect(),
                variadic: *variadic,
                result: Box::new(
                    result
                        .as_ref()
                        .map_or(Type::Unit, |result| self.resolve_type(result)),
                ),
            },
            TypeKind::Array { .. }
            | TypeKind::Slice(_)
            | TypeKind::Optional(_)
            | TypeKind::Result { .. }
            | TypeKind::Reference { .. } => {
                self.report(
                    annotation.span,
                    format!("type `{annotation}` is not supported"),
                );
                Type::Error
            }
        }
    }

    fn check_bodies(&mut self) {
        for (index, declaration) in self.program.declarations.iter().enumerate() {
            let DeclarationKind::Function(function) = &declaration.kind;
            let Some(body) = &function.body else {
                continue;
            };
            let signature = self.functions[index].clone();
            let mut parameters = HashMap::new();
            for (parameter, declared) in function.parameters.iter().zip(&signature.parameters) {
                let local = Local {
                    declared: declared.clone(),
                    mutable: false,
                };
                if parameters.insert(parameter.name.clone(), local).is_some() {
                    self.report(
                        parameter.span,
                        format!("parameter `{}` is already declared", parameter.name),
                    );
                }
            }
            self.scopes = vec![parameters];
            self.result = signature.result.clone();
            self.unsafe_depth = 0;

            let ExpressionKind::Block { statements, value } = &body.kind else {
                unreachable!("function bodies are blocks")
            };
            let found = self.check_block(statements, value.as_deref(), None);
            self.expression_types.insert(body.id, found.clone());
            if signature.result != Type::Unit && !found.is_unknown() {
                self.report(
                    declaration.span,
                    format!(
                        "function `{}` declares `-> {}` but its body does not return",
                        function.name, signature.result
                    ),
                );
            }
        }
    }

    fn lookup_local(&self, name: &str) -> Option<&Local> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn signature_type(signature: &Signature) -> Type {
        Type::Function {
            parameters: signature.parameters.clone(),
            variadic: signature.variadic,
            result: Box::new(signature.result.clone()),
        }
    }

    fn check_block(
        &mut self,
        statements: &'a [Statement],
        value: Option<&'a Expression>,
        expected: Option<&Type>,
    ) -> Type {
        self.scopes.push(HashMap::new());
        let mut diverges = false;
        for statement in statements {
            diverges = self.check_statement(statement);
        }
        let found = match value {
            Some(value) => self.check_expression(value, expected),
            None if diverges => Type::Never,
            None => Type::Unit,
        };
        self.scopes.pop();
        found
    }

    fn check_statement(&mut self, statement: &'a Statement) -> bool {
        match &statement.kind {
            StatementKind::Let {
                mutable,
                name,
                annotation,
                value,
            } => {
                let declared = match annotation {
                    Some(annotation) => {
                        let declared = self.resolve_type(annotation);
                        self.check_against(value, &declared);
                        declared
                    }
                    None => self.check_expression(value, None),
                };
                let scope = self.scopes.last_mut().expect("a scope is always open");
                let local = Local {
                    declared,
                    mutable: *mutable,
                };
                if scope.insert(name.clone(), local).is_some() {
                    self.report(
                        statement.span,
                        format!("`{name}` is already declared in this scope"),
                    );
                }
                false
            }
            StatementKind::Assignment {
                operator,
                target,
                value,
            } => {
                self.check_assignment(*operator, target, value);
                false
            }
            StatementKind::Return(value) => {
                match value {
                    Some(value) if self.result == Type::Unit => {
                        self.check_expression(value, None);
                        self.report(
                            value.span,
                            "`return` carries a value in a function returning unit",
                        );
                    }
                    Some(value) => {
                        let result = self.result.clone();
                        self.check_against(value, &result);
                    }
                    None if self.result != Type::Unit => {
                        let result = self.result.clone();
                        self.report(
                            statement.span,
                            format!("`return` needs a value of type `{result}`"),
                        );
                    }
                    None => {}
                }
                true
            }
            StatementKind::Expression(expression) => {
                self.check_expression(expression, None);
                false
            }
        }
    }

    fn check_assignment(
        &mut self,
        operator: AssignmentOperator,
        target: &'a Expression,
        value: &'a Expression,
    ) {
        let ExpressionKind::Identifier(name) = &target.kind else {
            self.report(target.span, "only bindings can be assigned");
            self.check_expression(value, None);
            return;
        };
        let Some(local) = self.lookup_local(name).cloned() else {
            let message = if self.by_name.contains_key(name) {
                format!("`{name}` is a function, not a binding")
            } else {
                format!("unknown binding `{name}`")
            };
            self.report(target.span, message);
            self.check_expression(value, None);
            return;
        };
        if !local.mutable {
            self.report(
                target.span,
                format!("`{name}` is immutable; declare it with `let mut`"),
            );
        }
        self.expression_types
            .insert(target.id, local.declared.clone());
        self.check_against(value, &local.declared);

        use AssignmentOperator::*;
        let allowed = match operator {
            Assign => true,
            Add | Subtract | Multiply | Divide | Modulo => local.declared.is_numeric(),
            BitwiseAnd | BitwiseOr | BitwiseXor | ShiftLeft | ShiftRight => {
                local.declared.is_integer()
            }
            BooleanAnd | BooleanOr | BooleanXor => local.declared == Type::Bool,
        };
        if !allowed && !local.declared.is_unknown() {
            self.report(
                target.span,
                format!(
                    "`{}` cannot be applied to `{}`",
                    operator.symbol(),
                    local.declared
                ),
            );
        }
    }

    fn check_against(&mut self, expression: &'a Expression, expected: &Type) -> Type {
        let found = self.check_expression(expression, Some(expected));
        if !expected.accepts(&found) {
            self.report(
                expression.span,
                format!("expected `{expected}`, found `{found}`"),
            );
        }
        found
    }

    fn check_expression(&mut self, expression: &'a Expression, expected: Option<&Type>) -> Type {
        let found = match &expression.kind {
            ExpressionKind::Integer(_) => match expected {
                Some(expected) if expected.is_integer() => expected.clone(),
                _ => Type::I32,
            },
            ExpressionKind::Float(_) => match expected {
                Some(expected) if expected.is_float() => expected.clone(),
                _ => Type::F64,
            },
            ExpressionKind::String(_) => self.unsupported(expression.span, "`string` values"),
            ExpressionKind::ByteString(_) => {
                self.unsupported(expression.span, "byte string literals")
            }
            ExpressionKind::CString(_) => Type::Pointer {
                mutable: false,
                pointee: Box::new(Type::U8),
            },
            ExpressionKind::Character(_) => Type::Char,
            ExpressionKind::Boolean(_) => Type::Bool,
            ExpressionKind::Identifier(name) => self.resolve_name(name, expression.span),
            ExpressionKind::ImplicitMember(_) => {
                self.unsupported(expression.span, "implicit members")
            }
            ExpressionKind::Unary { operator, operand } => {
                self.check_unary(*operator, operand, expected, expression.span)
            }
            ExpressionKind::Binary {
                operator,
                left,
                right,
            } => self.check_binary(*operator, left, right, expected, expression.span),
            ExpressionKind::Call { callee, arguments } => {
                self.check_call(expression, callee, arguments)
            }
            ExpressionKind::Field { .. } => self.unsupported(expression.span, "field accesses"),
            ExpressionKind::Index { .. } => self.unsupported(expression.span, "index expressions"),
            ExpressionKind::Block { statements, value } => {
                self.check_block(statements, value.as_deref(), expected)
            }
            ExpressionKind::Unsafe(inner) => {
                self.unsafe_depth += 1;
                let found = self.check_expression(inner, expected);
                self.unsafe_depth -= 1;
                found
            }
        };
        self.expression_types.insert(expression.id, found.clone());
        found
    }

    fn resolve_name(&mut self, name: &str, span: Span) -> Type {
        if let Some(local) = self.lookup_local(name) {
            return local.declared.clone();
        }
        if let Some(id) = self.by_name.get(name) {
            return Self::signature_type(&self.functions[id.0]);
        }
        self.report(span, format!("unknown name `{name}`"));
        Type::Error
    }

    fn check_unary(
        &mut self,
        operator: UnaryOperator,
        operand: &'a Expression,
        expected: Option<&Type>,
        span: Span,
    ) -> Type {
        let found = match operator {
            UnaryOperator::Not => self.check_against(operand, &Type::Bool),
            _ => self.check_expression(operand, expected),
        };
        if found.is_unknown() {
            return found;
        }
        let allowed = match operator {
            UnaryOperator::Negate => found.is_signed(),
            UnaryOperator::Not => true,
            UnaryOperator::BitwiseNot => found.is_integer(),
        };
        if !allowed {
            self.report(
                span,
                format!("`{}` cannot be applied to `{found}`", operator.symbol()),
            );
            return Type::Error;
        }
        found
    }

    fn check_binary(
        &mut self,
        operator: BinaryOperator,
        left: &'a Expression,
        right: &'a Expression,
        expected: Option<&Type>,
        span: Span,
    ) -> Type {
        use BinaryOperator::*;
        let boolean = matches!(operator, BooleanAnd | BooleanOr | BooleanXor);
        let comparison = matches!(
            operator,
            Less | LessEqual | Greater | GreaterEqual | Equal | NotEqual
        );
        if boolean {
            self.check_against(left, &Type::Bool);
            self.check_against(right, &Type::Bool);
            return Type::Bool;
        }

        let operand_expectation = match expected {
            Some(expected) if !comparison && expected.is_numeric() => Some(expected),
            _ => None,
        };
        let swapped = is_literal(left) && !is_literal(right);
        let (anchor, other) = if swapped {
            (right, left)
        } else {
            (left, right)
        };
        let anchor_type = self.check_expression(anchor, operand_expectation);
        let operand = if swapped && !anchor_type.is_numeric() && !anchor_type.is_unknown() {
            let literal = self.check_expression(other, operand_expectation);
            self.report(
                anchor.span,
                format!("expected `{literal}`, found `{anchor_type}`"),
            );
            Type::Error
        } else {
            let other_type = self.check_against(other, &anchor_type);
            if anchor_type.is_unknown() {
                other_type
            } else {
                anchor_type
            }
        };
        if operand.is_unknown() {
            return if comparison { Type::Bool } else { operand };
        }

        let allowed = match operator {
            Add | Subtract | Multiply | Divide | Modulo => operand.is_numeric(),
            ShiftLeft | ShiftRight | BitwiseAnd | BitwiseXor | BitwiseOr => operand.is_integer(),
            Less | LessEqual | Greater | GreaterEqual => {
                operand.is_numeric() || operand == Type::Char
            }
            Equal | NotEqual => !matches!(operand, Type::Unit | Type::Function { .. }),
            BooleanAnd | BooleanXor | BooleanOr => unreachable!("handled above"),
        };
        if !allowed {
            self.report(
                span,
                format!("`{}` cannot be applied to `{operand}`", operator.symbol()),
            );
            return Type::Error;
        }
        if comparison { Type::Bool } else { operand }
    }

    fn check_call(
        &mut self,
        call: &'a Expression,
        callee: &'a Expression,
        arguments: &'a [Argument],
    ) -> Type {
        let function = match &callee.kind {
            ExpressionKind::Identifier(name) if self.lookup_local(name).is_none() => {
                self.by_name.get(name).copied()
            }
            _ => None,
        };
        let Some(id) = function else {
            let message = match &callee.kind {
                ExpressionKind::Identifier(name) if self.lookup_local(name).is_some() => {
                    format!("`{name}` is a binding, not a function")
                }
                ExpressionKind::Identifier(name) => format!("unknown function `{name}`"),
                _ => "only named functions can be called".to_string(),
            };
            self.report(callee.span, message);
            for argument in arguments {
                self.check_expression(&argument.value, None);
            }
            return Type::Error;
        };
        let signature = self.functions[id.0].clone();
        self.expression_types
            .insert(callee.id, Self::signature_type(&signature));
        self.callees.insert(call.id, id);

        if self.unsafe_depth == 0 {
            if signature.foreign.is_some() {
                self.report(
                    call.span,
                    format!(
                        "`{}` is `@extern`, so calling it requires an `unsafe` block",
                        signature.name
                    ),
                );
            } else if signature.requires_unsafe {
                self.report(
                    call.span,
                    format!(
                        "`{}` is `@unsafe`, so calling it requires an `unsafe` block",
                        signature.name
                    ),
                );
            }
        }

        let declared = signature.parameters.len();
        let too_few = arguments.len() < declared;
        let too_many = !signature.variadic && arguments.len() > declared;
        if too_few || too_many {
            let count = if signature.variadic {
                format!("at least {declared}")
            } else {
                declared.to_string()
            };
            let plural = if declared == 1 { "" } else { "s" };
            self.report(
                call.span,
                format!(
                    "`{}` takes {count} argument{plural}, found {}",
                    signature.name,
                    arguments.len()
                ),
            );
        }

        for (index, argument) in arguments.iter().enumerate() {
            if index < declared {
                let label = &signature.labels[index];
                if argument.label != *label {
                    let message = match (label, &argument.label) {
                        (Some(label), None) => {
                            format!("argument {} needs the label `{label}:`", index + 1)
                        }
                        (None, Some(found)) => {
                            format!("argument {} takes no label, found `{found}:`", index + 1)
                        }
                        (Some(label), Some(found)) => {
                            format!("expected label `{label}:`, found `{found}:`")
                        }
                        (None, None) => unreachable!("equal labels compare equal"),
                    };
                    self.report(argument.span, message);
                }
                self.check_against(&argument.value, &signature.parameters[index]);
            } else {
                if argument.label.is_some() {
                    self.report(argument.span, "variadic arguments take no label");
                }
                let found = self.check_expression(&argument.value, None);
                let passable = found.is_numeric() || matches!(found, Type::Pointer { .. });
                if !passable && !found.is_unknown() {
                    self.report(
                        argument.value.span,
                        format!("`{found}` cannot be passed as a C variadic argument"),
                    );
                }
            }
        }
        signature.result
    }
}

fn is_literal(expression: &Expression) -> bool {
    matches!(
        expression.kind,
        ExpressionKind::Integer(_) | ExpressionKind::Float(_)
    )
}
