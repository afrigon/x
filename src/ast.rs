use crate::token::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub attributes: Vec<Attribute>,
    pub kind: DeclarationKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<Argument>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclarationKind {
    Function(Function),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub variadic: bool,
    pub result: Option<Type>,
    pub body: Option<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub label: Option<String>,
    pub name: String,
    pub annotation: Type,
    pub default: Option<Expression>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExpressionId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    pub id: ExpressionId,
    pub kind: ExpressionKind,
    pub span: Span,
}

impl Expression {
    pub fn new(id: ExpressionId, kind: ExpressionKind, span: Span) -> Self {
        Expression { id, kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExpressionKind {
    Integer(u128),
    Float(String),
    String(String),
    ByteString(Vec<u8>),
    CString(Vec<u8>),
    Character(char),
    Boolean(bool),

    Identifier(String),
    ImplicitMember(String),

    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },

    Call {
        callee: Box<Expression>,
        arguments: Vec<Argument>,
    },
    Field {
        receiver: Box<Expression>,
        name: String,
    },
    Index {
        receiver: Box<Expression>,
        index: Box<Expression>,
    },

    Block {
        statements: Vec<Statement>,
        value: Option<Box<Expression>>,
    },
    Unsafe(Box<Expression>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

impl Type {
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Type { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Named {
        name: String,
        arguments: Vec<Type>,
    },
    Array {
        element: Box<Type>,
        size: Box<Expression>,
    },
    Slice(Box<Type>),
    Optional(Box<Type>),
    Result {
        value: Box<Type>,
        error: Box<Type>,
    },
    Reference {
        mutable: bool,
        referent: Box<Type>,
    },
    Pointer {
        mutable: bool,
        pointee: Box<Type>,
    },
    Function {
        parameters: Vec<Type>,
        variadic: bool,
        result: Option<Box<Type>>,
    },
}

impl fmt::Display for Program {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for declaration in &self.declarations {
            writeln!(formatter, "{declaration}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Declaration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for attribute in &self.attributes {
            write!(formatter, "{attribute} ")?;
        }
        match &self.kind {
            DeclarationKind::Function(function) => write!(formatter, "{function}"),
        }
    }
}

impl fmt::Display for Attribute {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "(@{}", self.name)?;
        for argument in &self.arguments {
            write!(formatter, " {argument}")?;
        }
        write!(formatter, ")")
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.label {
            Some(label) => write!(formatter, "{label}: {}", self.value),
            None => write!(formatter, "{}", self.value),
        }
    }
}

impl fmt::Display for Function {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "(fun {}", self.name)?;
        for parameter in &self.parameters {
            write!(formatter, " {parameter}")?;
        }
        if self.variadic {
            write!(formatter, " ...")?;
        }
        if let Some(result) = &self.result {
            write!(formatter, " -> {result}")?;
        }
        if let Some(body) = &self.body {
            write!(formatter, " {body}")?;
        }
        write!(formatter, ")")
    }
}

impl fmt::Display for Parameter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "(")?;
        match &self.label {
            None => write!(formatter, "_ ")?,
            Some(label) if *label != self.name => write!(formatter, "{label} ")?,
            Some(_) => {}
        }
        write!(formatter, "{}: {}", self.name, self.annotation)?;
        if let Some(default) = &self.default {
            write!(formatter, " := {default}")?;
        }
        write!(formatter, ")")
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TypeKind::*;
        match &self.kind {
            Named { name, arguments } => {
                write!(formatter, "{name}")?;
                if let Some((first, rest)) = arguments.split_first() {
                    write!(formatter, "<{first}")?;
                    for argument in rest {
                        write!(formatter, ", {argument}")?;
                    }
                    write!(formatter, ">")?;
                }
                Ok(())
            }
            Array { element, size } => write!(formatter, "[{element}; {size}]"),
            Slice(element) => write!(formatter, "[{element}]"),
            Optional(inner) => write!(formatter, "{inner}?"),
            Result { value, error } => write!(formatter, "{value}!{error}"),
            Reference { mutable, referent } => {
                write!(
                    formatter,
                    "&{}{referent}",
                    if *mutable { "mut " } else { "" }
                )
            }
            Pointer { mutable, pointee } => {
                write!(
                    formatter,
                    "*{}{pointee}",
                    if *mutable { "mut " } else { "" }
                )
            }
            Function {
                parameters,
                variadic,
                result,
            } => {
                write!(formatter, "fun(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "{parameter}")?;
                }
                if *variadic {
                    write!(
                        formatter,
                        "{}...",
                        if parameters.is_empty() { "" } else { ", " }
                    )?;
                }
                write!(formatter, ")")?;
                if let Some(result) = result {
                    write!(formatter, " -> {result}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub label: Option<String>,
    pub value: Expression,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

impl Statement {
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Statement { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatementKind {
    Let {
        mutable: bool,
        name: String,
        annotation: Option<Type>,
        value: Expression,
    },
    Assignment {
        operator: AssignmentOperator,
        target: Expression,
        value: Expression,
    },
    Return(Option<Expression>),
    Expression(Expression),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    BooleanAnd,
    BooleanOr,
    BooleanXor,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
}

impl AssignmentOperator {
    pub fn symbol(self) -> &'static str {
        use AssignmentOperator::*;
        match self {
            Assign => ":=",
            Add => "+=",
            Subtract => "-=",
            Multiply => "*=",
            Divide => "/=",
            Modulo => "%=",
            BooleanAnd => "&=",
            BooleanOr => "|=",
            BooleanXor => "^=",
            BitwiseAnd => "&&=",
            BitwiseOr => "||=",
            BitwiseXor => "^^=",
            ShiftLeft => "<<=",
            ShiftRight => ">>=",
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use StatementKind::*;
        match &self.kind {
            Let {
                mutable,
                name,
                annotation,
                value,
            } => {
                write!(formatter, "(let ")?;
                if *mutable {
                    write!(formatter, "mut ")?;
                }
                write!(formatter, "{name}")?;
                if let Some(annotation) = annotation {
                    write!(formatter, ": {annotation}")?;
                }
                write!(formatter, " := {value})")
            }
            Assignment {
                operator,
                target,
                value,
            } => {
                write!(formatter, "({} {target} {value})", operator.symbol())
            }
            Return(Some(value)) => write!(formatter, "(return {value})"),
            Return(None) => write!(formatter, "(return)"),
            Expression(expression) => write!(formatter, "{expression}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
    BitwiseNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    ShiftLeft,
    ShiftRight,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    BooleanAnd,
    BooleanXor,
    BooleanOr,
}

pub type Precedence = u8;

impl BinaryOperator {
    pub fn precedence(self) -> Precedence {
        use BinaryOperator::*;
        match self {
            Multiply | Divide | Modulo => 10,
            Add | Subtract => 9,
            ShiftLeft | ShiftRight => 8,
            BitwiseAnd => 7,
            BitwiseXor => 6,
            BitwiseOr => 5,
            Less | LessEqual | Greater | GreaterEqual | Equal | NotEqual => 4,
            BooleanAnd => 3,
            BooleanXor => 2,
            BooleanOr => 1,
        }
    }

    pub fn is_non_associative(self) -> bool {
        use BinaryOperator::*;
        matches!(
            self,
            Less | LessEqual | Greater | GreaterEqual | Equal | NotEqual
        )
    }

    pub fn symbol(self) -> &'static str {
        use BinaryOperator::*;
        match self {
            Add => "+",
            Subtract => "-",
            Multiply => "*",
            Divide => "/",
            Modulo => "%",
            ShiftLeft => "<<",
            ShiftRight => ">>",
            BitwiseAnd => "&&",
            BitwiseXor => "^^",
            BitwiseOr => "||",
            Less => "<",
            LessEqual => "<=",
            Greater => ">",
            GreaterEqual => ">=",
            Equal => "=",
            NotEqual => "!=",
            BooleanAnd => "&",
            BooleanXor => "^",
            BooleanOr => "|",
        }
    }
}

impl UnaryOperator {
    pub fn symbol(self) -> &'static str {
        match self {
            UnaryOperator::Negate => "-",
            UnaryOperator::Not => "!",
            UnaryOperator::BitwiseNot => "!!",
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ExpressionKind::*;
        match &self.kind {
            Integer(value) => write!(formatter, "{value}"),
            Float(text) => write!(formatter, "{text}"),
            String(text) => write!(formatter, "{text:?}"),
            ByteString(bytes) => write!(
                formatter,
                "b{:?}",
                std::string::String::from_utf8_lossy(bytes)
            ),
            CString(bytes) => write!(
                formatter,
                "c{:?}",
                std::string::String::from_utf8_lossy(bytes)
            ),
            Character(character) => write!(formatter, "'{character}'"),
            Boolean(value) => write!(formatter, "{value}"),
            Identifier(name) => write!(formatter, "{name}"),
            ImplicitMember(name) => write!(formatter, ".{name}"),
            Unary { operator, operand } => write!(formatter, "({} {operand})", operator.symbol()),
            Binary {
                operator,
                left,
                right,
            } => {
                write!(formatter, "({} {left} {right})", operator.symbol())
            }
            Field { receiver, name } => write!(formatter, "(. {receiver} {name})"),
            Index { receiver, index } => write!(formatter, "(index {receiver} {index})"),
            Block { statements, value } => {
                write!(formatter, "(block")?;
                for statement in statements {
                    write!(formatter, " {statement}")?;
                }
                if let Some(value) = value {
                    write!(formatter, " => {value}")?;
                }
                write!(formatter, ")")
            }
            Unsafe(block) => write!(formatter, "(unsafe {block})"),
            Call { callee, arguments } => {
                write!(formatter, "(call {callee}")?;
                for argument in arguments {
                    write!(formatter, " {argument}")?;
                }
                write!(formatter, ")")
            }
        }
    }
}
