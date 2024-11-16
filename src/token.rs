#[derive(Debug, PartialEq)]
pub enum Keyword {
    Let,
    Fun,
    If,
    Else,
    Loop,
    Match,
    Extern,
}

#[derive(Debug, PartialEq)]
pub enum Token {
    Keyword(Keyword),
    Identifier(String),
    Number(f64),
    Wildcard,
    Newline,
    Arrow,
    Colon,
    Comma,
    Dot,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Tilde,
    Equal,
    NotEqual,
    Assign,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
}
