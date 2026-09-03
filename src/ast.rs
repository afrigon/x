//! Abstract syntax tree for `x`.
//!
//! Every node carries the [`Span`] it was parsed from. This module currently
//! covers the expression grammar; statements, items (`fun` declarations), and
//! types are added in later parser rounds.
//!
//! [`Expression`]'s [`Display`] renders an s-expression (e.g. `(+ 1 (* 2 3))`),
//! which keeps parser tests readable and doubles as a debug dump.

use crate::token::Span;
use std::fmt;

/// An expression node: its kind plus the source span it covers.
#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}

impl Expression {
    pub fn new(kind: ExpressionKind, span: Span) -> Self {
        Expression { kind, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExpressionKind {
    // ---- Literals ------------------------------------------------------
    Integer(u128),
    Float(String),
    String(String),
    ByteString(Vec<u8>),
    CString(Vec<u8>),
    Character(char),
    Boolean(bool),

    /// A bare name reference, e.g. `count`.
    Identifier(String),
    /// A leading-dot member, e.g. `.plus` — an enum variant or implicit member
    /// whose type comes from context. `.number(n)` is a `Call` of this.
    ImplicitMember(String),

    /// Prefix operator application: `-x`, `!ok`, `!!bits`.
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    /// Infix operator application.
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Function/method call: `callee(arguments)`.
    Call {
        callee: Box<Expression>,
        arguments: Vec<Argument>,
    },
    /// Member access: `receiver.name`.
    Field {
        receiver: Box<Expression>,
        name: String,
    },
    /// Index: `receiver[index]`.
    Index {
        receiver: Box<Expression>,
        index: Box<Expression>,
    },

    /// A brace-delimited block, itself an expression (§9.5). Its value is the
    /// trailing bare expression, if any; otherwise unit.
    Block {
        statements: Vec<Statement>,
        value: Option<Box<Expression>>,
    },
    /// An `unsafe { ... }` block (§16.10); the inner expression is a `Block`.
    Unsafe(Box<Expression>),
}

// ====================================================================
// Types
// ====================================================================

/// A type expression: its kind plus the source span it covers.
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
    /// A named type with optional generic arguments: `i32`, `string`, `Self`,
    /// `List<i32>`, `HashMap<K, V>`.
    Named {
        name: String,
        arguments: Vec<Type>,
    },
    /// A fixed-size array: `[T; N]`, where `size` is a constant expression.
    Array {
        element: Box<Type>,
        size: Box<Expression>,
    },
    /// A slice: `[T]`.
    Slice(Box<Type>),
    /// An optional: `T?` (sugar over `Option<T>`).
    Optional(Box<Type>),
    /// A result: `T!E` (sugar over `Result<T, E>`).
    Result {
        value: Box<Type>,
        error: Box<Type>,
    },
    /// A reference: `&T` / `&mut T`.
    Reference {
        mutable: bool,
        referent: Box<Type>,
    },
    /// A raw pointer: `*T` / `*mut T`.
    Pointer {
        mutable: bool,
        pointee: Box<Type>,
    },
    /// A function pointer type: `fun(T, U) -> R`, optionally variadic.
    Function {
        parameters: Vec<Type>,
        variadic: bool,
        result: Option<Box<Type>>,
    },
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
                write!(formatter, "&{}{referent}", if *mutable { "mut " } else { "" })
            }
            Pointer { mutable, pointee } => {
                write!(formatter, "*{}{pointee}", if *mutable { "mut " } else { "" })
            }
            Function { parameters, variadic, result } => {
                write!(formatter, "fun(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "{parameter}")?;
                }
                if *variadic {
                    write!(formatter, "{}...", if parameters.is_empty() { "" } else { ", " })?;
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

/// A single call argument, optionally carrying a Swift-style label (`b: 5`).
#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub label: Option<String>,
    pub value: Expression,
    pub span: Span,
}

// ====================================================================
// Statements
// ====================================================================

/// A statement: its kind plus the source span it covers.
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
    /// `let [mut] name [: Type] := value`.
    Let {
        mutable: bool,
        name: String,
        annotation: Option<Type>,
        value: Expression,
    },
    /// A binding or mutation: `target := value`, or a compound form (`+=`, …).
    Assignment {
        operator: AssignmentOperator,
        target: Expression,
        value: Expression,
    },
    /// `return` with an optional value.
    Return(Option<Expression>),
    /// A bare expression used for its effect.
    Expression(Expression),
}

/// The operator of an assignment statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignmentOperator {
    Assign, // :=
    // Arithmetic compound
    Add,      // +=
    Subtract, // -=
    Multiply, // *=
    Divide,   // /=
    Modulo,   // %=
    // Boolean compound
    BooleanAnd, // &=
    BooleanOr,  // |=
    BooleanXor, // ^=
    // Bitwise compound
    BitwiseAnd, // &&=
    BitwiseOr,  // ||=
    BitwiseXor, // ^^=
    ShiftLeft,  // <<=
    ShiftRight, // >>=
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
            Let { mutable, name, annotation, value } => {
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
            Assignment { operator, target, value } => {
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
    Negate,     // -
    Not,        // !   (boolean)
    BitwiseNot, // !!
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    // Shifts
    ShiftLeft,  // <<
    ShiftRight, // >>
    // Bitwise
    BitwiseAnd, // &&
    BitwiseXor, // ^^
    BitwiseOr,  // ||
    // Relational
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    // Equality
    Equal,    // =
    NotEqual, // !=
    // Boolean (short-circuit)
    BooleanAnd, // &
    BooleanXor, // ^
    BooleanOr,  // |
}

/// Binding power for the precedence-climbing parser. Higher binds tighter.
/// Follows Rust's precedence (with `x`'s symbols substituted by role): unary >
/// `* / %` > `+ -` > shifts > bitwise `&&` > `^^` > `||` > comparison > boolean
/// `&` > `^` > `|`. All comparison operators share one **non-associative**
/// level — `a < b < c` is a parse error, matching Rust and Swift.
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

    /// Comparison operators do not associate: chaining them (`a < b < c`,
    /// `a = b != c`) is a parse error and must be parenthesized.
    pub fn is_non_associative(self) -> bool {
        use BinaryOperator::*;
        matches!(self, Less | LessEqual | Greater | GreaterEqual | Equal | NotEqual)
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
            ByteString(bytes) => write!(formatter, "b{:?}", std::string::String::from_utf8_lossy(bytes)),
            CString(bytes) => write!(formatter, "c{:?}", std::string::String::from_utf8_lossy(bytes)),
            Character(character) => write!(formatter, "'{character}'"),
            Boolean(value) => write!(formatter, "{value}"),
            Identifier(name) => write!(formatter, "{name}"),
            ImplicitMember(name) => write!(formatter, ".{name}"),
            Unary { operator, operand } => write!(formatter, "({} {operand})", operator.symbol()),
            Binary { operator, left, right } => {
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
                    match &argument.label {
                        Some(label) => write!(formatter, " {label}: {}", argument.value)?,
                        None => write!(formatter, " {}", argument.value)?,
                    }
                }
                write!(formatter, ")")
            }
        }
    }
}
