use crate::lexer::Lexer;
use crate::syntax::*;
use crate::token::*;

use std::iter::Peekable;
use std::str::Chars;

pub struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub fn new(lexer: Lexer) -> Parser {
        Parser { lexer }
    }

    pub fn parse(&self, input: &mut Peekable<Chars>) -> SourceFile {
        SourceFile {
            code_block: self.parse_code_block(input),
        }
    }

    fn parse_code_block_container(
        &self,
        input: &mut Peekable<Chars>,
    ) -> Option<CodeBlockContainer> {
        println!("parsing code block container");
        let mut token = self.lexer.next_token(input, true)?;

        input.peek();
        if token != Token::LeftBrace {
            return None;
        }

        let code_block = self.parse_code_block(input);

        token = self.lexer.next_token(input, true)?;

        if token != Token::RightBrace {
            return None;
        }

        Some(CodeBlockContainer { code_block })
    }

    fn parse_code_block(&self, input: &mut Peekable<Chars>) -> CodeBlock {
        println!("parsing code block");
        let mut items = Vec::new();

        loop {
            let token = self.lexer.peek_token(input, true);

            if let Some(token) = token {
                if token == Token::RightBrace {
                    break;
                }

                if let Some(item) = self.parse_code_block_item(input) {
                    items.push(item);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        CodeBlock { items }
    }

    fn parse_code_block_item(&self, input: &mut Peekable<Chars>) -> Option<CodeBlockItem> {
        println!("parsing code block item");
        let token = self.lexer.peek_token(input, true);

        if let Some(token) = token {
            match token {
                Token::Keyword(Keyword::Let)
                | Token::Keyword(Keyword::Fun)
                | Token::Keyword(Keyword::Extern)
                | Token::Keyword(Keyword::Enum)
                | Token::Keyword(Keyword::Type) => {
                    let declaration = self.parse_declaration(input)?;
                    return Some(CodeBlockItem::Declaration(declaration));
                }
                Token::Keyword(Keyword::Loop)
                | Token::Keyword(Keyword::Return)
                | Token::Keyword(Keyword::Break)
                | Token::Keyword(Keyword::Continue) => {
                    let statement = self.parse_statement(input)?;
                    return Some(CodeBlockItem::Statement(statement));
                }
                _ => {
                    let expression = self.parse_expression(input)?;
                    return Some(CodeBlockItem::Expression(expression));
                }
            }
        } else {
            None
        }
    }

    fn parse_statement(&self, input: &mut Peekable<Chars>) -> Option<Statement> {
        println!("parsing statement");
        None
    }

    fn parse_declaration(&self, input: &mut Peekable<Chars>) -> Option<Declaration> {
        println!("parsing declaration");
        let token = self.lexer.next_token(input, true)?;

        match token {
            Token::Keyword(Keyword::Let) => {
                let variable = self.parse_let_declaration(input)?;
                return Some(Declaration::VariableDeclaration(variable));
            }
            Token::Keyword(Keyword::Fun) => {
                let function = self.parse_fun_declaration(input)?;
                return Some(Declaration::FunctionDeclaration(function));
            }
            Token::Keyword(Keyword::Extern) => {
                let ext = self.parse_extern_declaration(input)?;
                return Some(Declaration::ExternDeclaration(ext));
            }
            Token::Keyword(Keyword::Enum) => {
                let ext = self.parse_enum_declaration(input)?;
                return Some(Declaration::EnumDeclaration(ext));
            }
            Token::Keyword(Keyword::Type) => {
                let ext = self.parse_type_declaration(input)?;
                return Some(Declaration::TypeDeclaration(ext));
            }
            _ => None,
        }
    }

    fn parse_enum_declaration(&self, input: &mut Peekable<Chars>) -> Option<EnumDeclaration> {
        let token = self.lexer.next_token(input, true)?;

        Some(EnumDeclaration {
            name: self.parse_identifier(token)?,
            container: self.parse_member_block_container(input)?,
        })
    }

    fn parse_type_declaration(&self, input: &mut Peekable<Chars>) -> Option<TypeDeclaration> {
        let token = self.lexer.next_token(input, true)?;

        Some(TypeDeclaration {
            name: self.parse_identifier(token)?,
            container: self.parse_member_block_container(input)?,
        })
    }

    fn parse_member_block_container(
        &self,
        input: &mut Peekable<Chars>,
    ) -> Option<MemberBlockContainer> {
        let mut token = self.lexer.peek_token(input, true)?;

        if token != Token::LeftBrace {
            return None;
        }

        self.lexer.next_token(input, true)?;

        let member_block = self.parse_member_block(input);

        token = self.lexer.peek_token(input, true)?;

        if token != Token::RightBrace {
            return None;
        }

        self.lexer.next_token(input, true);

        Some(MemberBlockContainer { member_block })
    }

    fn parse_member_block(&self, input: &mut Peekable<Chars>) -> MemberBlock {
        let mut members = Vec::new();

        loop {
            let Some(token) = self.lexer.peek_token(input, true) else {
                break;
            };

            if token == Token::RightBrace {
                break;
            }

            self.lexer.next_token(input, true);

            let Some(member) = self.parse_member_item(input) else {
                break;
            };

            members.push(member);
        }

        MemberBlock { members }
    }

    fn parse_member_item(&self, input: &mut Peekable<Chars>) -> Option<MemberBlockItem> {
        let token = self.lexer.next_token(input, true)?;

        match token {
            // TODO: add case for "case"
            Token::Keyword(Keyword::Let) => {
                let variable = self.parse_let_declaration(input)?;
                return Some(MemberBlockItem::VariableDeclaration(variable));
            }
            Token::Keyword(Keyword::Fun) => {
                // TODO: add cases for init
                let function = self.parse_fun_declaration(input)?;
                return Some(MemberBlockItem::FunctionDeclaration(function));
            }
            // Token::Keyword(Keyword::Enum) => {
            //     let enumeration = self.parse_enum_declaration(input)?;
            //     return Some(MemberBlockItem::EnumDeclaration(enumeration));
            // }
            // Token::Keyword(Keyword::Type) => {
            //     let _type = self.parse_type_declaration(input)?;
            //     return Some(MemberBlockItem::TypeDeclaration(_type));
            // }
            _ => None,
        }
    }

    fn parse_let_declaration(&self, input: &mut Peekable<Chars>) -> Option<VariableDeclaration> {
        println!("parsing let declaration");
        let mut token = self.lexer.next_token(input, true)?;
        let identifier = self.parse_identifier(token)?;

        token = self.lexer.peek_token(input, true)?;
        if token != Token::Assign {
            return None;
        }

        self.lexer.next_token(input, true)?;

        let expression = self.parse_expression(input)?;

        Some(VariableDeclaration {
            identifier,
            expression,
        })
    }

    fn parse_fun_declaration(&self, input: &mut Peekable<Chars>) -> Option<FunctionDeclaration> {
        println!("parsing fun declaration");
        let token = self.lexer.next_token(input, true)?;
        let identifier = self.parse_identifier(token)?;

        let signature = self.parse_function_signature(input)?;
        let body = self.parse_code_block_container(input)?;

        Some(FunctionDeclaration {
            identifier,
            signature,
            body,
        })
    }

    fn parse_extern_declaration(&self, input: &mut Peekable<Chars>) -> Option<ExternDeclaration> {
        let mut token = self.lexer.next_token(input, true)?;

        if token != Token::Keyword(Keyword::Fun) {
            return None;
        }
        token = self.lexer.next_token(input, true)?;

        let identifier = self.parse_identifier(token)?;
        let signature = self.parse_function_signature(input)?;

        Some(ExternDeclaration {
            identifier,
            signature,
        })
    }

    fn parse_function_signature(&self, input: &mut Peekable<Chars>) -> Option<FunctionSignature> {
        println!("parsing function signature");
        let parameters = self.parse_function_parameters(input)?;
        let return_clause = self.parse_return_clause(input);

        Some(FunctionSignature {
            parameters,
            return_clause,
        })
    }

    fn parse_function_parameters(&self, input: &mut Peekable<Chars>) -> Option<FunctionParameters> {
        println!("parsing function parameters");
        let mut token = self.lexer.next_token(input, true)?;

        if token != Token::LeftParen {
            return None;
        }

        let mut parameters = Vec::new();

        loop {
            token = self.lexer.peek_token(input, true)?;

            if token == Token::RightParen {
                self.lexer.next_token(input, true);
                break;
            }

            if let Some(parameter) = self.parse_function_parameter(input) {
                parameters.push(parameter);
            }

            token = self.lexer.peek_token(input, true)?;

            if token == Token::Comma {
                self.lexer.next_token(input, true);
            }
        }

        Some(FunctionParameters { parameters })
    }

    fn parse_function_parameter(&self, input: &mut Peekable<Chars>) -> Option<FunctionParameter> {
        println!("parsing function parameter");
        let mut token = self.lexer.next_token(input, true)?;
        let name = self.parse_identifier(token)?; // TODO: add label support

        token = self.lexer.peek_token(input, true)?;

        if token != Token::Colon {
            return None;
        }

        self.lexer.next_token(input, true);

        let parameter_type = self.parse_type(input)?;

        Some(FunctionParameter {
            label: None,
            name,
            parameter_type,
        })
    }

    fn parse_return_clause(&self, input: &mut Peekable<Chars>) -> Option<ReturnClause> {
        println!("parsing return clause");
        let token = self.lexer.peek_token(input, true)?;

        if token != Token::Arrow {
            return None;
        }

        self.lexer.next_token(input, true);
        let return_type = self.parse_type(input)?;

        Some(ReturnClause { return_type })
    }

    fn parse_type(&self, input: &mut Peekable<Chars>) -> Option<TypeSyntax> {
        println!("parsing type");
        let token = self.lexer.next_token(input, true)?;

        match token {
            Token::Identifier(name) => Some(TypeSyntax::IdentifierType(Identifier { name })),
            _ => None,
        }
    }

    fn parse_expression(&self, input: &mut Peekable<Chars>) -> Option<Expression> {
        println!("parsing expression");

        let left = self.parse_primary_expression(input)?;
        self.parse_binary_operation(input, left, 1)
    }

    fn parse_primary_expression(&self, input: &mut Peekable<Chars>) -> Option<Expression> {
        let token = self.lexer.next_token(input, true)?;

        println!("{:?}", token);

        match token {
            Token::Identifier(name) => {
                let Some(token) = self.lexer.peek_token(input, true) else {
                    return Some(Expression::Identifier(Identifier { name }));
                };

                if token == Token::LeftParen {
                    self.lexer.next_token(input, true);
                    let arguments = self.parse_tuple(input)?;

                    return Some(Expression::FunctionCall(FunctionCallExpression {
                        function: Identifier { name },
                        arguments,
                    }));
                }

                Some(Expression::Identifier(Identifier { name }))
            }
            Token::Number(value) => Some(Expression::FloatNumberLiteral(value)),
            Token::Keyword(Keyword::True) => Some(Expression::BooleanLiteral(true)),
            Token::Keyword(Keyword::False) => Some(Expression::BooleanLiteral(false)),
            Token::Keyword(Keyword::If) => self
                .parse_if_expression(input)
                .map(|if_expr| Expression::If(if_expr)),
            Token::Keyword(Keyword::Nil) => Some(Expression::NilLiteral),
            Token::LeftParen => self
                .parse_tuple(input)
                .map(|tuple| Expression::Tuple(tuple)),
            _ => None,
        }
    }

    fn parse_if_expression(&self, input: &mut Peekable<Chars>) -> Option<IfExpression> {
        let condition = self.parse_expression(input)?;
        let then_expression = self.parse_expression_container(input)?;

        let Some(token) = self.lexer.peek_token(input, true) else {
            return None;
        };

        if token != Token::Keyword(Keyword::Else) {
            return None;
        }

        self.lexer.next_token(input, true);

        let else_expression = self.parse_expression_container(input)?;

        Some(IfExpression {
            condition: Box::new(condition),
            then_expression: Box::new(then_expression),
            else_expression: Box::new(else_expression),
        })
    }

    fn parse_expression_container(&self, input: &mut Peekable<Chars>) -> Option<Expression> {
        let Some(token) = self.lexer.peek_token(input, true) else {
            return None;
        };

        if token != Token::LeftBrace {
            return None;
        }

        self.lexer.next_token(input, true);
        let expression = self.parse_expression(input)?;

        let Some(token) = self.lexer.peek_token(input, true) else {
            return None;
        };

        if token != Token::RightBrace {
            return None;
        }

        self.lexer.next_token(input, true);

        return Some(expression);
    }

    fn parse_binary_operation(
        &self,
        input: &mut Peekable<Chars>,
        left: Expression,
        precedence: u32,
    ) -> Option<Expression> {
        let mut new_left = left;

        println!("parsing binary expression");

        loop {
            let Some(token) = self.lexer.peek_token(input, true) else {
                return Some(new_left);
            };

            let Some(operator) = BinaryOperator::from_token(&token) else {
                return Some(new_left);
            };

            self.lexer.next_token(input, true);

            dbg!(&token);

            let operator_precedence = operator.precedence();

            if operator_precedence < precedence {
                return Some(new_left);
            }

            let mut right = self.parse_primary_expression(input)?;

            if let Some(next_token) = self.lexer.peek_token(input, true) {
                if let Some(next_operator) = BinaryOperator::from_token(&next_token) {
                    let next_operator_precedence = next_operator.precedence();

                    if operator_precedence < next_operator_precedence {
                        let Some(new_right) =
                            self.parse_binary_operation(input, right, operator_precedence + 1)
                        else {
                            return None;
                        };

                        right = new_right;
                    }
                }
            }

            println!("setting a new left");
            new_left = Expression::BinaryOperator(BinaryOperatorExpression {
                operator,
                left: Box::new(new_left),
                right: Box::new(right),
            });
        }
    }

    fn parse_tuple(&self, input: &mut Peekable<Chars>) -> Option<TupleExpression> {
        Some(TupleExpression {
            expressions: self.parse_expression_list(input),
        })
    }

    fn parse_expression_list(&self, input: &mut Peekable<Chars>) -> ExpressionList {
        let mut expressions = Vec::new();

        loop {
            let expression = self.parse_expression(input);

            if let Some(expression) = expression {
                expressions.push(expression);
            }

            let Some(token) = self.lexer.peek_token(input, true) else {
                break;
            };

            match token {
                Token::Comma => {
                    self.lexer.next_token(input, true);
                }
                Token::RightParen => {
                    self.lexer.next_token(input, true);
                    break;
                }
                _ => break, // TODO: probably error this out
            }
        }

        ExpressionList { items: expressions }
    }

    fn parse_identifier(&self, token: Token) -> Option<Identifier> {
        println!("parsing identifier: {:?}", token);

        if let Token::Identifier(name) = token {
            Some(Identifier { name })
        } else {
            None
        }
    }
}
