//! Hand-written lexer for `x`.
//!
//! Turns UTF-8 source into a flat [`Token`] stream with source spans. The
//! lexer is error-recovering: it never aborts on a bad character, it records a
//! [`LexError`] and keeps going, so a single file can surface many errors at
//! once. [`tokenize`] returns both the tokens it managed to produce and every
//! error it hit.
//!
//! Notable behaviours:
//! - Newlines are significant (they terminate statements). Consecutive blank
//!   lines coalesce into a single [`TokenKind::Newline`], and leading newlines
//!   are dropped.
//! - `//` line comments are skipped; `///` doc comments are kept as tokens.
//! - String/character escapes follow a provisional grammar (the spec leaves
//!   the exact rules open) covering `\n \r \t \0 \\ \" \'`, `\xHH`, `\u{...}`.

use crate::token::{Radix, Span, Token, TokenKind};
use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

/// What went wrong at a particular [`Span`] during lexing.
#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LexErrorKind {
    /// A character that cannot begin any token.
    UnexpectedCharacter(char),
    /// A string literal with no closing quote before end of line/input.
    UnterminatedString,
    /// A character literal with no closing quote.
    UnterminatedCharacter,
    /// An empty character literal `''`.
    EmptyCharacter,
    /// A character literal containing more than one character.
    MultipleCharacters,
    /// A backslash escape that isn't recognised.
    InvalidEscape(char),
    /// A `\xHH` escape above 0x7F in a context that only allows ASCII
    /// (`"..."` / `'...'`); use `\u{...}`, or a byte/C string for raw bytes.
    AsciiEscapeOutOfRange,
    /// A `\u{...}` escape in a byte string, where it is not allowed.
    UnicodeEscapeNotAllowed,
    /// A non-ASCII character in a byte string literal (only ASCII is allowed;
    /// use `\xHH` for raw bytes).
    NonAsciiByteString,
    /// A NUL (`\0`, `\x00`, `\u{0}`, or a raw NUL) inside a C string, which is
    /// implicitly NUL-terminated and may not contain an interior NUL.
    NulInCString,
    /// A malformed `\u{...}` escape.
    InvalidUnicodeEscape,
    /// An integer literal that doesn't fit in the lexer's `u128` accumulator.
    IntegerOverflow,
    /// A numeric literal missing required digits (e.g. `0x`, `1e`).
    MissingDigits,
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LexErrorKind::*;
        let message = match &self.kind {
            UnexpectedCharacter(c) => format!("unexpected character {c:?}"),
            UnterminatedString => "unterminated string literal".to_string(),
            UnterminatedCharacter => "unterminated character literal".to_string(),
            EmptyCharacter => "empty character literal".to_string(),
            MultipleCharacters => "character literal has more than one character".to_string(),
            InvalidEscape(c) => format!("invalid escape sequence '\\{c}'"),
            AsciiEscapeOutOfRange => {
                "'\\x' escape must be in 0x00..=0x7F; use '\\u{...}' for higher".to_string()
            }
            UnicodeEscapeNotAllowed => "'\\u{...}' is not allowed in a byte string".to_string(),
            NonAsciiByteString => {
                "byte string literals allow ASCII only; use '\\xHH' for other bytes".to_string()
            }
            NulInCString => "C string literals may not contain a NUL".to_string(),
            InvalidUnicodeEscape => "invalid unicode escape".to_string(),
            IntegerOverflow => "integer literal is too large".to_string(),
            MissingDigits => "numeric literal is missing digits".to_string(),
        };
        write!(
            formatter,
            "line {}, column {}: {}",
            self.span.line, self.span.column, message
        )
    }
}

/// A saved cursor position, used to span a token from where it started to the
/// current position once fully consumed.
#[derive(Clone, Copy)]
struct Mark {
    byte: usize,
    line: u32,
    column: u32,
}

/// Which quoted string flavor a `b`/`c` prefix introduces.
#[derive(Clone, Copy)]
enum StringFlavor {
    Byte, // b"..."
    C,    // c"..."
}

/// Per-flavor escape and content rules, modelled on the Rust reference.
#[derive(Clone, Copy)]
struct EscapeRules {
    /// `\u{...}` is permitted (string / char / C string; not byte string).
    unicode_escape: bool,
    /// `\xHH` may reach 0x00–0xFF (byte / C string) rather than ASCII 0x00–0x7F.
    byte_hex: bool,
    /// A NUL (`\0`, `\x00`, `\u{0}`, raw) is permitted (forbidden in C strings).
    allow_nul: bool,
    /// Unescaped source characters must be ASCII (byte strings only).
    ascii_only_raw: bool,
}

impl EscapeRules {
    /// `"..."` and `'...'`: Unicode escapes, ASCII-only `\x`, NUL allowed.
    const STRING: EscapeRules = EscapeRules {
        unicode_escape: true,
        byte_hex: false,
        allow_nul: true,
        ascii_only_raw: false,
    };
    /// `b"..."`: no Unicode escapes, full-range `\x`, ASCII-only raw content.
    const BYTE_STRING: EscapeRules = EscapeRules {
        unicode_escape: false,
        byte_hex: true,
        allow_nul: true,
        ascii_only_raw: true,
    };
    /// `c"..."`: Unicode escapes, full-range `\x`, no NUL of any form.
    const C_STRING: EscapeRules = EscapeRules {
        unicode_escape: true,
        byte_hex: true,
        allow_nul: false,
        ascii_only_raw: false,
    };
}

/// Convenience: lex `source` into tokens and errors.
pub fn tokenize(source: &str) -> (Vec<Token>, Vec<LexError>) {
    Lexer::new(source).run()
}

pub struct Lexer<'source> {
    source: &'source str,
    /// `(byte offset, character)` for every scalar in the source.
    characters: Vec<(usize, char)>,
    /// Index into `characters` of the next character to read.
    index: usize,
    line: u32,
    column: u32,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Lexer {
            source,
            characters: source.char_indices().collect(),
            index: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn run(mut self) -> (Vec<Token>, Vec<LexError>) {
        while let Some(c) = self.current() {
            match c {
                ' ' | '\t' => {
                    self.advance();
                }
                '\r' | '\n' => self.lex_newline(),
                '/' if self.peek(1) == Some('/') => self.lex_comment(),
                _ => self.lex_significant(c),
            }
        }
        let end = self.mark();
        self.emit(TokenKind::Eof, end);
        (self.tokens, self.errors)
    }

    // ---- Cursor primitives ---------------------------------------------

    /// The next character, without consuming it.
    fn current(&self) -> Option<char> {
        self.characters.get(self.index).map(|&(_, c)| c)
    }

    /// Look `offset` characters ahead without consuming.
    fn peek(&self, offset: usize) -> Option<char> {
        self.characters.get(self.index + offset).map(|&(_, c)| c)
    }

    /// Byte offset of the next character (or end of source if exhausted).
    fn current_byte(&self) -> usize {
        self.characters
            .get(self.index)
            .map(|&(byte, _)| byte)
            .unwrap_or(self.source.len())
    }

    /// Consume and return the next character, updating line/column.
    fn advance(&mut self) -> Option<char> {
        let &(_, c) = self.characters.get(self.index)?;
        self.index += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// If the next character is `expected`, consume it and return true.
    fn match_char(&mut self, expected: char) -> bool {
        if self.current() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume characters while `predicate` holds.
    fn consume_while(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(c) = self.current() {
            if predicate(c) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn mark(&self) -> Mark {
        Mark {
            byte: self.current_byte(),
            line: self.line,
            column: self.column,
        }
    }

    fn span_from(&self, start: Mark) -> Span {
        Span::new(start.byte, self.current_byte(), start.line, start.column)
    }

    fn emit(&mut self, kind: TokenKind, start: Mark) {
        let span = self.span_from(start);
        self.tokens.push(Token::new(kind, span));
    }

    fn error(&mut self, kind: LexErrorKind, start: Mark) {
        let span = self.span_from(start);
        self.errors.push(LexError { kind, span });
    }

    // ---- Trivia --------------------------------------------------------

    fn lex_newline(&mut self) {
        let start = self.mark();
        // Treat `\r`, `\n`, and `\r\n` as a single line break.
        if self.current() == Some('\r') {
            self.advance();
        }
        if self.current() == Some('\n') {
            self.advance();
        }
        // Coalesce: only emit if there's a non-newline token to terminate.
        let should_emit = matches!(self.tokens.last(), Some(token) if token.kind != TokenKind::Newline);
        if should_emit {
            self.emit(TokenKind::Newline, start);
        }
    }

    fn lex_comment(&mut self) {
        let start = self.mark();
        self.advance(); // first '/'
        self.advance(); // second '/'
        // Exactly three slashes (and not more) marks a doc comment.
        let is_doc = self.current() == Some('/') && self.peek(1) != Some('/');
        if is_doc {
            self.advance(); // third '/'
            let body_start = self.current_byte();
            self.consume_while(|c| c != '\n' && c != '\r');
            let body = &self.source[body_start..self.current_byte()];
            let trimmed = body.strip_prefix(' ').unwrap_or(body).trim_end();
            let text = trimmed.to_string();
            self.emit(TokenKind::DocumentationComment(text), start);
        } else {
            self.consume_while(|c| c != '\n' && c != '\r');
        }
    }

    // ---- Significant tokens --------------------------------------------

    fn lex_significant(&mut self, c: char) {
        if c == '"' {
            self.lex_string();
        } else if c == '\'' {
            self.lex_character();
        } else if c == 'b' && self.peek(1) == Some('"') {
            self.lex_prefixed_string(StringFlavor::Byte);
        } else if c == 'c' && self.peek(1) == Some('"') {
            self.lex_prefixed_string(StringFlavor::C);
        } else if c == '\\' && self.peek(1) == Some('\\') {
            self.lex_multiline_string();
        } else if c.is_ascii_digit() {
            self.lex_number();
        } else if is_identifier_start(c) {
            self.lex_identifier();
        } else {
            self.lex_operator(c);
        }
    }

    fn lex_identifier(&mut self) {
        let start = self.mark();
        self.consume_while(is_identifier_continue);
        let text = &self.source[start.byte..self.current_byte()];
        let kind = TokenKind::keyword_from(text)
            .unwrap_or_else(|| TokenKind::Identifier(text.to_string()));
        self.emit(kind, start);
    }

    fn lex_number(&mut self) {
        let start = self.mark();

        // Radix-prefixed integers: 0x.., 0o.., 0b..
        if self.current() == Some('0') {
            if let Some(radix) = match self.peek(1) {
                Some('x' | 'X') => Some(Radix::Hexadecimal),
                Some('o' | 'O') => Some(Radix::Octal),
                Some('b' | 'B') => Some(Radix::Binary),
                _ => None,
            } {
                self.advance(); // 0
                self.advance(); // prefix letter
                let digits_start = self.current_byte();
                self.consume_while(|c| is_radix_digit(c, radix) || c == '_');
                let digits = self.source[digits_start..self.current_byte()].to_string();
                self.finish_integer(&digits, radix, start);
                return;
            }
        }

        // Decimal integer part.
        self.consume_while(|c| c.is_ascii_digit() || c == '_');

        let mut is_float = false;

        // Fractional part — only if a digit follows the dot, so `1.method()`
        // stays an integer followed by `.`.
        if self.current() == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.advance(); // '.'
            self.consume_while(|c| c.is_ascii_digit() || c == '_');
        }

        // Exponent.
        if matches!(self.current(), Some('e' | 'E')) {
            is_float = true;
            self.advance(); // 'e'
            if matches!(self.current(), Some('+' | '-')) {
                self.advance();
            }
            let exponent_start = self.current_byte();
            self.consume_while(|c| c.is_ascii_digit() || c == '_');
            if self.current_byte() == exponent_start {
                self.error(LexErrorKind::MissingDigits, start);
                return;
            }
        }

        let raw = self.source[start.byte..self.current_byte()].to_string();
        if is_float {
            self.emit(TokenKind::Float(raw.replace('_', "")), start);
        } else {
            self.finish_integer(&raw, Radix::Decimal, start);
        }
    }

    /// Parse the (underscore-allowed) `digits` in `radix` and emit an integer,
    /// recording an error if it's empty or overflows `u128`.
    fn finish_integer(&mut self, digits: &str, radix: Radix, start: Mark) {
        let cleaned: String = digits.chars().filter(|&c| c != '_').collect();
        if cleaned.is_empty() {
            self.error(LexErrorKind::MissingDigits, start);
            return;
        }
        match u128::from_str_radix(&cleaned, radix.base()) {
            Ok(value) => self.emit(TokenKind::Integer { value, radix }, start),
            Err(_) => self.error(LexErrorKind::IntegerOverflow, start),
        }
    }

    fn lex_string(&mut self) {
        let start = self.mark();
        self.advance(); // opening quote
        let Some(raw) = self.collect_quoted(start, '"', LexErrorKind::UnterminatedString) else {
            return;
        };
        let bytes = self.decode_literal(&raw, EscapeRules::STRING, start);
        self.emit(TokenKind::String(utf8(bytes)), start);
    }

    /// Lex a `b"..."` byte string or `c"..."` C string. The `b`/`c` prefix and
    /// the opening quote are still pending at the cursor.
    fn lex_prefixed_string(&mut self, flavor: StringFlavor) {
        let start = self.mark();
        self.advance(); // prefix letter (`b` or `c`)
        self.advance(); // opening quote
        let Some(raw) = self.collect_quoted(start, '"', LexErrorKind::UnterminatedString) else {
            return;
        };
        let kind = match flavor {
            StringFlavor::Byte => {
                TokenKind::ByteString(self.decode_literal(&raw, EscapeRules::BYTE_STRING, start))
            }
            StringFlavor::C => {
                TokenKind::CString(self.decode_literal(&raw, EscapeRules::C_STRING, start))
            }
        };
        self.emit(kind, start);
    }

    fn lex_character(&mut self) {
        let start = self.mark();
        self.advance(); // opening quote
        let Some(raw) = self.collect_quoted(start, '\'', LexErrorKind::UnterminatedCharacter)
        else {
            return;
        };
        let decoded = utf8(self.decode_literal(&raw, EscapeRules::STRING, start));
        let mut characters = decoded.chars();
        match (characters.next(), characters.next()) {
            (None, _) => self.error(LexErrorKind::EmptyCharacter, start),
            (Some(character), None) => self.emit(TokenKind::Character(character), start),
            (Some(_), Some(_)) => self.error(LexErrorKind::MultipleCharacters, start),
        }
    }

    /// A Zig-style multiline string: one or more consecutive lines each
    /// beginning (after optional leading whitespace) with `\\`. The text after
    /// each marker is taken **verbatim** (no escape processing — these are raw),
    /// lines are joined with `\n`, and there is no trailing newline. The string
    /// ends at the first line that does not start with a `\\` marker.
    fn lex_multiline_string(&mut self) {
        let start = self.mark();
        let mut value = String::new();
        loop {
            self.advance(); // first '\'
            self.advance(); // second '\'
            // Rest of this line is content, taken verbatim.
            while let Some(c) = self.current() {
                if c == '\n' || c == '\r' {
                    break;
                }
                value.push(c);
                self.advance();
            }
            if self.next_line_continues_multiline() {
                value.push('\n');
                self.advance(); // '\r' or '\n'
                if self.current() == Some('\n') {
                    self.advance(); // '\n' of a '\r\n' pair
                }
                self.consume_while(|c| c == ' ' || c == '\t');
            } else {
                // Leave the trailing line break for the main loop to emit as a
                // statement-terminating Newline.
                break;
            }
        }
        self.emit(TokenKind::String(value), start);
    }

    /// Peek (without consuming) whether the next line, after its line break and
    /// leading whitespace, opens with a `\\` multiline marker.
    fn next_line_continues_multiline(&self) -> bool {
        let mut index = self.index;
        let char_at = |index: usize| self.characters.get(index).map(|&(_, c)| c);
        match char_at(index) {
            Some('\r') => {
                index += 1;
                if char_at(index) == Some('\n') {
                    index += 1;
                }
            }
            Some('\n') => index += 1,
            _ => return false, // EOF or no line break ⇒ string is finished
        }
        while matches!(char_at(index), Some(' ') | Some('\t')) {
            index += 1;
        }
        char_at(index) == Some('\\') && char_at(index + 1) == Some('\\')
    }

    /// Read raw literal text up to (and consuming) the closing `quote`,
    /// preserving backslash escapes verbatim for later decoding. A backslash
    /// pairs with the following character so an escaped quote does not close the
    /// literal. Records `unterminated` and returns `None` on a raw newline or
    /// end of input.
    fn collect_quoted(&mut self, start: Mark, quote: char, unterminated: LexErrorKind) -> Option<String> {
        let mut raw = String::new();
        loop {
            match self.current() {
                None | Some('\n') | Some('\r') => {
                    self.error(unterminated, start);
                    return None;
                }
                Some(c) if c == quote => {
                    self.advance();
                    return Some(raw);
                }
                Some('\\') => {
                    raw.push('\\');
                    self.advance();
                    match self.current() {
                        // A backslash before a line break is a dangling escape
                        // (no string continuation in v1) — treat as unterminated.
                        None | Some('\n') | Some('\r') => {
                            self.error(unterminated, start);
                            return None;
                        }
                        Some(escaped) => {
                            raw.push(escaped);
                            self.advance();
                        }
                    }
                }
                Some(c) => {
                    raw.push(c);
                    self.advance();
                }
            }
        }
    }

    /// Decode backslash escapes in already-collected literal text into bytes,
    /// applying the per-flavor `rules`. Errors are reported against the
    /// literal's start; recovery continues past a bad escape so one mistake
    /// doesn't swallow the rest of the literal.
    ///
    /// The result is the literal's bytes: for `"..."`/`'...'` and `c"..."` it is
    /// valid UTF-8 by construction; for `b"..."` it may be arbitrary bytes.
    fn decode_literal(&mut self, text: &str, rules: EscapeRules, start: Mark) -> Vec<u8> {
        let mut out = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\\' {
                self.push_raw_char(&mut out, c, rules, start);
                continue;
            }
            match chars.next() {
                Some('n') => out.push(b'\n'),
                Some('r') => out.push(b'\r'),
                Some('t') => out.push(b'\t'),
                Some('\\') => out.push(b'\\'),
                Some('"') => out.push(b'"'),
                Some('\'') => out.push(b'\''),
                Some('0') => {
                    if rules.allow_nul {
                        out.push(0);
                    } else {
                        self.error(LexErrorKind::NulInCString, start);
                    }
                }
                Some('x') => {
                    if let Some(byte) = self.read_hex_escape(&mut chars, rules, start) {
                        out.push(byte);
                    }
                }
                Some('u') if rules.unicode_escape => {
                    if let Some(scalar) = self.read_unicode_escape(&mut chars, start) {
                        if scalar == '\0' && !rules.allow_nul {
                            self.error(LexErrorKind::NulInCString, start);
                        } else {
                            push_utf8(&mut out, scalar);
                        }
                    }
                }
                Some('u') => self.error(LexErrorKind::UnicodeEscapeNotAllowed, start),
                Some(other) => self.error(LexErrorKind::InvalidEscape(other), start),
                None => self.error(LexErrorKind::InvalidEscape('\\'), start),
            }
        }
        out
    }

    /// Append an unescaped source character to a literal's bytes, enforcing the
    /// flavor's content rules (byte strings are ASCII-only; C strings reject a
    /// raw NUL).
    fn push_raw_char(&mut self, out: &mut Vec<u8>, c: char, rules: EscapeRules, start: Mark) {
        if rules.ascii_only_raw && !c.is_ascii() {
            self.error(LexErrorKind::NonAsciiByteString, start);
            return;
        }
        if c == '\0' && !rules.allow_nul {
            self.error(LexErrorKind::NulInCString, start);
            return;
        }
        push_utf8(out, c);
    }

    /// `\xHH` — exactly two hex digits. The allowed range depends on the flavor:
    /// ASCII (0x00–0x7F) for `"..."`/`'...'`, full 0x00–0xFF for byte/C strings.
    fn read_hex_escape(&mut self, chars: &mut Peekable<Chars<'_>>, rules: EscapeRules, start: Mark) -> Option<u8> {
        let mut value = 0u32;
        for _ in 0..2 {
            match chars.next().and_then(|c| c.to_digit(16)) {
                Some(digit) => value = value * 16 + digit,
                None => {
                    self.error(LexErrorKind::InvalidEscape('x'), start);
                    return None;
                }
            }
        }
        if !rules.byte_hex && value > 0x7F {
            self.error(LexErrorKind::AsciiEscapeOutOfRange, start);
            return None;
        }
        if value == 0 && !rules.allow_nul {
            self.error(LexErrorKind::NulInCString, start);
            return None;
        }
        Some(value as u8)
    }

    /// `\u{H...}` — 1 to 6 hex digits in braces, underscores allowed between
    /// digits (not leading), resolving to a valid scalar value (≤ 0x10FFFF, not
    /// a surrogate).
    fn read_unicode_escape(&mut self, chars: &mut Peekable<Chars<'_>>, start: Mark) -> Option<char> {
        if chars.next() != Some('{') {
            self.error(LexErrorKind::InvalidUnicodeEscape, start);
            return None;
        }
        let mut value: u32 = 0;
        let mut digit_count = 0usize;
        let mut closed = false;
        while let Some(&c) = chars.peek() {
            if c == '}' {
                chars.next();
                closed = true;
                break;
            }
            if c == '_' {
                if digit_count == 0 {
                    self.error(LexErrorKind::InvalidUnicodeEscape, start); // leading underscore
                    return None;
                }
                chars.next();
                continue;
            }
            match c.to_digit(16) {
                Some(digit) => {
                    value = value.saturating_mul(16).saturating_add(digit);
                    digit_count += 1;
                    chars.next();
                }
                None => {
                    self.error(LexErrorKind::InvalidUnicodeEscape, start);
                    return None;
                }
            }
        }
        if !closed || !(1..=6).contains(&digit_count) {
            self.error(LexErrorKind::InvalidUnicodeEscape, start);
            return None;
        }
        match char::from_u32(value) {
            Some(character) => Some(character),
            None => {
                self.error(LexErrorKind::InvalidUnicodeEscape, start);
                None
            }
        }
    }

    /// Lex an operator or punctuation token via maximal munch.
    ///
    /// Note: `>>` / `>>=` are produced greedily; a parser handling nested
    /// generics like `List<List<i32>>` must split a trailing `>>` itself.
    fn lex_operator(&mut self, c: char) {
        use TokenKind::*;
        let start = self.mark();
        self.advance(); // consume `c`

        let kind = match c {
            '+' => self.choose('=', PlusEqual, Plus),
            '*' => self.choose('=', StarEqual, Star),
            '/' => self.choose('=', SlashEqual, Slash),
            '%' => self.choose('=', PercentEqual, Percent),
            '=' => Equal,
            ',' => Comma,
            ';' => Semicolon,
            '@' => At,
            '#' => Hash,
            '(' => LeftParenthesis,
            ')' => RightParenthesis,
            '{' => LeftBrace,
            '}' => RightBrace,
            '[' => LeftBracket,
            ']' => RightBracket,
            ':' => self.choose('=', ColonEqual, Colon),

            '-' => {
                if self.match_char('>') {
                    Arrow
                } else {
                    self.choose('=', MinusEqual, Minus)
                }
            }

            '!' => {
                if self.match_char('!') {
                    BangBang
                } else {
                    self.choose('=', NotEqual, Bang)
                }
            }

            '?' => {
                if self.match_char('.') {
                    QuestionDot
                } else if self.match_char('?') {
                    QuestionQuestion
                } else {
                    Question
                }
            }

            '<' => {
                if self.match_char('<') {
                    self.choose('=', ShiftLeftEqual, ShiftLeft)
                } else {
                    self.choose('=', LessEqual, Less)
                }
            }
            '>' => {
                if self.match_char('>') {
                    self.choose('=', ShiftRightEqual, ShiftRight)
                } else {
                    self.choose('=', GreaterEqual, Greater)
                }
            }

            '&' => {
                if self.match_char('&') {
                    self.choose('=', AmpersandAmpersandEqual, AmpersandAmpersand)
                } else {
                    self.choose('=', AmpersandEqual, Ampersand)
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.choose('=', PipePipeEqual, PipePipe)
                } else {
                    self.choose('=', PipeEqual, Pipe)
                }
            }
            '^' => {
                if self.match_char('^') {
                    self.choose('=', CaretCaretEqual, CaretCaret)
                } else {
                    self.choose('=', CaretEqual, Caret)
                }
            }

            '.' => {
                if self.match_char('.') {
                    if self.match_char('=') {
                        DotDotEqual
                    } else if self.match_char('.') {
                        Ellipsis
                    } else {
                        DotDot
                    }
                } else {
                    Dot
                }
            }

            _ => {
                self.error(LexErrorKind::UnexpectedCharacter(c), start);
                return;
            }
        };
        self.emit(kind, start);
    }

    /// If the next character is `next`, consume it and return `with_equal`;
    /// otherwise return `plain`. Used for the common `op` / `op=` split.
    fn choose(&mut self, next: char, with_next: TokenKind, plain: TokenKind) -> TokenKind {
        if self.match_char(next) {
            with_next
        } else {
            plain
        }
    }
}

/// Append a character's UTF-8 encoding to a byte buffer.
fn push_utf8(out: &mut Vec<u8>, c: char) {
    let mut buffer = [0u8; 4];
    out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
}

/// Convert literal bytes known to be valid UTF-8 (string/char flavors) into a
/// `String`. The bytes are valid by construction, so this never fails.
fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("string/char literal decodes to valid UTF-8")
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_radix_digit(c: char, radix: Radix) -> bool {
    match radix {
        Radix::Decimal => c.is_ascii_digit(),
        Radix::Hexadecimal => c.is_ascii_hexdigit(),
        Radix::Octal => ('0'..='7').contains(&c),
        Radix::Binary => c == '0' || c == '1',
    }
}

#[cfg(test)]
mod tests;
