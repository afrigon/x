use super::*;
use crate::token::{Radix, TokenKind};

/// Lex `source`, asserting it produced no errors, and return the token kinds
/// with the trailing `Eof` stripped for compact comparisons.
fn kinds(source: &str) -> Vec<TokenKind> {
    let (tokens, errors) = tokenize(source);
    assert!(errors.is_empty(), "unexpected lex errors: {errors:?}");
    assert_eq!(tokens.last().map(|t| &t.kind), Some(&TokenKind::Eof));
    tokens[..tokens.len() - 1]
        .iter()
        .map(|t| t.kind.clone())
        .collect()
}

#[test]
fn empty_source_is_just_eof() {
    let (tokens, errors) = tokenize("");
    assert!(errors.is_empty());
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn keywords_versus_identifiers() {
    use TokenKind::*;
    assert_eq!(
        kinds("fun let mut loops"),
        vec![Fun, Let, Mut, Identifier("loops".into())],
    );
}

#[test]
fn identifiers_allow_underscores_and_digits() {
    use TokenKind::*;
    assert_eq!(
        kinds("_ _unused camelCase x2 Self self"),
        vec![
            Identifier("_".into()),
            Identifier("_unused".into()),
            Identifier("camelCase".into()),
            Identifier("x2".into()),
            SelfType,
            SelfValue,
        ],
    );
}

#[test]
fn integer_radices() {
    use TokenKind::*;
    assert_eq!(
        kinds("0 42 1_000 0xFF 0o17 0b1010"),
        vec![
            Integer { value: 0, radix: Radix::Decimal },
            Integer { value: 42, radix: Radix::Decimal },
            Integer { value: 1000, radix: Radix::Decimal },
            Integer { value: 255, radix: Radix::Hexadecimal },
            Integer { value: 15, radix: Radix::Octal },
            Integer { value: 10, radix: Radix::Binary },
        ],
    );
}

#[test]
fn floats_with_fraction_and_exponent() {
    use TokenKind::*;
    assert_eq!(
        kinds("3.14159 1e10 1.5e-3 2.0E+8"),
        vec![
            Float("3.14159".into()),
            Float("1e10".into()),
            Float("1.5e-3".into()),
            Float("2.0E+8".into()),
        ],
    );
}

#[test]
fn dot_after_integer_is_not_a_float() {
    use TokenKind::*;
    // `1.foo` is integer, dot, identifier — method call, not a float.
    assert_eq!(
        kinds("1.foo"),
        vec![
            Integer { value: 1, radix: Radix::Decimal },
            Dot,
            Identifier("foo".into()),
        ],
    );
}

#[test]
fn strings_and_escapes() {
    use TokenKind::*;
    assert_eq!(
        kinds(r#""hello, world\n""#),
        vec![String("hello, world\n".into())],
    );
    assert_eq!(
        kinds(r#""tab\tquote\"end""#),
        vec![String("tab\tquote\"end".into())],
    );
    assert_eq!(kinds(r#""\u{1F600}""#), vec![String("\u{1F600}".into())]);
    assert_eq!(kinds(r#""\x41""#), vec![String("A".into())]);
    assert_eq!(kinds(r#""\x7f""#), vec![String("\u{7f}".into())]); // top of ASCII range
}

#[test]
fn unicode_escape_allows_underscores() {
    // Rust permits underscores between hex digits in `\u{...}`.
    assert_eq!(kinds(r#""\u{1_F600}""#), vec![TokenKind::String("\u{1F600}".into())]);
}

#[test]
fn byte_strings() {
    use TokenKind::*;
    assert_eq!(kinds(r#"b"abc""#), vec![ByteString(vec![97, 98, 99])]);
    // Byte escapes reach the full 0x00–0xFF range (unlike `"..."`).
    assert_eq!(kinds(r#"b"\xff\x00\n""#), vec![ByteString(vec![0xFF, 0x00, b'\n'])]);
}

#[test]
fn byte_string_rejects_unicode_escape() {
    let (_, errors) = tokenize(r#"b"\u{41}""#);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnicodeEscapeNotAllowed);
}

#[test]
fn byte_string_rejects_non_ascii_source() {
    let (_, errors) = tokenize("b\"é\"");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::NonAsciiByteString);
}

#[test]
fn c_strings_utf8_encode_and_forbid_nul() {
    use TokenKind::*;
    // ASCII content is one byte each; the implicit terminator is not stored.
    assert_eq!(kinds(r#"c"hi""#), vec![CString(vec![b'h', b'i'])]);
    // Non-ASCII is UTF-8 encoded; a raw char and its `\u{}` agree.
    assert_eq!(kinds("c\"é\""), vec![CString(vec![0xC3, 0xA9])]);
    assert_eq!(kinds(r#"c"\u{e9}""#), vec![CString(vec![0xC3, 0xA9])]);
    // A `\xHH` escape produces a raw byte.
    assert_eq!(kinds(r#"c"\xC3\xA9""#), vec![CString(vec![0xC3, 0xA9])]);
}

#[test]
fn c_string_rejects_every_form_of_nul() {
    for source in [r#"c"\0""#, r#"c"\x00""#, r#"c"\u{0}""#] {
        let (_, errors) = tokenize(source);
        assert_eq!(errors.len(), 1, "source: {source}");
        assert_eq!(errors[0].kind, LexErrorKind::NulInCString, "source: {source}");
    }
}

#[test]
fn b_and_c_are_identifiers_without_a_following_quote() {
    use TokenKind::*;
    // The byte/C prefix only applies immediately before `"`.
    assert_eq!(
        kinds("b bar c"),
        vec![Identifier("b".into()), Identifier("bar".into()), Identifier("c".into())],
    );
}

#[test]
fn multiline_string_joins_marked_lines() {
    use TokenKind::*;
    // The `\\` marker; leading whitespace before it is ignored, text after it
    // is verbatim. Lines join with `\n`, no trailing newline.
    let source = "let s := \\\\a\n         \\\\  b\n";
    assert_eq!(
        kinds(source),
        vec![Let, Identifier("s".into()), ColonEqual, String("a\n  b".into()), Newline],
    );
}

#[test]
fn multiline_string_is_raw() {
    // Backslash escapes are NOT processed inside a multiline string.
    let source = "\\\\path\\to\\file\\n";
    assert_eq!(kinds(source), vec![TokenKind::String("path\\to\\file\\n".into())]);
}

#[test]
fn multiline_string_blank_line_via_bare_marker() {
    let source = "\\\\a\n\\\\\n\\\\b";
    assert_eq!(kinds(source), vec![TokenKind::String("a\n\nb".into())]);
}

#[test]
fn multiline_string_ends_at_unmarked_line() {
    use TokenKind::*;
    // The second line isn't a marker, so the string is just "a" and `b` lexes
    // as an ordinary identifier on the next line.
    let source = "\\\\a\nb";
    assert_eq!(kinds(source), vec![String("a".into()), Newline, Identifier("b".into())]);
}

#[test]
fn character_literals() {
    use TokenKind::*;
    assert_eq!(
        kinds(r"'a' '\n' '\'' '0'"),
        vec![
            Character('a'),
            Character('\n'),
            Character('\''),
            Character('0'),
        ],
    );
}

#[test]
fn maximal_munch_operators() {
    use TokenKind::*;
    assert_eq!(
        kinds(":= -> .. ..= ... ?. ?? << >>= &&= ||"),
        vec![
            ColonEqual, Arrow, DotDot, DotDotEqual, Ellipsis, QuestionDot,
            QuestionQuestion, ShiftLeft, ShiftRightEqual, AmpersandAmpersandEqual,
            PipePipe,
        ],
    );
}

#[test]
fn semicolon_lexes_as_a_token() {
    use TokenKind::*;
    // `;` is not a statement terminator, but it is a token (array sizes).
    assert_eq!(
        kinds("[u8; 4]"),
        vec![
            LeftBracket,
            Identifier("u8".into()),
            Semicolon,
            Integer { value: 4, radix: Radix::Decimal },
            RightBracket,
        ],
    );
}

#[test]
fn boolean_versus_bitwise_operators() {
    use TokenKind::*;
    assert_eq!(
        kinds("& && | || ! !! ^ ^^"),
        vec![
            Ampersand, AmpersandAmpersand, Pipe, PipePipe, Bang, BangBang, Caret,
            CaretCaret,
        ],
    );
}

#[test]
fn newlines_coalesce_and_skip_leading() {
    use TokenKind::*;
    // Leading blank lines dropped; the run of blank lines between `a` and `b`
    // collapses to one Newline; trailing newline before Eof emits one Newline.
    assert_eq!(
        kinds("\n\n  a\n\n\n  b\n"),
        vec![Identifier("a".into()), Newline, Identifier("b".into()), Newline],
    );
}

#[test]
fn line_comments_are_skipped_doc_comments_kept() {
    use TokenKind::*;
    let source = "// just a comment\nlet x // trailing\n/// doc body\nfun";
    assert_eq!(
        kinds(source),
        vec![
            Let,
            Identifier("x".into()),
            Newline,
            DocumentationComment("doc body".into()),
            Newline,
            Fun,
        ],
    );
}

#[test]
fn four_slashes_is_a_line_comment_not_doc() {
    // `////` is an ordinary comment, so nothing is emitted on its line.
    assert_eq!(kinds("//// not a doc\n42"),
        vec![TokenKind::Integer { value: 42, radix: Radix::Decimal }]);
}

#[test]
fn spans_track_line_and_column() {
    let (tokens, errors) = tokenize("fun\n  add");
    assert!(errors.is_empty());
    assert_eq!((tokens[0].span.line, tokens[0].span.column), (1, 1)); // fun
    // tokens[1] is the Newline; tokens[2] is `add` on line 2, column 3.
    let add = &tokens[2];
    assert_eq!(add.kind, TokenKind::Identifier("add".into()));
    assert_eq!((add.span.line, add.span.column), (2, 3));
}

#[test]
fn unterminated_string_reports_error() {
    let (_, errors) = tokenize("\"no close\n");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnterminatedString);
}

#[test]
fn hex_escape_above_ascii_is_an_error() {
    let (_, errors) = tokenize(r#""\xFF""#);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::AsciiEscapeOutOfRange);
}

#[test]
fn multi_character_literal_is_an_error() {
    let (_, errors) = tokenize("'ab'");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::MultipleCharacters);
}

#[test]
fn overlong_unicode_escape_is_an_error() {
    let (_, errors) = tokenize(r#""\u{1234567}""#); // 7 hex digits
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::InvalidUnicodeEscape);
}

#[test]
fn integer_overflow_reports_error() {
    let huge = "9".repeat(40);
    let (_, errors) = tokenize(&huge);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::IntegerOverflow);
}

#[test]
fn unexpected_character_recovers() {
    use TokenKind::*;
    // The stray `$` is reported but lexing continues around it.
    let (tokens, errors) = tokenize("a $ b");
    assert_eq!(errors.len(), 1);
    assert!(matches!(errors[0].kind, LexErrorKind::UnexpectedCharacter('$')));
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(kinds, vec![Identifier("a".into()), Identifier("b".into()), Eof]);
}

#[test]
fn lexes_the_phase_one_hello_world() {
    use TokenKind::*;
    let source = "\
@extern(.c)
fun printf(_ fmt: *u8, ...) -> i32

fun main() {
    unsafe {
        printf(\"hello, world\\n\".cstr())
    }
}
";
    let (tokens, errors) = tokenize(source);
    assert!(errors.is_empty(), "errors: {errors:?}");
    // Spot-check the shape rather than the full stream.
    assert_eq!(tokens[0].kind, At);
    assert_eq!(tokens[1].kind, Identifier("extern".into()));
    assert!(tokens.iter().any(|t| t.kind == Ellipsis));
    assert!(tokens.iter().any(|t| t.kind == Arrow));
    assert!(tokens.iter().any(|t| matches!(&t.kind, String(s) if s == "hello, world\n")));
    assert_eq!(tokens.last().unwrap().kind, Eof);
}
