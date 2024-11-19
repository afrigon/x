use crate::token::Token;

#[derive(Debug)]
pub struct SourceFile {
    pub code_block: CodeBlock,
}

#[derive(Debug)]
pub struct CodeBlockContainer {
    pub code_block: CodeBlock,
}

#[derive(Debug)]
pub struct CodeBlock {
    pub items: Vec<CodeBlockItem>,
}

#[derive(Debug)]
pub enum CodeBlockItem {
    Declaration(Declaration),
    Expression(Expression),
    Statement(Statement),
}

#[derive(Debug)]
pub enum Declaration {
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    ExternDeclaration(ExternDeclaration),
    TypeDeclaration(TypeDeclaration),
    EnumDeclaration(EnumDeclaration),
}

#[derive(Debug)]
pub struct TypeDeclaration {
    pub name: Identifier,
    pub container: MemberBlockContainer,
}

#[derive(Debug)]
pub struct EnumDeclaration {
    pub name: Identifier,
    pub container: MemberBlockContainer,
}

#[derive(Debug)]
pub struct MemberBlockContainer {
    pub member_block: MemberBlock,
}

#[derive(Debug)]
pub struct MemberBlock {
    pub members: Vec<MemberBlockItem>,
}

#[derive(Debug)]
pub enum MemberBlockItem {
    // EnumCaseDeclaration(EnumCaseDeclaration),
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
}

#[derive(Debug)]
pub struct VariableDeclaration {
    pub identifier: Identifier,
    pub expression: Expression,
}

#[derive(Debug)]
pub struct FunctionDeclaration {
    pub identifier: Identifier,
    pub signature: FunctionSignature,
    pub body: CodeBlockContainer,
}

#[derive(Debug)]
pub struct ExternDeclaration {
    pub identifier: Identifier,
    pub signature: FunctionSignature,
}

#[derive(Debug)]
pub struct FunctionSignature {
    pub parameters: FunctionParameters,
    pub return_clause: Option<ReturnClause>,
}

#[derive(Debug)]
pub struct FunctionParameters {
    pub parameters: Vec<FunctionParameter>,
}

#[derive(Debug)]
pub struct FunctionParameter {
    pub label: Option<Identifier>,
    pub name: Identifier,
    pub parameter_type: TypeSyntax,
}

#[derive(Debug)]
pub struct ReturnClause {
    pub return_type: TypeSyntax,
}

#[derive(Debug)]
pub enum Expression {
    Identifier(Identifier),
    FunctionCall(FunctionCallExpression),
    BooleanLiteral(bool),
    FloatNumberLiteral(f64),
    NilLiteral,
    // IntegerNumberLiteral(u64)
    BinaryOperator(BinaryOperatorExpression),
    Tuple(TupleExpression),
    If(IfExpression),
    // MatchExpression
}

#[derive(Debug)]
pub struct IfExpression {
    pub condition: Box<Expression>,
    pub then_expression: Box<Expression>,
    pub else_expression: Box<Expression>,
}

#[derive(Debug)]
pub struct FunctionCallExpression {
    pub function: Identifier,
    pub arguments: TupleExpression,
}

#[derive(Debug)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl BinaryOperator {
    pub fn from_token(token: &Token) -> Option<BinaryOperator> {
        match token {
            Token::Plus => Some(BinaryOperator::Add),
            Token::Minus => Some(BinaryOperator::Subtract),
            Token::Asterisk => Some(BinaryOperator::Multiply),
            Token::Slash => Some(BinaryOperator::Divide),
            _ => None,
        }
    }

    pub fn precedence(&self) -> u32 {
        match self {
            BinaryOperator::Add => 10,
            BinaryOperator::Subtract => 10,
            BinaryOperator::Multiply => 20,
            BinaryOperator::Divide => 20,
        }
    }
}

#[derive(Debug)]
pub struct BinaryOperatorExpression {
    pub operator: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug)]
pub struct TupleExpression {
    pub expressions: ExpressionList,
}

#[derive(Debug)]
pub struct ExpressionList {
    pub items: Vec<Expression>,
}

#[derive(Debug)]
pub enum Statement {
    // return
    // break
    // continue
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum TypeSyntax {
    IdentifierType(Identifier),
}
