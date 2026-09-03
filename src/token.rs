//! Token definitions for the `x` lexer.
//!
//! A [`Token`] pairs a [`TokenKind`] with the source [`Span`] it covers.
//! Keywords each get their own `TokenKind` variant (rather than being folded
//! into `Identifier`) so the parser can match on them directly.
//!
//! The stream is always terminated by a single [`TokenKind::Eof`] with a
//! zero-length span at end of input, so the parser's `peek` can return a real
//! `Token` rather than an `Option` and produce spanned "unexpected end of
//! file" diagnostics for free.

use std::fmt;

/// A byte range in the source, plus the 1-based line/column of its start.
///
/// `start`/`end` are byte offsets into the original UTF-8 source (`end`
/// exclusive). `line`/`column` describe the first character and exist purely
/// for diagnostics — `column` counts Unicode scalar values, not bytes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, column: u32) -> Self {
        Span { start, end, line, column }
    }

    /// A span covering from the start of `self` to the end of `end`. Used to
    /// give a composite AST node the full extent of its children.
    pub fn to(self, end: Span) -> Span {
        Span { start: self.start, end: end.end, line: self.line, column: self.column }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}..{}", self.line, self.start, self.end)
    }
}

/// The base a numeric integer literal was written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radix {
    Decimal,
    Hexadecimal,
    Octal,
    Binary,
}

impl Radix {
    /// The numeric base (e.g. 16 for hexadecimal).
    pub fn base(self) -> u32 {
        match self {
            Radix::Decimal => 10,
            Radix::Hexadecimal => 16,
            Radix::Octal => 8,
            Radix::Binary => 2,
        }
    }
}

/// The lexical category of a token.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // ---- Literals -------------------------------------------------------
    /// Integer literal. Value is parsed with underscores stripped; `radix`
    /// records how it was written (for round-tripping / diagnostics).
    Integer { value: u128, radix: Radix },
    /// Floating-point literal, stored as the cleaned source text (underscores
    /// removed). Kept as text to avoid an early lossy `f64` round-trip.
    Float(String),
    /// String literal with escapes already decoded.
    String(String),
    /// Byte string literal `b"..."` — decoded bytes (ASCII source plus byte
    /// escapes; no Unicode escapes).
    ByteString(Vec<u8>),
    /// C string literal `c"..."` — decoded UTF-8 bytes. The terminating NUL is
    /// implicit (not stored), and no interior NUL is permitted.
    CString(Vec<u8>),
    /// Character literal with its escape already decoded.
    Character(char),

    // ---- Identifiers & comments ----------------------------------------
    /// An identifier (also covers a lone `_`).
    Identifier(String),
    /// A `///` documentation comment, body trimmed of the leading marker/space.
    DocumentationComment(String),

    // ---- Keywords -------------------------------------------------------
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
    SelfValue, // `self`
    SelfType,  // `Self`
    True,
    False,
    Private,
    Unsafe,
    Move,

    // ---- Operators & punctuation ---------------------------------------
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Percent,      // %
    PlusEqual,    // +=
    MinusEqual,   // -=
    StarEqual,    // *=
    SlashEqual,   // /=
    PercentEqual, // %=

    Equal,        // =   (equality; binding/mutation is `:=`)
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    Ampersand,           // &   boolean AND / reference-of
    Pipe,                // |   boolean OR
    Bang,                // !   boolean NOT
    Caret,               // ^   boolean XOR
    AmpersandAmpersand,  // &&  bitwise AND
    PipePipe,            // ||  bitwise OR
    BangBang,            // !!  bitwise NOT
    CaretCaret,          // ^^  bitwise XOR
    ShiftLeft,           // <<
    ShiftRight,          // >>

    AmpersandEqual,           // &=
    PipeEqual,                // |=
    CaretEqual,               // ^=
    AmpersandAmpersandEqual,  // &&=
    PipePipeEqual,            // ||=
    CaretCaretEqual,          // ^^=
    ShiftLeftEqual,           // <<=
    ShiftRightEqual,          // >>=

    ColonEqual, // :=

    Question,         // ?
    QuestionDot,      // ?.
    QuestionQuestion, // ??

    Arrow, // ->

    DotDot,      // ..
    DotDotEqual, // ..=
    Ellipsis,    // ...

    Dot,       // .
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;   (only meaningful as the array-size separator in `[T; N]`)
    At,        // @
    Hash,      // #

    LeftParenthesis,  // (
    RightParenthesis, // )
    LeftBrace,        // {
    RightBrace,       // }
    LeftBracket,      // [
    RightBracket,     // ]

    // ---- Trivia / structure --------------------------------------------
    /// A statement-terminating newline. Consecutive blank lines coalesce into
    /// a single `Newline`; leading newlines are not emitted.
    Newline,
    /// End of input. Always the final token, with a zero-length span.
    Eof,
}

impl TokenKind {
    /// Maps an identifier string to its keyword `TokenKind`, if it is one.
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

    /// A short human-readable name for diagnostics and token dumps.
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

/// A lexed token: its kind plus the source span it covers.
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
