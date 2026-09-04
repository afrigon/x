use crate::ast::{
    Argument, AssignmentOperator, BinaryOperator, Declaration, DeclarationKind, Expression,
    ExpressionKind, Function, Parameter, Precedence, Program, Statement, StatementKind, Type,
    TypeKind, UnaryOperator,
};
use crate::token::{Span, Token, TokenKind};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {}, column {}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

type Parse<T> = Result<T, ParseError>;

pub fn parse_program(tokens: Vec<Token>) -> Parse<Program> {
    let mut parser = Parser::new(tokens);
    let mut declarations = Vec::new();
    parser.skip_newlines();
    while !parser.at_end() {
        declarations.push(parser.parse_declaration()?);
        parser.skip_newlines();
    }
    Ok(Program { declarations })
}

pub fn parse_expression(tokens: Vec<Token>) -> Parse<Expression> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    let expression = parser.parse_binary(0)?;
    parser.skip_newlines();
    if !parser.at_end() {
        return Err(parser.error_here(format!(
            "unexpected {} after expression",
            parser.current_kind().describe()
        )));
    }
    Ok(expression)
}

pub fn parse_block(tokens: Vec<Token>) -> Parse<Expression> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    let block = parser.parse_block()?;
    parser.skip_newlines();
    if !parser.at_end() {
        return Err(parser.error_here(format!(
            "unexpected {} after block",
            parser.current_kind().describe()
        )));
    }
    Ok(block)
}

pub fn parse_type(tokens: Vec<Token>) -> Parse<Type> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    let parsed = parser.parse_type()?;
    parser.skip_newlines();
    if !parser.at_end() {
        return Err(parser.error_here(format!(
            "unexpected {} after type",
            parser.current_kind().describe()
        )));
    }
    Ok(parsed)
}

fn identifier_name(token: Token) -> String {
    let TokenKind::Identifier(name) = token.kind else {
        unreachable!("only identifier tokens are passed here")
    };
    name
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn current_kind(&self) -> &TokenKind {
        &self.tokens[self.position].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.position].span
    }

    fn peek_kind(&self, offset: usize) -> &TokenKind {
        let index = (self.position + offset).min(self.tokens.len() - 1);
        &self.tokens[index].kind
    }

    fn at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.position += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.current_kind() == kind
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, description: &str) -> Parse<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error_here(format!(
                "expected {description}, found {}",
                self.current_kind().describe()
            )))
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn error_here(&self, message: String) -> ParseError {
        ParseError {
            message,
            span: self.current_span(),
        }
    }

    fn parse_binary(&mut self, minimum: Precedence) -> Parse<Expression> {
        let mut left = self.parse_unary()?;
        while let Some(operator) = binary_operator(self.current_kind()) {
            let precedence = operator.precedence();
            if precedence < minimum {
                break;
            }
            self.advance();
            let right = self.parse_binary(precedence + 1)?;
            let span = left.span.to(right.span);
            left = Expression::new(
                ExpressionKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
            if operator.is_non_associative()
                && binary_operator(self.current_kind())
                    .is_some_and(|next| next.precedence() == precedence)
            {
                return Err(self.error_here(
                    "comparison operators cannot be chained; parenthesize instead".to_string(),
                ));
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Parse<Expression> {
        let operator = match self.current_kind() {
            TokenKind::Minus => Some(UnaryOperator::Negate),
            TokenKind::Bang => Some(UnaryOperator::Not),
            TokenKind::BangBang => Some(UnaryOperator::BitwiseNot),
            _ => None,
        };
        let Some(operator) = operator else {
            return self.parse_postfix();
        };
        let start = self.current_span();
        self.advance();
        let operand = self.parse_unary()?;
        let span = start.to(operand.span);
        Ok(Expression::new(
            ExpressionKind::Unary {
                operator,
                operand: Box::new(operand),
            },
            span,
        ))
    }

    fn parse_postfix(&mut self) -> Parse<Expression> {
        let mut expression = self.parse_primary()?;
        loop {
            expression = match self.current_kind() {
                TokenKind::LeftParenthesis => self.parse_call(expression)?,
                TokenKind::Dot => self.parse_field(expression)?,
                TokenKind::LeftBracket => self.parse_index(expression)?,
                _ => break,
            };
        }
        Ok(expression)
    }

    fn parse_call(&mut self, callee: Expression) -> Parse<Expression> {
        self.advance();
        self.skip_newlines();
        let mut arguments = Vec::new();
        while !self.check(&TokenKind::RightParenthesis) {
            arguments.push(self.parse_argument()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        let close = self.expect(&TokenKind::RightParenthesis, "`)`")?;
        let span = callee.span.to(close.span);
        Ok(Expression::new(
            ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span,
        ))
    }

    fn parse_argument(&mut self) -> Parse<Argument> {
        let label = match (self.current_kind(), self.peek_kind(1)) {
            (TokenKind::Identifier(name), TokenKind::Colon) => {
                let name = name.clone();
                let start = self.current_span();
                self.advance();
                self.advance();
                self.skip_newlines();
                Some((name, start))
            }
            _ => None,
        };
        let value = self.parse_binary(0)?;
        let span = match &label {
            Some((_, start)) => start.to(value.span),
            None => value.span,
        };
        Ok(Argument {
            label: label.map(|(name, _)| name),
            value,
            span,
        })
    }

    fn parse_field(&mut self, receiver: Expression) -> Parse<Expression> {
        self.advance();
        let name_token = self.expect_identifier("a field name")?;
        let TokenKind::Identifier(name) = name_token.kind else {
            unreachable!()
        };
        let span = receiver.span.to(name_token.span);
        Ok(Expression::new(
            ExpressionKind::Field {
                receiver: Box::new(receiver),
                name,
            },
            span,
        ))
    }

    fn parse_index(&mut self, receiver: Expression) -> Parse<Expression> {
        self.advance();
        self.skip_newlines();
        let index = self.parse_binary(0)?;
        self.skip_newlines();
        let close = self.expect(&TokenKind::RightBracket, "`]`")?;
        let span = receiver.span.to(close.span);
        Ok(Expression::new(
            ExpressionKind::Index {
                receiver: Box::new(receiver),
                index: Box::new(index),
            },
            span,
        ))
    }

    fn parse_primary(&mut self) -> Parse<Expression> {
        let token = self.current().clone();
        let kind = match &token.kind {
            TokenKind::Integer { value, .. } => ExpressionKind::Integer(*value),
            TokenKind::Float(text) => ExpressionKind::Float(text.clone()),
            TokenKind::String(text) => ExpressionKind::String(text.clone()),
            TokenKind::ByteString(bytes) => ExpressionKind::ByteString(bytes.clone()),
            TokenKind::CString(bytes) => ExpressionKind::CString(bytes.clone()),
            TokenKind::Character(character) => ExpressionKind::Character(*character),
            TokenKind::True => ExpressionKind::Boolean(true),
            TokenKind::False => ExpressionKind::Boolean(false),
            TokenKind::Identifier(name) => ExpressionKind::Identifier(name.clone()),
            TokenKind::SelfValue => ExpressionKind::Identifier("self".to_string()),
            TokenKind::LeftParenthesis => return self.parse_grouping(),
            TokenKind::LeftBrace => return self.parse_block(),
            TokenKind::Unsafe => return self.parse_unsafe(),
            TokenKind::Dot => return self.parse_implicit_member(),
            _ => {
                return Err(self.error_here(format!(
                    "expected an expression, found {}",
                    token.kind.describe()
                )));
            }
        };
        self.advance();
        Ok(Expression::new(kind, token.span))
    }

    fn parse_grouping(&mut self) -> Parse<Expression> {
        let open = self.advance().span;
        self.skip_newlines();
        let mut inner = self.parse_binary(0)?;
        self.skip_newlines();
        let close = self.expect(&TokenKind::RightParenthesis, "`)`")?;
        inner.span = open.to(close.span);
        Ok(inner)
    }

    fn parse_implicit_member(&mut self) -> Parse<Expression> {
        let dot = self.advance().span;
        let name_token = self.expect_identifier("a member name after `.`")?;
        let TokenKind::Identifier(name) = name_token.kind else {
            unreachable!()
        };
        let span = dot.to(name_token.span);
        Ok(Expression::new(ExpressionKind::ImplicitMember(name), span))
    }

    fn expect_identifier(&mut self, description: &str) -> Parse<Token> {
        if matches!(self.current_kind(), TokenKind::Identifier(_)) {
            Ok(self.advance())
        } else {
            Err(self.error_here(format!(
                "expected {description}, found {}",
                self.current_kind().describe()
            )))
        }
    }

    // Prefix `&` / `*` bind tighter than the `?` / `!E` suffixes: `&mut Node?` is `(&mut Node)?`.
    fn parse_type(&mut self) -> Parse<Type> {
        let mut parsed = self.parse_prefix_type()?;
        loop {
            match self.current_kind() {
                TokenKind::Question => {
                    let question = self.advance();
                    let span = parsed.span.to(question.span);
                    parsed = Type::new(TypeKind::Optional(Box::new(parsed)), span);
                }
                TokenKind::Bang => {
                    self.advance();
                    let error = self.parse_prefix_type()?;
                    let span = parsed.span.to(error.span);
                    parsed = Type::new(
                        TypeKind::Result {
                            value: Box::new(parsed),
                            error: Box::new(error),
                        },
                        span,
                    );
                }
                _ => break,
            }
        }
        Ok(parsed)
    }

    fn parse_prefix_type(&mut self) -> Parse<Type> {
        match self.current_kind() {
            TokenKind::Ampersand => {
                let ampersand = self.advance();
                let mutable = self.eat(&TokenKind::Mut);
                let referent = self.parse_prefix_type()?;
                let span = ampersand.span.to(referent.span);
                Ok(Type::new(
                    TypeKind::Reference {
                        mutable,
                        referent: Box::new(referent),
                    },
                    span,
                ))
            }
            TokenKind::Star => {
                let star = self.advance();
                let mutable = self.eat(&TokenKind::Mut);
                let pointee = self.parse_prefix_type()?;
                let span = star.span.to(pointee.span);
                Ok(Type::new(
                    TypeKind::Pointer {
                        mutable,
                        pointee: Box::new(pointee),
                    },
                    span,
                ))
            }
            TokenKind::Fun => self.parse_function_type(),
            _ => self.parse_atom_type(),
        }
    }

    fn parse_atom_type(&mut self) -> Parse<Type> {
        let token = self.current().clone();
        match &token.kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if self.check(&TokenKind::Less) {
                    let (arguments, close) = self.parse_generic_arguments()?;
                    let span = token.span.to(close);
                    Ok(Type::new(TypeKind::Named { name, arguments }, span))
                } else {
                    Ok(Type::new(
                        TypeKind::Named {
                            name,
                            arguments: Vec::new(),
                        },
                        token.span,
                    ))
                }
            }
            TokenKind::SelfType => {
                self.advance();
                Ok(Type::new(
                    TypeKind::Named {
                        name: "Self".to_string(),
                        arguments: Vec::new(),
                    },
                    token.span,
                ))
            }
            TokenKind::LeftParenthesis => self.parse_grouped_type(),
            TokenKind::LeftBracket => self.parse_array_or_slice_type(),
            _ => Err(self.error_here(format!("expected a type, found {}", token.kind.describe()))),
        }
    }

    fn parse_generic_arguments(&mut self) -> Parse<(Vec<Type>, Span)> {
        self.advance();
        self.skip_newlines();
        let mut arguments = Vec::new();
        loop {
            arguments.push(self.parse_type()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
            if self.at_generic_close() {
                break;
            }
        }
        let close = self.expect_generic_close()?;
        Ok((arguments, close))
    }

    fn at_generic_close(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Greater | TokenKind::ShiftRight
        )
    }

    // The lexer munches `>>` greedily, so `List<List<i32>>` closes with one token split in two.
    fn expect_generic_close(&mut self) -> Parse<Span> {
        match self.current_kind() {
            TokenKind::Greater => Ok(self.advance().span),
            TokenKind::ShiftRight => {
                let span = self.current_span();
                let first = Span::new(span.start, span.start + 1, span.line, span.column);
                self.tokens[self.position].kind = TokenKind::Greater;
                self.tokens[self.position].span =
                    Span::new(span.start + 1, span.end, span.line, span.column + 1);
                Ok(first)
            }
            _ => Err(self.error_here(format!(
                "expected `>`, found {}",
                self.current_kind().describe()
            ))),
        }
    }

    fn parse_grouped_type(&mut self) -> Parse<Type> {
        self.advance();
        self.skip_newlines();
        if self.check(&TokenKind::RightParenthesis) {
            return Err(self.error_here(
                "expected a type; `()` is not a type (x has no tuple or unit literal)".to_string(),
            ));
        }
        let inner = self.parse_type()?;
        self.skip_newlines();
        if self.check(&TokenKind::Comma) {
            return Err(self.error_here(
                "tuples are not supported; group values in a record instead".to_string(),
            ));
        }
        self.expect(&TokenKind::RightParenthesis, "`)`")?;
        Ok(inner)
    }

    fn parse_array_or_slice_type(&mut self) -> Parse<Type> {
        let open = self.advance();
        self.skip_newlines();
        let element = self.parse_type()?;
        self.skip_newlines();
        if self.eat(&TokenKind::Semicolon) {
            self.skip_newlines();
            let size = self.parse_binary(0)?;
            self.skip_newlines();
            let close = self.expect(&TokenKind::RightBracket, "`]`")?;
            let span = open.span.to(close.span);
            Ok(Type::new(
                TypeKind::Array {
                    element: Box::new(element),
                    size: Box::new(size),
                },
                span,
            ))
        } else {
            let close = self.expect(&TokenKind::RightBracket, "`]`")?;
            let span = open.span.to(close.span);
            Ok(Type::new(TypeKind::Slice(Box::new(element)), span))
        }
    }

    fn parse_function_type(&mut self) -> Parse<Type> {
        let start = self.advance();
        self.expect(&TokenKind::LeftParenthesis, "`(`")?;
        self.skip_newlines();
        let mut parameters = Vec::new();
        let mut variadic = false;
        while !self.check(&TokenKind::RightParenthesis) {
            if self.check(&TokenKind::Ellipsis) {
                self.advance();
                variadic = true;
                self.skip_newlines();
                break;
            }
            parameters.push(self.parse_type()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        let close = self.expect(&TokenKind::RightParenthesis, "`)`")?;
        let (result, end) = if self.eat(&TokenKind::Arrow) {
            self.skip_newlines();
            let result = self.parse_type()?;
            let span = result.span;
            (Some(Box::new(result)), span)
        } else {
            (None, close.span)
        };
        let span = start.span.to(end);
        Ok(Type::new(
            TypeKind::Function {
                parameters,
                variadic,
                result,
            },
            span,
        ))
    }

    fn parse_declaration(&mut self) -> Parse<Declaration> {
        match self.current_kind() {
            TokenKind::Fun => self.parse_function(),
            other => Err(self.error_here(format!(
                "expected a declaration, found {}",
                other.describe()
            ))),
        }
    }

    fn parse_function(&mut self) -> Parse<Declaration> {
        let start = self.advance();
        let name = self.expect_identifier("a function name")?;
        self.expect(&TokenKind::LeftParenthesis, "`(` after the function name")?;
        let parameters = self.parse_parameters()?;
        let result = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        if !self.check(&TokenKind::LeftBrace) {
            return Err(self.error_here(format!(
                "expected `{{` to begin the function body, found {}",
                self.current_kind().describe()
            )));
        }
        let body = self.parse_block()?;
        let span = start.span.to(body.span);
        Ok(Declaration {
            kind: DeclarationKind::Function(Function {
                name: identifier_name(name),
                parameters,
                result,
                body,
            }),
            span,
        })
    }

    fn parse_parameters(&mut self) -> Parse<Vec<Parameter>> {
        let mut parameters = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RightParenthesis) {
            parameters.push(self.parse_parameter()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RightParenthesis, "`)` after the parameters")?;
        Ok(parameters)
    }

    fn parse_parameter(&mut self) -> Parse<Parameter> {
        let first = self.expect_identifier("a parameter name")?;
        let start = first.span;
        let first = identifier_name(first);
        let (label, name) = if matches!(self.current_kind(), TokenKind::Identifier(_)) {
            let internal = identifier_name(self.advance());
            let label = if first == "_" { None } else { Some(first) };
            (label, internal)
        } else if first == "_" {
            return Err(self.error_here(format!(
                "expected a parameter name after `_`, found {}",
                self.current_kind().describe()
            )));
        } else {
            (Some(first.clone()), first)
        };
        self.expect(&TokenKind::Colon, "`:` and a parameter type")?;
        let annotation = self.parse_type()?;
        let default = if self.eat(&TokenKind::ColonEqual) {
            Some(self.parse_binary(0)?)
        } else {
            None
        };
        let end = default
            .as_ref()
            .map_or(annotation.span, |default| default.span);
        Ok(Parameter {
            label,
            name,
            annotation,
            default,
            span: start.to(end),
        })
    }

    fn parse_block(&mut self) -> Parse<Expression> {
        let open = self.expect(&TokenKind::LeftBrace, "`{`")?;
        self.skip_newlines();
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.at_end() {
            statements.push(self.parse_statement()?);
            self.expect_statement_end()?;
            self.skip_newlines();
        }
        let close = self.expect(&TokenKind::RightBrace, "`}`")?;

        let value = match statements.last() {
            Some(Statement {
                kind: StatementKind::Expression(_),
                ..
            }) => {
                let Some(Statement {
                    kind: StatementKind::Expression(expression),
                    ..
                }) = statements.pop()
                else {
                    unreachable!()
                };
                Some(Box::new(expression))
            }
            _ => None,
        };

        let span = open.span.to(close.span);
        Ok(Expression::new(
            ExpressionKind::Block { statements, value },
            span,
        ))
    }

    fn parse_unsafe(&mut self) -> Parse<Expression> {
        let start = self.advance();
        let block = self.parse_block()?;
        let span = start.span.to(block.span);
        Ok(Expression::new(
            ExpressionKind::Unsafe(Box::new(block)),
            span,
        ))
    }

    fn expect_statement_end(&mut self) -> Parse<()> {
        match self.current_kind() {
            TokenKind::Newline => {
                self.advance();
                Ok(())
            }
            TokenKind::RightBrace | TokenKind::Eof => Ok(()),
            other => Err(self.error_here(format!(
                "expected a newline to end the statement, found {}",
                other.describe()
            ))),
        }
    }

    fn parse_statement(&mut self) -> Parse<Statement> {
        match self.current_kind() {
            TokenKind::Let => self.parse_let(),
            TokenKind::Return => self.parse_return(),
            _ => self.parse_expression_or_assignment(),
        }
    }

    fn parse_let(&mut self) -> Parse<Statement> {
        let start = self.advance();
        let mutable = self.eat(&TokenKind::Mut);
        let name_token = self.expect_identifier("a binding name")?;
        let TokenKind::Identifier(name) = name_token.kind else {
            unreachable!()
        };
        let annotation = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::ColonEqual, "`:=`")?;
        self.skip_newlines();
        let value = self.parse_binary(0)?;
        let span = start.span.to(value.span);
        Ok(Statement::new(
            StatementKind::Let {
                mutable,
                name,
                annotation,
                value,
            },
            span,
        ))
    }

    fn parse_return(&mut self) -> Parse<Statement> {
        let keyword = self.advance();
        let value = match self.current_kind() {
            TokenKind::Newline | TokenKind::RightBrace | TokenKind::Eof => None,
            _ => Some(self.parse_binary(0)?),
        };
        let span = match &value {
            Some(value) => keyword.span.to(value.span),
            None => keyword.span,
        };
        Ok(Statement::new(StatementKind::Return(value), span))
    }

    fn parse_expression_or_assignment(&mut self) -> Parse<Statement> {
        let target = self.parse_binary(0)?;
        let Some(operator) = assignment_operator(self.current_kind()) else {
            let span = target.span;
            return Ok(Statement::new(StatementKind::Expression(target), span));
        };
        self.advance();
        self.skip_newlines();
        let value = self.parse_binary(0)?;
        let span = target.span.to(value.span);
        Ok(Statement::new(
            StatementKind::Assignment {
                operator,
                target,
                value,
            },
            span,
        ))
    }
}

fn binary_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    use BinaryOperator as Operator;
    Some(match kind {
        TokenKind::Plus => Operator::Add,
        TokenKind::Minus => Operator::Subtract,
        TokenKind::Star => Operator::Multiply,
        TokenKind::Slash => Operator::Divide,
        TokenKind::Percent => Operator::Modulo,
        TokenKind::ShiftLeft => Operator::ShiftLeft,
        TokenKind::ShiftRight => Operator::ShiftRight,
        TokenKind::AmpersandAmpersand => Operator::BitwiseAnd,
        TokenKind::CaretCaret => Operator::BitwiseXor,
        TokenKind::PipePipe => Operator::BitwiseOr,
        TokenKind::Less => Operator::Less,
        TokenKind::LessEqual => Operator::LessEqual,
        TokenKind::Greater => Operator::Greater,
        TokenKind::GreaterEqual => Operator::GreaterEqual,
        TokenKind::Equal => Operator::Equal,
        TokenKind::NotEqual => Operator::NotEqual,
        TokenKind::Ampersand => Operator::BooleanAnd,
        TokenKind::Caret => Operator::BooleanXor,
        TokenKind::Pipe => Operator::BooleanOr,
        _ => return None,
    })
}

fn assignment_operator(kind: &TokenKind) -> Option<AssignmentOperator> {
    use AssignmentOperator as Operator;
    Some(match kind {
        TokenKind::ColonEqual => Operator::Assign,
        TokenKind::PlusEqual => Operator::Add,
        TokenKind::MinusEqual => Operator::Subtract,
        TokenKind::StarEqual => Operator::Multiply,
        TokenKind::SlashEqual => Operator::Divide,
        TokenKind::PercentEqual => Operator::Modulo,
        TokenKind::AmpersandEqual => Operator::BooleanAnd,
        TokenKind::PipeEqual => Operator::BooleanOr,
        TokenKind::CaretEqual => Operator::BooleanXor,
        TokenKind::AmpersandAmpersandEqual => Operator::BitwiseAnd,
        TokenKind::PipePipeEqual => Operator::BitwiseOr,
        TokenKind::CaretCaretEqual => Operator::BitwiseXor,
        TokenKind::ShiftLeftEqual => Operator::ShiftLeft,
        TokenKind::ShiftRightEqual => Operator::ShiftRight,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
