use crate::token::{Radix, Span, Token, TokenKind};
use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Clone, Debug, PartialEq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LexErrorKind {
    UnexpectedCharacter(char),
    UnterminatedString,
    UnterminatedCharacter,
    EmptyCharacter,
    MultipleCharacters,
    InvalidEscape(char),
    AsciiEscapeOutOfRange,
    UnicodeEscapeNotAllowed,
    NonAsciiByteString,
    NulInCString,
    InvalidUnicodeEscape,
    IntegerOverflow,
    MissingDigits,
}

impl LexError {
    pub fn message(&self) -> String {
        use LexErrorKind::*;
        match &self.kind {
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
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "line {}, column {}: {}",
            self.span.line,
            self.span.column,
            self.message()
        )
    }
}

#[derive(Clone, Copy)]
struct Mark {
    byte: usize,
    line: u32,
    column: u32,
}

#[derive(Clone, Copy)]
enum StringFlavor {
    Byte,
    C,
}

#[derive(Clone, Copy)]
struct EscapeRules {
    unicode_escape: bool,
    byte_hex: bool,
    allow_nul: bool,
    ascii_only_raw: bool,
}

impl EscapeRules {
    const STRING: EscapeRules = EscapeRules {
        unicode_escape: true,
        byte_hex: false,
        allow_nul: true,
        ascii_only_raw: false,
    };
    const BYTE_STRING: EscapeRules = EscapeRules {
        unicode_escape: false,
        byte_hex: true,
        allow_nul: true,
        ascii_only_raw: true,
    };
    const C_STRING: EscapeRules = EscapeRules {
        unicode_escape: true,
        byte_hex: true,
        allow_nul: false,
        ascii_only_raw: false,
    };
}

pub fn tokenize(source: &str) -> (Vec<Token>, Vec<LexError>) {
    Lexer::new(source).run()
}

pub struct Lexer<'source> {
    source: &'source str,
    characters: Vec<(usize, char)>,
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

    fn current(&self) -> Option<char> {
        self.characters.get(self.index).map(|&(_, c)| c)
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.characters.get(self.index + offset).map(|&(_, c)| c)
    }

    fn current_byte(&self) -> usize {
        self.characters
            .get(self.index)
            .map(|&(byte, _)| byte)
            .unwrap_or(self.source.len())
    }

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

    fn match_char(&mut self, expected: char) -> bool {
        if self.current() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

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

    fn lex_newline(&mut self) {
        let start = self.mark();
        if self.current() == Some('\r') {
            self.advance();
        }
        if self.current() == Some('\n') {
            self.advance();
        }
        let should_emit =
            matches!(self.tokens.last(), Some(token) if token.kind != TokenKind::Newline);
        if should_emit {
            self.emit(TokenKind::Newline, start);
        }
    }

    fn lex_comment(&mut self) {
        let start = self.mark();
        self.advance();
        self.advance();
        // Exactly three slashes (and not more) marks a doc comment.
        let is_doc = self.current() == Some('/') && self.peek(1) != Some('/');
        if is_doc {
            self.advance();
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

        if self.current() == Some('0') {
            if let Some(radix) = match self.peek(1) {
                Some('x' | 'X') => Some(Radix::Hexadecimal),
                Some('o' | 'O') => Some(Radix::Octal),
                Some('b' | 'B') => Some(Radix::Binary),
                _ => None,
            } {
                self.advance();
                self.advance();
                let digits_start = self.current_byte();
                self.consume_while(|c| is_radix_digit(c, radix) || c == '_');
                let digits = self.source[digits_start..self.current_byte()].to_string();
                self.finish_integer(&digits, radix, start);
                return;
            }
        }

        self.consume_while(|c| c.is_ascii_digit() || c == '_');

        let mut is_float = false;

        // Fractional part — only if a digit follows the dot, so `1.method()`
        // stays an integer followed by `.`.
        if self.current() == Some('.') && self.peek(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            self.advance();
            self.consume_while(|c| c.is_ascii_digit() || c == '_');
        }

        if matches!(self.current(), Some('e' | 'E')) {
            is_float = true;
            self.advance();
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
        self.advance();
        let Some(raw) = self.collect_quoted(start, '"', LexErrorKind::UnterminatedString) else {
            return;
        };
        let bytes = self.decode_literal(&raw, EscapeRules::STRING, start);
        self.emit(TokenKind::String(utf8(bytes)), start);
    }

    fn lex_prefixed_string(&mut self, flavor: StringFlavor) {
        let start = self.mark();
        self.advance();
        self.advance();
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
        self.advance();
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

    fn lex_multiline_string(&mut self) {
        let start = self.mark();
        let mut value = String::new();
        loop {
            self.advance();
            self.advance();
            while let Some(c) = self.current() {
                if c == '\n' || c == '\r' {
                    break;
                }
                value.push(c);
                self.advance();
            }
            if self.next_line_continues_multiline() {
                value.push('\n');
                self.advance();
                if self.current() == Some('\n') {
                    self.advance();
                }
                self.consume_while(|c| c == ' ' || c == '\t');
            } else {
                break;
            }
        }
        self.emit(TokenKind::String(value), start);
    }

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
            _ => return false,
        }
        while matches!(char_at(index), Some(' ') | Some('\t')) {
            index += 1;
        }
        char_at(index) == Some('\\') && char_at(index + 1) == Some('\\')
    }

    fn collect_quoted(
        &mut self,
        start: Mark,
        quote: char,
        unterminated: LexErrorKind,
    ) -> Option<String> {
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
                        // A backslash before a line break is a dangling escape, not a continuation:
                        // the literal is unterminated.
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

    fn read_hex_escape(
        &mut self,
        chars: &mut Peekable<Chars<'_>>,
        rules: EscapeRules,
        start: Mark,
    ) -> Option<u8> {
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

    fn read_unicode_escape(
        &mut self,
        chars: &mut Peekable<Chars<'_>>,
        start: Mark,
    ) -> Option<char> {
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
                    self.error(LexErrorKind::InvalidUnicodeEscape, start);
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

    // `>>` / `>>=` are munched greedily; the parser splits a trailing `>>` in nested generics.
    fn lex_operator(&mut self, c: char) {
        use TokenKind::*;
        let start = self.mark();
        self.advance();

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

    fn choose(&mut self, next: char, with_next: TokenKind, plain: TokenKind) -> TokenKind {
        if self.match_char(next) {
            with_next
        } else {
            plain
        }
    }
}

fn push_utf8(out: &mut Vec<u8>, c: char) {
    let mut buffer = [0u8; 4];
    out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
}

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
