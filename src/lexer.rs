use crate::token::{Keyword, Token};

use std::iter::Peekable;
use std::str::Chars;

static TEST: u32 = 10;
const test: u32 = 10;

pub struct Lexer {}

impl Lexer {
    pub fn new() -> Lexer {
        Lexer {}
    }

    pub fn peek_token(&self, input: &Peekable<Chars>, ignore_new_line: bool) -> Option<Token> {
        // probably a bad idea to copy the entire input
        // should be using a peekable of tokens instead
        let mut input_clone = input.clone();
        self.next_token(&mut input_clone, ignore_new_line)
    }

    pub fn next_token(&self, input: &mut Peekable<Chars>, ignore_new_line: bool) -> Option<Token> {
        while let Some(c) = input.next() {
            if c.is_whitespace() && (c != '\n' || ignore_new_line) {
                continue;
            }

            match c {
                '\n' => return Some(Token::Newline),
                '(' => return Some(Token::LeftParen),
                ')' => return Some(Token::RightParen),
                '{' => return Some(Token::LeftBrace),
                '}' => return Some(Token::RightBrace),
                '[' => return Some(Token::LeftBracket),
                ']' => return Some(Token::RightBracket),
                ',' => return Some(Token::Comma),
                '.' => return Some(Token::Dot),
                '+' => return Some(Token::Plus),
                '-' => {
                    if input.next_if(|c| *c == '>').is_some() {
                        return Some(Token::Arrow);
                    }

                    return Some(Token::Minus);
                }
                '*' => return Some(Token::Asterisk),
                ':' => {
                    if input.next_if(|c| *c == '=').is_some() {
                        return Some(Token::Assign);
                    }

                    return Some(Token::Colon);
                }
                '=' => return Some(Token::Equal),
                '~' => {
                    if input.next_if(|c| *c == '=').is_some() {
                        return Some(Token::NotEqual);
                    }

                    return Some(Token::Tilde);
                }
                '/' => {
                    if input.next_if(|c| *c == '/').is_some() {
                        while let Some(comment_character) = input.next() {
                            if comment_character == '\n' || comment_character == '\r' {
                                break;
                            }
                        }

                        continue;
                    }

                    return Some(Token::Slash);
                }
                _ => {
                    if c == '_' && input.peek().map_or(true, |c| !c.is_alphabetic()) {
                        return Some(Token::Wildcard);
                    }

                    if c.is_alphabetic() || c == '_' {
                        let identifier = self.next_identifier(input, c);

                        return match identifier.as_str() {
                            "let" => Some(Token::Keyword(Keyword::Let)),
                            "fun" => Some(Token::Keyword(Keyword::Fun)),
                            "if" => Some(Token::Keyword(Keyword::If)),
                            "else" => Some(Token::Keyword(Keyword::Else)),
                            "loop" => Some(Token::Keyword(Keyword::Loop)),
                            "match" => Some(Token::Keyword(Keyword::Match)),
                            "extern" => Some(Token::Keyword(Keyword::Extern)),
                            "true" => Some(Token::Keyword(Keyword::True)),
                            "false" => Some(Token::Keyword(Keyword::False)),
                            "nil" => Some(Token::Keyword(Keyword::Nil)),
                            "enum" => Some(Token::Keyword(Keyword::Enum)),
                            "type" => Some(Token::Keyword(Keyword::Type)),
                            "proto" => Some(Token::Keyword(Keyword::Proto)),
                            "return" => Some(Token::Keyword(Keyword::Return)),
                            "break" => Some(Token::Keyword(Keyword::Break)),
                            "continue" => Some(Token::Keyword(Keyword::Continue)),
                            _ => Some(Token::Identifier(identifier)),
                        };
                    }

                    if c.is_numeric() {
                        return Some(Token::Number(self.next_number(input, c)));
                    }

                    continue;
                }
            }
        }

        return None;
    }

    fn next_identifier(&self, input: &mut Peekable<Chars>, first: char) -> String {
        let mut identifier = String::new();
        identifier.push(first);

        while let Some(c) = input.next_if(|c| c.is_alphanumeric() || *c == '_') {
            identifier.push(c);
        }

        return identifier;
    }

    fn next_number(&self, input: &mut Peekable<Chars>, first: char) -> f64 {
        let mut number = String::new();
        number.push(first);

        while let Some(c) = input.next_if(|c| c.is_numeric() || *c == '.') {
            number.push(c);
        }

        return number.parse().unwrap_or(0.0); // TODO: maybe error instead of 0
    }
}
