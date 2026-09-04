use super::*;
use crate::lexer;
use crate::parser;

const PRINTF: &str = "@extern(.c)\nfun printf(_ format: *u8, ...) -> i32\n";

fn check_source(source: &str) -> Result<TypedProgram, Vec<String>> {
    let (tokens, errors) = lexer::tokenize(source);
    assert!(errors.is_empty(), "lex errors: {errors:?}");
    let program =
        parser::parse_program(tokens).unwrap_or_else(|error| panic!("parse error: {error}"));
    check(program).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}: {}",
                    diagnostic.span.line, diagnostic.span.column, diagnostic.message
                )
            })
            .collect()
    })
}

fn passes(source: &str) -> TypedProgram {
    check_source(source).unwrap_or_else(|diagnostics| panic!("diagnostics: {diagnostics:#?}"))
}

fn diagnostics(source: &str) -> Vec<String> {
    check_source(source).expect_err("expected diagnostics")
}

fn type_of(typed: &TypedProgram, line: u32, column: u32) -> String {
    typed
        .expression_types
        .iter()
        .filter_map(|(id, found)| {
            let span = find_span(&typed.program, *id).expect("expression exists");
            (span.line == line && span.column == column).then_some((span.end, found))
        })
        .max_by_key(|(end, _)| *end)
        .map(|(_, found)| found.to_string())
        .unwrap_or_else(|| panic!("no expression at {line}:{column}"))
}

fn find_span(program: &Program, id: ExpressionId) -> Option<Span> {
    fn in_expression(expression: &Expression, id: ExpressionId) -> Option<Span> {
        if expression.id == id {
            return Some(expression.span);
        }
        match &expression.kind {
            ExpressionKind::Unary { operand, .. } => in_expression(operand, id),
            ExpressionKind::Binary { left, right, .. } => {
                in_expression(left, id).or_else(|| in_expression(right, id))
            }
            ExpressionKind::Call { callee, arguments } => in_expression(callee, id).or_else(|| {
                arguments
                    .iter()
                    .find_map(|argument| in_expression(&argument.value, id))
            }),
            ExpressionKind::Field { receiver, .. } => in_expression(receiver, id),
            ExpressionKind::Index { receiver, index } => {
                in_expression(receiver, id).or_else(|| in_expression(index, id))
            }
            ExpressionKind::Block { statements, value } => statements
                .iter()
                .find_map(|statement| in_statement(statement, id))
                .or_else(|| value.as_ref().and_then(|value| in_expression(value, id))),
            ExpressionKind::Unsafe(inner) => in_expression(inner, id),
            _ => None,
        }
    }
    fn in_statement(statement: &Statement, id: ExpressionId) -> Option<Span> {
        match &statement.kind {
            StatementKind::Let { value, .. } => in_expression(value, id),
            StatementKind::Assignment { target, value, .. } => {
                in_expression(target, id).or_else(|| in_expression(value, id))
            }
            StatementKind::Return(value) => {
                value.as_ref().and_then(|value| in_expression(value, id))
            }
            StatementKind::Expression(expression) => in_expression(expression, id),
        }
    }
    program.declarations.iter().find_map(|declaration| {
        let DeclarationKind::Function(function) = &declaration.kind;
        function
            .body
            .as_ref()
            .and_then(|body| in_expression(body, id))
    })
}

#[test]
fn hello_world_checks() {
    let source = format!(
        "{PRINTF}\nfun main() {{\n    unsafe {{\n        printf(c\"hello, world\\n\")\n    }}\n}}\n"
    );
    let typed = passes(&source);
    assert_eq!(typed.functions.len(), 2);
    assert_eq!(
        typed.functions[0].foreign,
        Some(Foreign {
            symbol: "printf".to_string(),
            link: None
        })
    );
    assert_eq!(typed.callees.len(), 1);
    assert_eq!(type_of(&typed, 6, 9), "i32");
    assert_eq!(type_of(&typed, 6, 16), "*u8");
}

#[test]
fn extern_call_needs_unsafe() {
    let source = format!("{PRINTF}\nfun main() {{\n    printf(c\"hi\")\n}}\n");
    assert_eq!(
        diagnostics(&source),
        ["5:5: `printf` is `@extern`, so calling it requires an `unsafe` block"]
    );
}

#[test]
fn unsafe_attribute_needs_unsafe_at_call_sites() {
    let source = "@unsafe\nfun poke() {}\n\nfun main() {\n    poke()\n    unsafe { poke() }\n}\n";
    assert_eq!(
        diagnostics(source),
        ["5:5: `poke` is `@unsafe`, so calling it requires an `unsafe` block"]
    );
}

#[test]
fn unknown_function_and_name() {
    assert_eq!(
        diagnostics("fun main() {\n    greet()\n    let y := x\n}\n"),
        ["2:5: unknown function `greet`", "3:14: unknown name `x`"]
    );
}

#[test]
fn arity_and_labels() {
    let source = "fun add(_ a: i32, b: i32) -> i32 { return a + b }\n\nfun main() {\n    add(1)\n    add(1, 2)\n    add(a: 1, b: 2)\n    add(1, c: 2)\n    add(1, b: 2, 3)\n}\n";
    assert_eq!(
        diagnostics(source),
        [
            "4:5: `add` takes 2 arguments, found 1",
            "5:12: argument 2 needs the label `b:`",
            "6:9: argument 1 takes no label, found `a:`",
            "7:12: expected label `b:`, found `c:`",
            "8:5: `add` takes 2 arguments, found 3",
        ]
    );
}

#[test]
fn variadic_accepts_extra_arguments() {
    let source = format!(
        "{PRINTF}\nfun main() {{\n    unsafe {{\n        printf(c\"%d %f\\n\", 1, 2.5)\n        printf()\n        printf(c\"\", true)\n    }}\n}}\n"
    );
    assert_eq!(
        diagnostics(&source),
        [
            "7:9: `printf` takes at least 1 argument, found 0",
            "8:21: `bool` cannot be passed as a C variadic argument",
        ]
    );
}

#[test]
fn literal_defaults_and_expectations() {
    let typed = passes(
        "fun main() {\n    let a := 5\n    let b: i64 := 5\n    let c := 2.5\n    let d: f32 := 2.5\n    let e := -a\n}\n",
    );
    assert_eq!(type_of(&typed, 2, 14), "i32");
    assert_eq!(type_of(&typed, 3, 19), "i64");
    assert_eq!(type_of(&typed, 4, 14), "f64");
    assert_eq!(type_of(&typed, 5, 19), "f32");
    assert_eq!(type_of(&typed, 6, 14), "i32");
}

#[test]
fn literal_adopts_the_other_operand() {
    let typed =
        passes("fun main() {\n    let a: i64 := 1\n    let b := 2 * a\n    let c := a < 3\n}\n");
    assert_eq!(type_of(&typed, 3, 14), "i64");
    assert_eq!(type_of(&typed, 4, 14), "bool");
}

#[test]
fn type_mismatches() {
    assert_eq!(
        diagnostics(
            "fun main() {\n    let a: i64 := 2.5\n    let b: bool := 1\n    let c := 1 + true\n    let d := !3\n}\n"
        ),
        [
            "2:19: expected `i64`, found `f64`",
            "3:20: expected `bool`, found `i32`",
            "4:18: expected `i32`, found `bool`",
            "5:15: expected `bool`, found `i32`",
        ]
    );
}

#[test]
fn assignment_rules() {
    assert_eq!(
        diagnostics(
            "fun main() {\n    let a := 1\n    a := 2\n    let mut b := 1\n    b := 2\n    b += 1\n    b := 2.5\n    let mut c := true\n    c += true\n}\n"
        ),
        [
            "3:5: `a` is immutable; declare it with `let mut`",
            "7:10: expected `i32`, found `f64`",
            "9:5: `+=` cannot be applied to `bool`",
        ]
    );
}

#[test]
fn return_rules() {
    assert_eq!(
        diagnostics(
            "fun a() -> i32 { 1 }\nfun b() -> i32 { return }\nfun c() { return 1 }\nfun d() -> i32 { return true }\nfun e() -> i32 { return 1 }\n"
        ),
        [
            "1:1: function `a` declares `-> i32` but its body does not return",
            "2:18: `return` needs a value of type `i32`",
            "3:18: `return` carries a value in a function returning unit",
            "4:25: expected `i32`, found `bool`",
        ]
    );
}

#[test]
fn blocks_have_the_type_of_their_value() {
    let typed = passes(
        "fun main() {\n    let a := {\n        let r := 2\n        r * r\n    }\n    let b: i64 := { 7 }\n}\n",
    );
    assert_eq!(type_of(&typed, 2, 14), "i32");
    assert_eq!(type_of(&typed, 6, 19), "i64");
}

#[test]
fn same_scope_redeclaration_is_an_error() {
    assert_eq!(
        diagnostics(
            "fun main() {\n    let a := 1\n    let a := 2\n    {\n        let a := 3\n    }\n}\n"
        ),
        ["3:5: `a` is already declared in this scope"]
    );
}

#[test]
fn extern_validation() {
    let source = "@extern\nfun a()\n@extern(.rust)\nfun b()\n@extern(.c, symbol: 3)\nfun c()\n@extern(.c, callconv: .stdcall)\nfun d()\n@extern(.c, name: \"x\")\nfun e()\n@extern(.c)\nfun f(_ text: string, _ c: char) -> char\n@extern(.c, symbol: \"puts\", link: \"c\")\nfun g(_ text: *u8) -> i32\n";
    assert_eq!(
        diagnostics(source),
        [
            "1:1: `@extern` needs an ABI as its first argument, such as `.c`",
            "3:9: `@extern` ABI must be `.c`",
            "5:21: `symbol:` takes a string literal",
            "7:13: `callconv:` is not supported",
            "9:13: unknown `@extern` argument `name:`",
            "12:15: `string` values are not supported",
            "12:28: `char` has no C representation",
            "12:37: `char` has no C representation",
        ]
    );
    let typed = passes("@extern(.c, symbol: \"puts\", link: \"c\")\nfun g(_ text: *u8) -> i32\n");
    assert_eq!(
        typed.functions[0].foreign,
        Some(Foreign {
            symbol: "puts".to_string(),
            link: Some("c".to_string())
        })
    );
}

#[test]
fn main_and_declaration_rules() {
    assert_eq!(
        diagnostics(
            "fun main(_ argument: i32) -> i32 { return 0 }\nfun twice() {}\nfun twice() {}\n@inline\nfun fast() {}\nfun late(_ a: i32 := 1) {}\n"
        ),
        [
            "1:1: `main` takes no parameters",
            "1:1: `main` returns unit",
            "3:1: `twice` is already declared",
            "4:1: attribute `@inline` is not supported",
            "6:22: default arguments are not supported",
        ]
    );
}

#[test]
fn unsupported_types_and_expressions() {
    assert_eq!(
        diagnostics(
            "fun a(_ x: [i32], _ y: i32?, _ z: List<i32>, _ w: void) {}\nfun main() {\n    let s := \"text\"\n    let m := .member\n}\n"
        ),
        [
            "1:12: type `[i32]` is not supported",
            "1:24: type `i32?` is not supported",
            "1:35: generic types are not supported",
            "1:51: `void` is only allowed behind a pointer",
            "3:14: `string` values are not supported",
            "4:14: implicit members are not supported",
        ]
    );
}

#[test]
fn void_pointers_and_function_types() {
    let typed =
        passes("@extern(.c)\nfun free(_ pointer: *mut void)\nfun apply(_ f: fun(i32) -> i32) {}\n");
    assert_eq!(typed.functions[0].parameters[0].to_string(), "*mut void");
    assert_eq!(
        typed.functions[1].parameters[0].to_string(),
        "fun(i32) -> i32"
    );
}
