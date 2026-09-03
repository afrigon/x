use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, column: u32) -> Self {
        Span {
            start,
            end,
            line,
            column,
        }
    }

    pub fn to(self, end: Span) -> Span {
        Span {
            start: self.start,
            end: end.end,
            line: self.line,
            column: self.column,
        }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}..{}", self.line, self.start, self.end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radix {
    Decimal,
    Hexadecimal,
    Octal,
    Binary,
}

impl Radix {
    pub fn base(self) -> u32 {
        match self {
            Radix::Decimal => 10,
            Radix::Hexadecimal => 16,
            Radix::Octal => 8,
            Radix::Binary => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Integer { value: u128, radix: Radix },
    Float(String),
    String(String),
    ByteString(Vec<u8>),
    CString(Vec<u8>),
    Character(char),

    Identifier(String),
    DocumentationComment(String),

    Let,
    Mut,
    Fun,
    Type,
    Proto,
    Static,
    Import,
    If,
    Else,
    Match,
    Guard,
    Loop,
    Until,
    In,
    Break,
    Continue,
    Return,
    As,
    SelfValue,
    SelfType,
    True,
    False,
    Private,
    Unsafe,
    Move,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,

    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    Ampersand,
    Pipe,
    Bang,
    Caret,
    AmpersandAmpersand,
    PipePipe,
    BangBang,
    CaretCaret,
    ShiftLeft,
    ShiftRight,

    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    AmpersandAmpersandEqual,
    PipePipeEqual,
    CaretCaretEqual,
    ShiftLeftEqual,
    ShiftRightEqual,

    ColonEqual,

    Question,
    QuestionDot,
    QuestionQuestion,

    Arrow,

    DotDot,
    DotDotEqual,
    Ellipsis,

    Dot,
    Comma,
    Colon,
    Semicolon,
    At,
    Hash,

    LeftParenthesis,
    RightParenthesis,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,

    Newline,
    Eof,
}

impl TokenKind {
    pub fn keyword_from(identifier: &str) -> Option<TokenKind> {
        use TokenKind::*;
        Some(match identifier {
            "let" => Let,
            "mut" => Mut,
            "fun" => Fun,
            "type" => Type,
            "proto" => Proto,
            "static" => Static,
            "import" => Import,
            "if" => If,
            "else" => Else,
            "match" => Match,
            "guard" => Guard,
            "loop" => Loop,
            "until" => Until,
            "in" => In,
            "break" => Break,
            "continue" => Continue,
            "return" => Return,
            "as" => As,
            "self" => SelfValue,
            "Self" => SelfType,
            "true" => True,
            "false" => False,
            "private" => Private,
            "unsafe" => Unsafe,
            "move" => Move,
            _ => return None,
        })
    }

    pub fn describe(&self) -> &'static str {
        use TokenKind::*;
        match self {
            Integer { .. } => "integer",
            Float(_) => "float",
            String(_) => "string",
            ByteString(_) => "byte-string",
            CString(_) => "c-string",
            Character(_) => "character",
            Identifier(_) => "identifier",
            DocumentationComment(_) => "doc-comment",
            Let => "let",
            Mut => "mut",
            Fun => "fun",
            Type => "type",
            Proto => "proto",
            Static => "static",
            Import => "import",
            If => "if",
            Else => "else",
            Match => "match",
            Guard => "guard",
            Loop => "loop",
            Until => "until",
            In => "in",
            Break => "break",
            Continue => "continue",
            Return => "return",
            As => "as",
            SelfValue => "self",
            SelfType => "Self",
            True => "true",
            False => "false",
            Private => "private",
            Unsafe => "unsafe",
            Move => "move",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Percent => "%",
            PlusEqual => "+=",
            MinusEqual => "-=",
            StarEqual => "*=",
            SlashEqual => "/=",
            PercentEqual => "%=",
            Equal => "=",
            NotEqual => "!=",
            Less => "<",
            LessEqual => "<=",
            Greater => ">",
            GreaterEqual => ">=",
            Ampersand => "&",
            Pipe => "|",
            Bang => "!",
            Caret => "^",
            AmpersandAmpersand => "&&",
            PipePipe => "||",
            BangBang => "!!",
            CaretCaret => "^^",
            ShiftLeft => "<<",
            ShiftRight => ">>",
            AmpersandEqual => "&=",
            PipeEqual => "|=",
            CaretEqual => "^=",
            AmpersandAmpersandEqual => "&&=",
            PipePipeEqual => "||=",
            CaretCaretEqual => "^^=",
            ShiftLeftEqual => "<<=",
            ShiftRightEqual => ">>=",
            ColonEqual => ":=",
            Question => "?",
            QuestionDot => "?.",
            QuestionQuestion => "??",
            Arrow => "->",
            DotDot => "..",
            DotDotEqual => "..=",
            Ellipsis => "...",
            Dot => ".",
            Comma => ",",
            Colon => ":",
            Semicolon => ";",
            At => "@",
            Hash => "#",
            LeftParenthesis => "(",
            RightParenthesis => ")",
            LeftBrace => "{",
            RightBrace => "}",
            LeftBracket => "[",
            RightBracket => "]",
            Newline => "newline",
            Eof => "eof",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TokenKind::*;
        match &self.kind {
            Integer { value, radix } => write!(formatter, "Integer({value}, {radix:?})"),
            Float(text) => write!(formatter, "Float({text})"),
            String(text) => write!(formatter, "String({text:?})"),
            ByteString(bytes) => write!(formatter, "ByteString({bytes:?})"),
            CString(bytes) => write!(formatter, "CString({bytes:?})"),
            Character(character) => write!(formatter, "Character({character:?})"),
            Identifier(name) => write!(formatter, "Identifier({name})"),
            DocumentationComment(text) => write!(formatter, "Doc({text:?})"),
            other => write!(formatter, "{}", other.describe()),
        }
    }
}
