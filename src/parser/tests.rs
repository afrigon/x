use crate::ast::Expression;
use crate::lexer;

fn parse(source: &str) -> Expression {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    super::parse_expression(tokens).unwrap_or_else(|error| panic!("parse error: {error}"))
}

fn s_expression(source: &str) -> String {
    parse(source).to_string()
}

fn parse_error(source: &str) -> String {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    super::parse_expression(tokens)
        .expect_err("expected a parse error")
        .message
}

#[test]
fn literals() {
    assert_eq!(s_expression("42"), "42");
    assert_eq!(s_expression("0xFF"), "255");
    assert_eq!(s_expression("3.14"), "3.14");
    assert_eq!(s_expression(r#""hi""#), "\"hi\"");
    assert_eq!(s_expression("'a'"), "'a'");
    assert_eq!(s_expression("true"), "true");
    assert_eq!(s_expression("false"), "false");
    assert_eq!(s_expression("count"), "count");
}

#[test]
fn multiplicative_binds_tighter_than_additive() {
    assert_eq!(s_expression("1 + 2 * 3"), "(+ 1 (* 2 3))");
    assert_eq!(s_expression("1 * 2 + 3"), "(+ (* 1 2) 3)");
}

#[test]
fn binary_operators_are_left_associative() {
    assert_eq!(s_expression("1 - 2 - 3"), "(- (- 1 2) 3)");
    assert_eq!(s_expression("a / b / c"), "(/ (/ a b) c)");
}

#[test]
fn grouping_overrides_precedence() {
    assert_eq!(s_expression("(1 + 2) * 3"), "(* (+ 1 2) 3)");
}

#[test]
fn full_precedence_ladder() {
    assert_eq!(
        s_expression("a & b = c && d << e"),
        "(& a (= b (&& c (<< d e))))"
    );
    assert_eq!(s_expression("1 + 2 << 3"), "(<< (+ 1 2) 3)");
    assert_eq!(s_expression("a && b ^^ c || d"), "(|| (^^ (&& a b) c) d)");
}

#[test]
fn comparison_is_one_non_associative_level() {
    assert_eq!(s_expression("a < b"), "(< a b)");
    assert_eq!(s_expression("a = b"), "(= a b)");
    assert_eq!(s_expression("a < b & c = d"), "(& (< a b) (= c d))");
    assert_eq!(s_expression("a && b = c"), "(= (&& a b) c)");
}

#[test]
fn chained_comparison_is_rejected() {
    assert!(parse_error("a < b < c").contains("chain"));
    assert!(parse_error("a = b != c").contains("chain"));
}

#[test]
fn unary_operators() {
    assert_eq!(s_expression("-x"), "(- x)");
    assert_eq!(s_expression("!!bits"), "(!! bits)");
    assert_eq!(s_expression("-a * b"), "(* (- a) b)");
    assert_eq!(s_expression("!a & b"), "(& (! a) b)");
}

#[test]
fn calls_with_labels() {
    assert_eq!(s_expression("f()"), "(call f)");
    assert_eq!(s_expression("add(3, b: 5)"), "(call add 3 b: 5)");
    assert_eq!(
        s_expression("greet(name: \"Alice\")"),
        "(call greet name: \"Alice\")"
    );
}

#[test]
fn field_access_and_method_chains() {
    assert_eq!(s_expression("a.b.c"), "(. (. a b) c)");
    assert_eq!(s_expression("x.cstr()"), "(call (. x cstr))");
    assert_eq!(
        s_expression(r#"printf("hi".cstr())"#),
        "(call printf (call (. \"hi\" cstr)))",
    );
}

#[test]
fn indexing() {
    assert_eq!(s_expression("a[i]"), "(index a i)");
    assert_eq!(s_expression("grid[x][y]"), "(index (index grid x) y)");
    assert_eq!(s_expression("a[i + 1]"), "(index a (+ i 1))");
}

#[test]
fn implicit_member() {
    assert_eq!(s_expression(".plus"), ".plus");
    assert_eq!(s_expression(".number(5)"), "(call .number 5)");
}

#[test]
fn argument_list_may_span_lines() {
    let source = "f(\n    1,\n    b: 2,\n)";
    assert_eq!(s_expression(source), "(call f 1 b: 2)");
}

#[test]
fn reports_missing_operand() {
    assert!(parse_error("1 +").contains("expected an expression"));
}

#[test]
fn reports_unclosed_paren() {
    assert!(parse_error("(1 + 2").contains("expected `)`"));
}

#[test]
fn reports_leftover_tokens() {
    assert!(parse_error("1 2").contains("unexpected"));
}

fn type_of(source: &str) -> String {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    super::parse_type(tokens)
        .unwrap_or_else(|error| panic!("parse error: {error}"))
        .to_string()
}

fn type_error(source: &str) -> String {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    super::parse_type(tokens)
        .expect_err("expected a parse error")
        .message
}

#[test]
fn named_and_generic_types() {
    assert_eq!(type_of("i32"), "i32");
    assert_eq!(type_of("string"), "string");
    assert_eq!(type_of("Self"), "Self");
    assert_eq!(type_of("List<i32>"), "List<i32>");
    assert_eq!(
        type_of("HashMap<string, List<i32>>"),
        "HashMap<string, List<i32>>"
    );
}

#[test]
fn nested_generics_split_the_shift_token() {
    // The lexer produces `>>` here; the parser must split it.
    assert_eq!(type_of("List<List<List<i32>>>"), "List<List<List<i32>>>");
}

#[test]
fn references_and_pointers() {
    assert_eq!(type_of("&T"), "&T");
    assert_eq!(type_of("&mut Point"), "&mut Point");
    assert_eq!(type_of("*u8"), "*u8");
    assert_eq!(type_of("*mut u8"), "*mut u8");
    assert_eq!(type_of("*void"), "*void");
    assert_eq!(type_of("&mut *T"), "&mut *T");
}

#[test]
fn optional_and_result_suffixes() {
    assert_eq!(type_of("string?"), "string?");
    assert_eq!(type_of("i32!IoError"), "i32!IoError");
    assert_eq!(type_of("List<Token>!LexError"), "List<Token>!LexError");
    assert_eq!(type_of("T?!E"), "T?!E");
    assert_eq!(type_of("T!E?"), "T!E?");
}

#[test]
fn prefix_binds_tighter_than_optional_suffix() {
    let parsed_type = {
        let (tokens, _) = lexer::tokenize("&mut Node?");
        super::parse_type(tokens).unwrap()
    };
    assert!(matches!(
        parsed_type.kind,
        crate::ast::TypeKind::Optional(_)
    ));
    assert_eq!(type_of("*(u8?)"), "*u8?");
}

#[test]
fn arrays_and_slices() {
    assert_eq!(type_of("[u8]"), "[u8]");
    assert_eq!(type_of("[u8; 16]"), "[u8; 16]");
    assert_eq!(type_of("[List<i32>; SIZE]"), "[List<i32>; SIZE]");
    assert_eq!(type_of("[u8; 4 * 4]"), "[u8; (* 4 4)]");
}

#[test]
fn function_types() {
    assert_eq!(type_of("fun()"), "fun()");
    assert_eq!(type_of("fun(i32, i32) -> i32"), "fun(i32, i32) -> i32");
    assert_eq!(type_of("fun(*u8, ...) -> i32"), "fun(*u8, ...) -> i32");
    assert_eq!(type_of("fun(string) -> bool?"), "fun(string) -> bool?");
}

#[test]
fn grouping_is_not_a_tuple() {
    assert_eq!(type_of("(i32)"), "i32");
    assert!(type_error("(i32, u8)").contains("tuples are not supported"));
    assert!(type_error("()").contains("not a type"));
}

fn block(source: &str) -> String {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    super::parse_block(tokens)
        .unwrap_or_else(|error| panic!("parse error: {error}"))
        .to_string()
}

fn block_error(source: &str) -> String {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    super::parse_block(tokens)
        .expect_err("expected a parse error")
        .message
}

#[test]
fn empty_block_is_unit_valued() {
    assert_eq!(block("{}"), "(block)");
    assert_eq!(block("{\n\n}"), "(block)");
}

#[test]
fn let_bindings() {
    assert_eq!(block("{ let x := 5 }"), "(block (let x := 5))");
    assert_eq!(
        block("{ let mut count := 0 }"),
        "(block (let mut count := 0))"
    );
    assert_eq!(
        block("{ let pi: f64 := 3.14159 }"),
        "(block (let pi: f64 := 3.14159))"
    );
}

#[test]
fn let_is_a_statement_not_a_block_value() {
    assert_eq!(block("{ let x := 5\nx }"), "(block (let x := 5) => x)");
}

#[test]
fn assignment_and_compound_assignment() {
    assert_eq!(
        block("{ count := count + 1 }"),
        "(block (:= count (+ count 1)))"
    );
    assert_eq!(block("{ total += x }"), "(block (+= total x))");
    assert_eq!(block("{ flags ||= mask }"), "(block (||= flags mask))");
    assert_eq!(block("{ a.b := 1 }"), "(block (:= (. a b) 1))");
}

#[test]
fn return_statements() {
    assert_eq!(block("{ return }"), "(block (return))");
    assert_eq!(block("{ return a + b }"), "(block (return (+ a b)))");
}

#[test]
fn trailing_expression_is_the_block_value() {
    assert_eq!(block("{ a\nb\nc }"), "(block a b => c)");
    assert_eq!(block("{ f()\ng() }"), "(block (call f) => (call g))");
}

#[test]
fn unsafe_block() {
    assert_eq!(
        block(r#"{ unsafe { printf("hi".cstr()) } }"#),
        "(block => (unsafe (block => (call printf (call (. \"hi\" cstr))))))",
    );
}

#[test]
fn nested_blocks_and_let_with_block_value() {
    assert_eq!(
        block("{ let y := {\n  let t := x * 2\n  t + 1\n} }"),
        "(block (let y := (block (let t := (* x 2)) => (+ t 1))))",
    );
}

#[test]
fn statements_must_be_newline_separated() {
    assert!(block_error("{ a b }").contains("newline"));
}

#[test]
fn parses_the_phase_one_main_body() {
    assert_eq!(
        block("{\n    unsafe {\n        printf(c\"hello, world\\n\")\n    }\n}"),
        "(block => (unsafe (block => (call printf c\"hello, world\\n\"))))",
    );
}
