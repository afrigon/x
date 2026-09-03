# `x` — Language Design

The authoritative reference for what `x` looks like. Every decision below is **locked** unless explicitly marked open/deferred.

Companion docs: `PLAN.md` (engineering roadmap), `CLAUDE.md` (project navigation).

---

## 1. Overview

`x` is a strongly-typed, performant, elegant systems language inspired by **Rust, Swift, and Zig**. Priority order: **performance → elegance → speed to self-host**.

Targets: OS dev, native apps (Linux/Win/Mac), games (full C interop), embeddable runtime.

A sample to set the visual tone:

```
import math

type Point: Equatable, Hashable {
    mut x: f32
    mut y: f32

    fun static origin() -> Point {
        return Point(x: 0.0, y: 0.0)
    }

    fun magnitude() -> f32 {
        return (x*x + y*y).sqrt()
    }

    fun mut translate(by v: Vector) {
        x := x + v.x
        y := y + v.y
    }
}

type Token {
    ident(string)
    number(i64)
    plus
    minus
}

fun parseDigit(_ c: char) -> u8? {
    if c >= '0' & c <= '9' {
        return c - '0'          // auto-wraps to some(...)
    }
    return none
}

fun classify(_ tok: Token) -> string {
    return match tok {
        .ident(name) -> #format("identifier: {name}")
        .number(n)   -> #format("number: {n}")
        .plus        -> "+"
        .minus       -> "-"
    }
}
```

---

## 2. Lexical

- Source extension: `.x`
- Comments: `//` line, `///` doc (markdown body). **No `/* */`.**
- Newlines terminate statements. **No statement semicolons.** (`;` is still a token — it is the size separator in the array type `[T; N]`, its only use.)
- Identifiers: camelCase for values/functions; PascalCase for types/protocols.
- Keywords: `let`, `mut`, `fun`, `type`, `proto`, `static`, `import`, `if`, `else`, `match`, `guard`, `loop`, `until`, `in`, `break`, `continue`, `return`, `as`, `self`, `Self`, `true`, `false`, `private`, `unsafe`, `move`.

### 2.1 String and character literals

**Character literals** — single quotes, exactly one Unicode scalar: `'a'`, `'\n'`, `'\''`.

**Single-line strings** — double quotes, content on one line, type `string`: `"hello, world\n"`. Escapes are decoded; a raw newline before the closing `"` is an error.

**Byte strings** — `b"..."`, type `[u8; N]`. Source content must be ASCII; use escapes for other bytes. `\xHH` reaches the full **0x00–0xFF**, and `\u{...}` is **not** allowed (a byte string is bytes, not text).

**C strings** — `c"..."`, a NUL-terminated byte sequence for FFI (complements `string.cstr()`, §15.3). Non-ASCII content is UTF-8 encoded; `\xHH` (0x00–0xFF) and `\u{...}` are both allowed. A NUL in any form — `\0`, `\x00`, `\u{0}`, or a raw NUL — is an error (the terminator is implicit). Raw `b`/`c` strings (`br"..."`, `cr"..."`) are deferred; for raw multiline text use the `\\` form.

**Multiline strings** — a quote-less, line-prefixed form (à la Zig), type `string`. Consecutive lines that begin — after optional leading whitespace — with `\\` form a single string. The text after each `\\` is taken **verbatim** (raw — no escape processing), lines join with `\n`, and there is no trailing newline. The string ends at the first line without a `\\` marker.

```
let usage := \\usage: x <command>
             \\  build   compile a file
             \\  run     run a file
```

≡ `"usage: x <command>\n  build   compile a file\n  run     run a file"`. Leading whitespace before `\\` is ignored (align markers to the surrounding code); a blank output line is a bare `\\`. Because the content is raw, multiline strings also serve as **raw strings** — a one-line `\\(\d+)\.txt` needs no escaping.

**Escapes** apply to every quoted form (`"..."`, `'...'`, `b"..."`, `c"..."`) — never to the raw `\\` multiline form. The rules follow Rust's:

| Escape | Meaning |
|---|---|
| `\n` `\r` `\t` `\0` | newline, carriage return, tab, NUL |
| `\\` `\"` `\'` | backslash, double quote, single quote |
| `\xHH` | exactly 2 hex digits — **0x00–0x7F** in `"..."`/`'...'`, **0x00–0xFF** in `b"..."`/`c"..."` |
| `\u{H…}` | Unicode scalar — 1–6 hex digits (underscores allowed between digits), ≤ 0x10FFFF, not a surrogate; not allowed in `b"..."` |

Two Rust escape behaviors are deliberately omitted: a literal newline inside `"..."` (use the `\\` multiline form) and string continuation (`\` before a newline) — both are errors here. Interpolation is never implicit — use `#format` (§14.2).

---

## 3. Primitives

| Type | Meaning |
|---|---|
| `i8` `i16` `i32` `i64` | Signed integers |
| `u8` `u16` `u32` `u64` | Unsigned integers |
| `usize` `isize` | Platform pointer-sized (TBD details) |
| `f32` `f64` | IEEE 754 floats |
| `bool` | Boolean (`true` / `false`) |
| `char` | Unicode codepoint |
| `string` | UTF-8 + length (no NUL terminator) |

Default literal types when unconstrained: `i32` for integers, `f64` for floats.

---

## 4. Bindings

```
let x := 5                   // immutable, type inferred as i32
let mut count := 0           // mutable
count := count + 1           // mutation uses :=
let pi: f64 := 3.14159       // explicit type
```

Rules:
- `let` and `let mut` are the only ways to introduce a binding.
- `:=` binds a value to a name — used in **all** declarations, mutations, default args, field defaults, type-level bindings.
- `=` is the **equality operator** in expression position only.
- `!=` is inequality.
- Type annotations are optional when inference works.
- Newlines terminate statements; no semicolons.

---

## 5. Functions

```
fun add(_ a: i32, b: i32) -> i32 {
    return a + b
}

fun greet(name: string) {        // no return type = returns unit
    sendNotification("Hello, " + name)
}

add(3, b: 5)
greet(name: "Alice")
```

- `fun` is the function keyword.
- Argument labels: Swift-style external/internal names.
  - `_ name: T` — no external label; call is positional.
  - `external internal: T` — different names for caller vs callee.
  - `name: T` — same name for both.
- Parameters require type annotations.
- Return type required for non-unit returns; omit `->` for unit.
- `return` is explicit at function-body scope.
- Default arguments: `fun foo(x: i32 := 5)`.

### 5.1 Method modifiers (postfix on `fun`)

| Form | Meaning | Receiver | Called as |
|---|---|---|---|
| `fun name(...)` | Instance method | implicit `self`, read-only | `instance.name(...)` |
| `fun mut name(...)` | Mutating instance | implicit `self`, mutates | `instance.name(...)` — requires `mut` binding |
| `fun static name(...)` | Type-level | no self | `Type.name(...)` |

`self` is implicit; write `x` not `self.x` (use `self.` only for disambiguation).

### 5.2 Overloading

Functions and methods are identified by **name + ordered labels + parameter types**. Distinct signatures coexist; ambiguity is an error. **Exact match beats auto-wrap** (e.g., `T` overload preferred over `T?` overload).

### 5.3 Closures

Anonymous functions use Swift-style braces with an `in` marker between params and body.

```
let add := { (a: i32, b: i32) -> i32 in a + b }    // full annotation
let double := { x in x * 2 }                       // types inferred
let f: () -> () := { handle() }                    // zero-arg, no `in` (context says closure)
let g := { () in handle() }                        // zero-arg, explicit form

items.map { x in x * 2 }
items.sort { a, b in a.compare(b) }
items.fold(initial: 0) { acc, x in acc + x }

items.map { x in
    let y := x * 2
    return y + 1
}
```

Rules:

- **`in` is mandatory when there are parameters** (`{ x in body }`, `{ a, b in body }`). For zero-arg closures, both forms work: `{ () in body }` and `{ body }`.
- **Closure vs. block expression** (when `in` is absent): the compiler picks based on the **expected type at that position**. If a function type is expected (function argument, function-typed binding annotation), `{ body }` is a zero-arg closure. Otherwise it's a block expression. At a bare `let x := { ... }` with no type annotation, the default is block expression.
- **Argument arity must match.** A zero-arg `{ body }` cannot be passed where a one-arg closure is expected. Use `{ _ in body }` to accept-and-ignore.
- **No implicit positional args** — there is no `$0` / `$1`. Every parameter must be named (use `_` to discard).
- **Function types** are written `(T1, T2) -> R`. Closures and named `fun`s share the same type system — interchangeable wherever the type matches.

```
let f: (i32) -> i32 := { x in x * 2 }
fun apply(_ f: (i32, i32) -> i32, _ a: i32, _ b: i32) -> i32 { return f(a, b) }
```

#### Trailing-closure form

When a call's **last argument** is a closure, the closure may be written outside the closing paren.

```
items.map { x in x * 2 }                            // parens omitted (only arg)
items.fold(initial: 0) { acc, x in acc + x }        // last arg is trailing
button.onClick { () in handle() }
```

Constraints:
- **At most one trailing closure per call.**
- The trailing closure must be the **last argument** (multi-trailing-closure forms à la Swift 5.3+ are rejected).
- **Parens may be omitted entirely** when the trailing closure is the only argument.

#### Capture semantics

Closures capture surrounding bindings. The exact rules (reference vs value capture, mutability propagation, explicit captures) depend on the memory model — see §16.

---

## 6. Types: records and enums

`type` is the unified type-introducing keyword. **Body shape determines kind**:

```
type Point {                  // record (has fields)
    x: f32
    y: f32
}

type Color {                  // enum (has variants)
    red
    green
    blue
}

type Token {                  // enum with payloads
    ident(string)
    number(i64)
    plus
    minus
}

type Maybe<T> {               // generic enum
    some(T)
    none
}
```

Body item rules:
- `name: T` → **immutable field**. `mut name: T` → mutable field.
- Bare `name`, `name(T, ...)`, or `name := value` → **enum variant**.
- `fun ...` → method (allowed on either kind).
- **Mixing fields with variants is a compile error.**
- Empty body `type Unit {}` is a unit (empty record).

Variant reference: `.variantName` when the enum type is known from context.

### 6.1 Inline enum form

```
type Color { red, green, blue }
```

### 6.2 Memberwise initializer

Every record auto-generates `TypeName(field1: ..., field2: ...)`. Custom constructors are `fun static` returning the type:

```
type Point {
    x: f32
    y: f32

    fun static origin() -> Point {
        return Point(x: 0.0, y: 0.0)
    }
}

let p := Point.origin()
let q := Point(x: 1.0, y: 2.0)
```

### 6.3 No type aliases

Every `type` introduces a fresh nominal type. To wrap an existing type, declare a new record:

```
type UserId { value: u64 }    // distinct from u64
```

Built-in type constructors (`[T]`, `[T; N]`, `T?`, `T!E`, `List<i32>`) are **not** user-defined aliases — they're language primitives. (There are no tuples; group multiple values in a record or return a `Result`.)

### 6.4 No inheritance

Polymorphism comes from protocols + composition. No `extends`, no parent-child types.

---

## 7. Protocols

```
proto Drawable {
    fun draw()                            // required
    fun area() -> f32                     // required

    fun describe() -> string {            // default implementation
        return "Drawable, area " + area().display()
    }
}
```

- `Self` refers to the conforming type.
- Refinement: `proto Comparable: Equatable { ... }` — conformance to Comparable requires Equatable.
- Conformance is declared **inline in the type's own declaration**, comma-separated:

```
type Point: Drawable, Equatable {
    x: f32
    y: f32

    fun draw() { /* ... */ }
    fun area() -> f32 { return 0.0 }
    // equals auto-synthesized (Equatable is compiler-synthesizable)
}
```

**No extensions in v1.** A type's complete protocol conformance set is fixed at its declaration site.

### 7.1 Compiler-synthesized protocols

For these protocols, the compiler generates field-wise implementations automatically when all fields conform:

- `Equatable` — field-wise equality
- `Hashable` — field-wise hash combine
- `Comparable` — lexicographic by declaration order

If any field doesn't conform, the compile error names the offending field. All other protocols require manual implementation in v1.

---

## 8. Generics and opaque types

### 8.1 Generic functions

```
fun maxOf<T: Comparable>(_ a: T, _ b: T) -> T {
    return if a.compare(b) = .greater { a } else { b }
}

// Multiple bounds on one param with `+`
fun render<T: Drawable + Comparable>(_ items: [T]) {
    let sorted := items.sorted()
    for item in sorted { item.draw() }
}

// Multiple params, separated by `,`
fun pair<T: Drawable, U: Hashable>(_ a: T, _ b: U) { /* ... */ }
```

### 8.2 Opaque types: `some Proto`

A concrete-but-hidden type. The caller sees the protocol; the compiler picks one concrete type per call site (monomorphized).

```
fun makeShape() -> some Drawable {
    return Circle(radius: 1.0)
}

fun render(_ x: some Drawable) {        // sugar for <T: Drawable>(_ x: T)
    x.draw()
}
```

Allowed in: argument position, return position. Not allowed in: struct fields, local bindings without an initializer, container element types.

### 8.3 No existentials in v1

`Proto` cannot be used directly as a type position. For heterogeneous collections, use enums (closed-world variants):

```
type Shape: Drawable {
    circle(Circle)
    square(Square)

    fun draw() {
        match self {
            .circle(c) -> c.draw()
            .square(s) -> s.draw()
        }
    }
}

let scene: [Shape] := [.circle(Circle(radius: 1.0)), .square(Square(side: 2.0))]
```

**All dispatch in v1 is static (monomorphized).**

---

## 9. Control flow

### 9.1 `if` / `else`

`if` is an **expression** in expression position, a **statement** otherwise. Branches must produce the same type when used as expression.

```
let absVal := if n < 0 { -n } else { n }

if user.isAdmin {
    grantAccess()
} else if user.isPending {
    waitForApproval()
} else {
    deny()
}
```

### 9.2 `match`

```
match token {
    .ident(name) -> "ident: " + name
    .number(n)   -> "number"
    .plus        -> "+"
    .minus       -> "-"
}

let label := match score {
    s if s >= 90 -> "A"
    s if s >= 80 -> "B"
    s if s >= 70 -> "C"
    _            -> "F"
}
```

- Patterns: literals, bindings, wildcard `_`, struct destructure, enum destructure (`.variant(pattern, ...)`).
- Guards: `pattern if expr -> result`.
- Arms separated by newlines; arm body is an expression or a `{ ... }` block.
- Match expressions require exhaustiveness (or `_` wildcard).

### 9.3 `guard`

Early-return form. The `else` block **must end in a divergent statement** (`return`, `break`, `continue`, `@panic(...)`).

```
fun divide(_ a: i32, by b: i32) -> i32? {
    guard b != 0 else { return none }
    return some(a / b)
}

fun process(_ opt: User?) -> string!IoError {
    guard let user := opt else { return err(.notFound) }
    // `user` is the unwrapped User from here on
    return ok(user.name)
}
```

### 9.4 Loops

One keyword (`loop`), three forms:

```
loop {                              // infinite — escape with break
    let cmd := readCommand()
    if cmd = .quit { break }
    process(cmd)
}

loop until count >= limit {         // conditional (terminate when true)
    count := count + 1
}

loop x in 0..10 {                   // iteration over a range
    process(x)
}

loop item in collection {           // iteration over anything iterable
    process(item)
}
```

`break` / `continue` to exit/skip innermost loop.

#### Named loops

> **Deferred — syntax under review, not in the bootstrap.** The `@name` label
> form below is provisional (it collides visually with `@attribute` macros) and
> is **not** implemented in the v1 bootstrap compiler. Plain `break`/`continue`
> (innermost loop only) are in; tagged `break @name` / `continue @name` wait
> until the labeled-loop syntax is locked.

```
loop @search row in grid {
    loop cell in row {
        if cell.matches(query) { break @search }
    }
}
```

Only named loops can be the target of a tagged `break`/`continue`.

### 9.5 Blocks as expressions

A `{ ... }` block in expression position evaluates to its **last expression** — no `return` keyword needed (which would exit the enclosing function instead).

```
let area := {
    let r := getRadius()
    3.14159 * r * r                 // block's value
}
```

---

## 10. Optionals and Results

Sugar:

| Sugar | Means | Constructors |
|---|---|---|
| `T?` | `Option<T>` | `some(v)`, `none` |
| `T!E` | `Result<T, E>` | `ok(v)`, `err(e)` |

### 10.1 Auto-wrapping

- `T` → `T?` (as `some(_)`) in known-target-type positions. **One level only.**
- `T` → `T!E` (as `ok(_)`) in known-target-type positions. Requires `T ≠ E`.
- `none` is polymorphic; type from context.
- `T?` → `T` is **never** automatic. Use match, `guard let`, `??`, or `.expect(...)`.
- The **error path is always explicit** — `err(.foo)`. Never auto-wrapped.

```
fun parseDigit(_ c: char) -> u8? {
    if c >= '0' & c <= '9' { return c - '0' }       // auto-wraps to some(_)
    return none                                      // polymorphic
}
```

### 10.2 Operators

| Operator | Position | Meaning |
|---|---|---|
| `?` | postfix | **Propagate** — early-return `none`/`err` from enclosing function |
| `?.` | infix | **Chain** — short-circuit to `none` if LHS is `none`. Option only. |
| `??` | infix | **Coalesce** — `optional ?? default` |

```
let title := user?.profile?.title ?? "Anonymous"

fun loadConfig() -> Config!IoError {
    let raw := readFile("config.txt")?          // propagates IoError
    return ok(parse(raw))
}
```

### 10.3 No force-unwrap

The `!` "give me the value or crash" operator is **banned**. For explicit runtime-checked unwrap, use the stdlib method `.expect("reason")` — verbose by design.

### 10.4 Multi-error pattern

When a system may return many errors at once, use `T!List<E>`:

```
fun validate(_ form: Form) -> Form!List<ValidationError> {
    let mut errs := List<ValidationError>.new()
    if !form.email.isValidEmail() { errs.push(.invalidEmail(form.email)) }
    if form.age < 0 { errs.push(.ageOutOfRange(form.age)) }
    if errs.isEmpty { return ok(form) }
    return err(errs)
}
```

The `Result` type itself stays single-error. Multi-error = "the error type is a collection."

---

## 11. Operators

| Operator(s) | Mechanism |
|---|---|
| `+ - * / %` | Arithmetic, via stdlib protocols |
| `=` | Equality (`Equatable`) — comparison only |
| `!=` | Inequality |
| `< <= > >=` | Ordering (`Comparable`) |
| `&  \|  !` | Boolean AND / OR / NOT (short-circuit for binary) |
| `&&  \|\|  !!` | Bitwise AND / OR / NOT |
| `^  ^^` | Boolean XOR / Bitwise XOR (boolean form rarely used) |
| `<<  >>` | Bitwise shifts |
| `+= -= *= /= %=` | Compound mutation (arithmetic) |
| `&= \|= ^=` | Compound mutation (boolean accumulators) |
| `&&= \|\|= ^^= <<= >>=` | Compound mutation (bitwise) |
| `:=` | Bind / mutate |
| `?  ?.  ??` | Option/Result operators |
| `as` | Numeric cast (semantics TBD) |
| `0..10`, `0..=10` | Range (full syntax TBD) |

**Precedence** (high to low) follows **Rust**, mapped onto `x`'s symbols by role:

| Tier | Operators (x symbols) | Notes |
|---|---|---|
| unary | `-` `!` `!!` | prefix |
| multiplicative | `*` `/` `%` | |
| additive | `+` `-` | |
| shift | `<<` `>>` | below additive (as in C/Rust, *not* Swift) |
| bitwise and | `&&` | |
| bitwise xor | `^^` | |
| bitwise or | `\|\|` | |
| comparison | `<` `<=` `>` `>=` `=` `!=` | one level, **non-associative** — `a < b < c` is an error |
| boolean and | `&` | short-circuit |
| boolean xor | `^` | |
| boolean or | `\|` | short-circuit |

This is C's shape with C's two known traps removed (as Rust does): the bitwise band sits **above** comparison — so `x && MASK = 0` is `(x && MASK) = 0`, not the C footgun — and comparisons are a single non-associative level rather than chainable. Boolean `&`/`^`/`\|` (logical, short-circuit) sit at the bottom, below comparison.

**`&` is dual-role** (consistent with C's `&`): binary `a & b` is boolean AND; unary `&value` will be the reference-of operator once the memory model lands. Position disambiguates.

- **Operator overloading goes through protocols.** Implement the protocol; the operator becomes available.
- **No custom operators** — you can only implement the operators above.
- **No `++` / `--`.** Use `x += 1`.
- **No ternary `?:`.** Use `if` as an expression.
- **No implicit numeric conversion.** `i32 + f64` is a type error; cast with `as`. Same for cross-type `=`.

### 11.1 Operator → protocol mapping

| Operator | Desugars to | Protocol |
|---|---|---|
| `a + b` | `a.add(b)` | `Addable` |
| `a - b` | `a.subtract(b)` | `Subtractable` |
| `a * b` | `a.multiply(b)` | `Multipliable` |
| `a / b` | `a.divide(b)` | `Divisible` |
| `a % b` | `a.modulo(b)` | `Modable` |
| `-a` | `a.negate()` | `Negatable` |
| `a = b` | `a.equals(b)` | `Equatable` |
| `a != b` | `!a.equals(b)` | `Equatable` |
| `a < b` | `a.compare(b) = .less` | `Comparable` |
| `a <= b` | `a.compare(b) != .greater` | `Comparable` |
| `a > b` | `a.compare(b) = .greater` | `Comparable` |
| `a >= b` | `a.compare(b) != .less` | `Comparable` |
| `a && b` | `a.bitAnd(b)` | `BitwiseAnd` |
| `a \|\| b` | `a.bitOr(b)` | `BitwiseOr` |
| `a ^^ b` | `a.bitXor(b)` | `BitwiseXor` |
| `!!a` | `a.bitNot()` | `BitwiseNot` |
| `a << n` | `a.shiftLeft(by: n)` | `Shiftable` |
| `a >> n` | `a.shiftRight(by: n)` | `Shiftable` |
| `a[i]` | `a.get(i)` | `Indexable<I, E>` |
| `a[i] := v` | `a.set(i, v)` | `MutableIndexable<I, E>` |

### 11.2 Core protocols

```
type Ordering { less, equal, greater }

proto Equatable {
    fun equals(_ other: Self) -> bool
}

proto Comparable: Equatable {
    fun compare(_ other: Self) -> Ordering
}

// Arithmetic — one method per operator, Self -> Self -> Self
proto Addable      { fun add(_ other: Self) -> Self }
proto Subtractable { fun subtract(_ other: Self) -> Self }
proto Multipliable { fun multiply(_ other: Self) -> Self }
proto Divisible    { fun divide(_ other: Self) -> Self }
proto Modable      { fun modulo(_ other: Self) -> Self }
proto Negatable    { fun negate() -> Self }

// Bitwise
proto BitwiseAnd { fun bitAnd(_ other: Self) -> Self }
proto BitwiseOr  { fun bitOr(_ other: Self) -> Self }
proto BitwiseXor { fun bitXor(_ other: Self) -> Self }
proto BitwiseNot { fun bitNot() -> Self }
proto Shiftable {
    fun shiftLeft(by amount: u32) -> Self
    fun shiftRight(by amount: u32) -> Self
}

// Indexing
proto Indexable<I, E> {
    fun get(_ index: I) -> E
}
proto MutableIndexable<I, E>: Indexable<I, E> {
    fun mut set(_ index: I, _ value: E)
}

// Hashing
proto Hashable: Equatable {
    fun hashValue() -> u64
}
```

**Why `compare` returns `Ordering`:** one comparison call covers all four orderings; the compiler derives `<`, `<=`, `>`, `>=` uniformly. More efficient than four separate bool-returning methods.

`Hashable`'s `hashValue() -> u64` signature is the v1 form. May evolve to a streaming `hash(into: Hasher)` once references/mut params from the memory model land.

### 11.3 Bundles via refinement

```
proto Numeric:
    Addable, Subtractable, Multipliable, Divisible, Modable,
    Negatable, Comparable
{
    fun static zero() -> Self
    fun static one()  -> Self
}

proto Bitwise: BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, Shiftable {}

proto Integer: Numeric, Bitwise, Hashable { /* future: bit_count, leading_zeros, ... */ }

proto FloatingPoint: Numeric {
    fun static nan() -> Self
    fun static infinity() -> Self
    fun isNaN() -> bool
    fun isFinite() -> bool
}
```

Built-in conformances:
- `i8`…`i64`, `u8`…`u64` : `Integer`
- `f32`, `f64` : `FloatingPoint`
- `bool` : `Equatable + Hashable`
- `char` : `Equatable + Hashable + Comparable`
- `string` : `Equatable + Hashable + Comparable + Displayable + Debugable`

### 11.4 Other stdlib protocols (not operators, but the menu)

```
proto Default {
    fun static default() -> Self
}

proto Displayable {
    fun display() -> string              // user-facing string form — used wherever a value is rendered as text (e.g., string interpolation)
}

proto Debugable {
    fun debug() -> string                // diagnostic string form — auto-synthesizable; used wherever internal structure should be shown
}

proto Iterator<Item> {
    fun mut next() -> Item?
}

proto Iterable<Item> {
    fun iterator() -> some Iterator<Item>
}
```

`loop x in iterable { ... }` desugars to:

```
let mut __it := iterable.iterator()
loop {
    match __it.next() {
        some(x) -> { /* body */ }
        none    -> break
    }
}
```

`Iterator`/`Iterable` are generic protocols in v1 (because associated types are deferred). The surface stays the same when they migrate to associated types post-self-host.

### 11.5 Compiler synthesis (updated full list)

The compiler auto-generates implementations when **all fields conform**:

- `Equatable` — field-wise equality
- `Hashable` — field-wise hash combine
- `Comparable` — lexicographic by declaration order
- `Default` — field-wise `default()` per field
- `Debugable` — field-wise dump in `TypeName { field: value, ... }` form

`Displayable` is **not** auto-synthesized — user-facing string output is intentional.

Other protocols require manual implementation in v1.

### 11.6 Compound assignment

`a OP= b` requires `a` to be a `mut` binding and the type to conform to the corresponding protocol. Desugars to `a := a.OP(b)` semantically (compiler may optimize in-place). Full set:

`+= -= *= /= %= &= |= ^= &&= ||= ^^= <<= >>=`

### 11.7 What is NOT user-overloadable

- **Boolean `&`, `|`, `!`, `^`** — work only on `bool`. Custom types cannot pretend to be boolean.
- **Assignment `:=`** — language operator, not a method call.
- **Range `..`, `..=`** — produces `Range` / `ClosedRange` (stdlib types); the syntax is built-in.
- **`?`, `?.`, `??`** — Option/Result operators, built-in.
- **Postfix `!`** — banned (no force-unwrap).

---

## 12. Arrays and collections

| Form | Meaning |
|---|---|
| `[T; N]` | Fixed-size array; `N` is a compile-time integer |
| `[T]` | Slice — non-owning `{ptr, len}` view |
| `List<T>` | Stdlib dynamic growable list |

```
let xs: [i32; 3] := [1, 2, 3]
let zeros: [i32; 100] := [0; 100]       // size 100, all zeros (repeating literal)
let view: [i32] := xs[..]
let dyn := List<i32>.new()
dyn.push(42)
```

Other collections (`HashMap<K, V>`, `Set<T>`, etc.) live in the stdlib.

---

## 13. Modules and visibility

- **No `module` keyword.** Modules are defined by **manifest + directory** (format TBD).
- Files within a module share visibility freely.
- **Default visibility is public** (exported across module boundaries).
- `private` is the only restriction modifier (scope TBD — file vs module).
- `import name` brings non-private items into scope.
- Name clashes resolve by qualifying with the module name:

```
import moduleA
import moduleB

functionA()                  // unambiguous
moduleA.functionAB()
moduleB.functionAB()
```

---

## 14. Compile-time macros

Compile-time directives split by role:

- **`@name(...)` — attribute macros.** Decorate a declaration (function or type). They don't evaluate to a value; they modify how the declaration is treated.
- **`#name(...)` — verb macros.** Appear in expression or statement position. Produce a value, emit code, or generate declarations.

All macro arguments must parse as valid `x` expressions/calls/literals — the lexer and parser don't need special cases per macro. Compiler-builtin macros in v1; user-definable macros come post-self-host with the same syntax.

### 14.1 Built-in macros

**Attribute macros (`@`)** — decorate declarations:

| Macro | Purpose |
|---|---|
| `@extern(abi, ...)` | FFI — function, opaque type, or local type with foreign layout |
| `@inline` / `@inline(.always)` / `@inline(.never)` | Inlining hints |
| `@deprecated("msg")` | Deprecation marker |
| `@test` | Test marker on a function |
| `@unsafe` | Mark a function as requiring an `unsafe` block at call sites |

**Verb macros (`#`)** — expression or statement position:

| Macro | Purpose |
|---|---|
| `#if(cond) { decls }` | Conditional declaration block |
| `#match(value) { ... }` | Match-form conditional declarations |
| `#format("...")` | String formatting; returns `string` |
| `#asm(...)` | Inline assembly |
| `#panic("msg")` | Panic / abort with a (formatted) message |
| `#assert(cond, "msg")` | Runtime check with formatted message |

### 14.2 `#format`

String formatting. The format string is parsed at compile time; **all values come from inside the braces** — no positional args, no implicit args, no separate argument list.

```
let name := "Alice"
let age := 30
let pi := 3.14159

#format("Hello {name}, age {age}")
#format("x = {x:?}")                          // ? = debug formatting via Debugable
#format("hex: {num:08x}")                     // format spec after `:` (Rust grammar)
#format("pi ≈ {pi:.2}")                       // .2 = two decimals
#format("sum: {a + b}")                       // expression inside braces
#format("nested: {user.profile.name}")        // chained access
#format("escaped: {{ and }}")                 // literal { and } via doubling
```

Placeholder grammar inside `{...}`:
- An expression evaluable in scope.
- Optional `:spec` follows: `?` means debug formatting (via `Debugable.debug()`); otherwise display formatting (via `Displayable.display()`) with the spec (width, fill, alignment, precision, base, etc., following Rust's grammar).

Escape literal braces with `{{` and `}}`.

String literals (`"..."` and the `\\` multiline form, §2.1) are *not* interpolated — they are exactly the text you wrote (single-line literals after escape decoding; multiline literals verbatim). Interpolation is **always** explicit via `#format`.

### 14.3 `#if`

Conditional compilation at declaration scope:

```
#if(target.os = .linux) {
    @extern(.c, link: "c") fun clock_gettime(_ clkid: i32, _ tp: *Timespec) -> i32
}

#if(target.arch = .x86_64) {
    // x86_64-specific declarations
}
```

Inside function bodies, plain `if` works — the compiler elides dead branches when conditions are statically known. `#if` is for declaration-level gating or to force comptime evaluation.

### 14.4 `#asm`

Inline assembly:

```
fun syscall1(_ num: i64, _ arg: i64) -> i64 {
    let mut result: i64 := 0
    #asm(
        "syscall",
        in("rax", num),
        in("rdi", arg),
        out("rax", &result),
        clobbers("rcx", "r11")
    )
    return result
}
```

`in`, `out`, `clobbers` are not real functions — they're tokens the `#asm` macro interprets as operand specifications. Every argument parses as regular `x` syntax (string literals, function-call expressions).

Operand model details (register allocation, dialect, side effects) are TBD.

---

## 15. FFI

Unified under the `@extern` macro. Three contexts depending on what it's attached to:

```
// (1) Foreign function — symbol exists in C, we declare the signature
@extern(.c)
fun printf(_ fmt: *u8, ...) -> i32

@extern(.c, link: "user32", symbol: "MessageBoxA", callconv: .stdcall)
fun MessageBox(_ hwnd: *void, _ text: *u8, _ caption: *u8, _ kind: u32) -> i32

// (2) Foreign opaque type — declared elsewhere, only held via pointer
@extern(.c)
type FILE

// (3) Locally-defined type with foreign layout
@extern(.c)
type Timespec {
    tv_sec:  i64
    tv_nsec: i64
}
```

ABI tags supported in v1: `.c`. Future: `.rust`, `.cpp` (deferred — require ABI knowledge / name mangling).

### 15.1 Variadic args

`...` in extern function signatures: `fun printf(_ fmt: *u8, ...) -> i32`.

### 15.2 Function pointer types

First-class type: `fun(T...) -> R`. Usable directly as a value type for callbacks.

### 15.3 String boundary

`x`'s `string` is UTF-8 + length, NOT NUL-terminated. Convert at FFI boundary:
- `string.cstr()` → `*u8` (NUL-terminated, lifetime tied to source).
- `cstr.toString()` decodes a `*u8` into `string`.
- A `c"..."` literal (§2.1) is a NUL-terminated byte sequence built at compile time — pass it directly where C wants a `*u8`, no runtime conversion.

### 15.4 C preprocessor macros

NOT interoperable. Wrap on the C side with a real function and bind to that.

### 15.5 Future: header-binding macro

Once user-definable macros land, a `@bindings(header: "stdio.h", link: "c")` macro can auto-generate `@extern` declarations from a C header at compile time — same model as Zig's `@cImport`. The language stays small; ergonomics grow through macros.

---

## 16. Memory model

How values are owned, moved, dropped, borrowed, and allocated. v1 design; some items have an explicit evolution path (§16.13). A few details are intentionally deferred to §17 (closure-capture specifics, `Hashable` signature evolution).

### 16.1 The model in one paragraph

Every value has exactly one **owner** (a binding). When the binding goes out of scope, the value's destructor runs (RAII) and any heap memory it owns is freed via the allocator it remembers. Ownership **moves** on assignment or pass; sources of moves are statically invalidated. Trivially-small types are **`Copyable`** and copy instead of moving. To pass a value without transferring ownership, use a **reference** (`&T` shared, `&mut T` exclusive) — non-owning, lifetime **not statically checked in v1**. **Raw pointers** (`*T`, `*mut T`) exist for FFI and manual memory and require an `unsafe` block. **Allocators** are first-class values; stdlib heap types take an allocator as a named parameter with a build-target default.

### 16.2 Ownership and moves

```
let s := string.new("hello")    // s owns the buffer
let t := s                       // ownership moves to t; s is invalidated

doSomething(s)                   // ERROR: use of moved value `s`
doSomething(t)                   // OK
                                 // at end of scope, t.drop() runs, buffer freed
```

Moves are zero-cost: a bitwise copy of the value with the source statically invalidated. The compiler refuses to read from a moved-from binding. Passing a non-`Copyable` value to a function transfers ownership; to lend without moving, pass a reference (§16.5).

### 16.3 `Copyable`

```
proto Copyable {}                       // marker — no methods
```

A type conforming to `Copyable` is copied on assignment/pass instead of moved.

- **Auto-synthesized:** a type is `Copyable` iff all fields are `Copyable`.
- **All primitives are `Copyable`:** `i8`–`i64`, `u8`–`u64`, `f32`, `f64`, `bool`, `char`, function pointers, all references, all raw pointers, enums whose payloads are all `Copyable`.
- **Heap-owning types are NOT `Copyable`:** `string`, `List<T>`, `Box<T>`, `HashMap<K, V>`, etc. Copying would be semantically ambiguous (deep vs shallow). Use an explicit `.clone()` method on those.
- **`Copyable` and `Droppable` are mutually exclusive.** Cleanup-on-drop + duplicate-on-copy would produce two owners of the same resource.

```
type Point: Copyable {
    x: f32
    y: f32
}

let p := Point(x: 1, y: 2)
let q := p                       // copy, not move
use(p)                            // still valid
use(q)                            // also valid
```

### 16.4 `Droppable`

```
proto Droppable {
    fun mut drop()
}
```

When an owner of a droppable value goes out of scope, `drop()` runs automatically (RAII).

- **Implicit recursive drop is always-on.** A type whose fields are themselves droppable gets a synthesized recursive `drop()` even without an explicit `: Droppable` declaration. Owners always run cleanup on their owned data.
- **Conform to `Droppable` explicitly only when you need custom cleanup** (closing a file descriptor, releasing a lock). Your manual `fun mut drop()` *replaces* the synthesized field-wise drop — call any sub-drops yourself if needed.
- **Fields drop in reverse declaration order** (matches construction order in reverse — last in, first out).

```
type FileHandle: Droppable {
    private fd: i32

    fun static open(_ path: string) -> FileHandle!IoError { /* ... */ }

    fun mut drop() {
        unsafe { libc.close(fd) }
    }
}

{
    let h := FileHandle.open("data.txt")?
    use(h)
}   // h.drop() runs here automatically
```

### 16.5 References — `&T` and `&mut T`

A reference is a non-owning view of a value owned elsewhere.

```
fun length(_ s: &string) -> usize {
    return s.byteCount                   // auto-deref on field access and method call
}

fun mut translate(_ p: &mut Point, by v: Vector) {
    p.x := p.x + v.x
    p.y := p.y + v.y
}

let s := string.new("hello")
let n := length(&s)                      // s still owns the buffer; pass a ref
let mut origin := Point(x: 0, y: 0)
translate(&mut origin, by: Vector(x: 1, y: 0))
```

- **`&value`** — shared (read-only) reference.
- **`&mut value`** — exclusive (mutable) reference; the source binding must be `mut`.
- **Auto-deref:** in expressions, `&T` and `&mut T` behave like `T` for field access, method calls, and operators.

#### v1 caveat — lifetimes are not statically checked

A reference can outlive its referent (use-after-free), and the compiler will not stop you. This is the conscious v1 trade-off: skip the borrow checker entirely, save a year of compiler work, accept that this class of bug is on the programmer.

Mitigations available in v1:
- RAII makes most lifetimes deterministic and visible (drop happens at end of scope).
- Debug-mode runtime sanitizers (planned).
- Idiomatic patterns — for graph-shaped data, prefer arena + indices (§16.12).

Planned v2 evolution: opt-in lifetime checking that catches use-after-free *without* imposing Rust-style aliasing rules. See §16.13 and §17.

### 16.6 Raw pointers — `*T` and `*mut T`

For FFI, manual memory, and assembly. Raw pointers can be null, can dangle, and the compiler does not help you.

| Form | Meaning |
|---|---|
| `*T` | Possibly-null pointer to immutable `T` |
| `*mut T` | Possibly-null pointer to mutable `T` |
| `*void` | Untyped pointer (C `void*`) |

All raw-pointer operations require an `unsafe` block (§16.10).

```
@extern(.c) fun malloc(_ size: usize) -> *void
@extern(.c) fun free(_ ptr: *void)

unsafe {
    let p := malloc(1024) as *mut u8
    p[0] := 42
    free(p as *void)
}
```

`as` performs raw casts inside `unsafe` (pointer casts, numeric-to-pointer, etc.).

### 16.7 `Allocator`

```
proto Allocator {
    fun mut alloc(_ size: usize, align: usize) -> *void?
    fun mut free(_ ptr: *void, size: usize, align: usize)
    fun mut realloc(_ ptr: *void, oldSize: usize, newSize: usize, align: usize) -> *void?
}
```

Stdlib types that allocate are **generic over the allocator** (`<A: Allocator>`), monomorphized per concrete allocator type. No existentials required.

Stdlib allocator menu:

- **`GeneralPurposeAllocator`** — wraps platform `malloc`/`free` (libc, or your kernel's allocator).
- **`ArenaAllocator`** — bump-allocator; all allocations freed at once when the arena drops. Ideal for graph-shaped data and short-lived bursts.
- **`PoolAllocator`** — fixed-size slots; O(1) alloc/free; game entities, kernel objects.
- **`FixedBufferAllocator`** — allocates from a stack/static buffer; needs no heap; works at literal-`t=0` boot before any real allocator exists.
- **`PageAllocator`** — raw OS or kernel pages; bottom of the stack.

### 16.8 Default allocator + override

Heap stdlib types take the allocator as a **named parameter with a default**:

```
let list := List<i32>.new()                  // uses defaultAllocator
let map  := HashMap<string, i32>.new()       // uses defaultAllocator
let arena := ArenaAllocator.new(in: defaultAllocator)
let temp := List<i32>.new(in: arena)         // override per-allocation
```

`defaultAllocator` is per build target:
- **Userspace builds** — defaults to a libc-backed `GeneralPurposeAllocator`.
- **Bare-metal / freestanding builds** — unset by default; the program must initialize it before any heap-using stdlib call. Using it unset is a compile-time error.

This is what makes **bare-metal and userspace use the same stdlib**: identical code runs in both; only the `defaultAllocator` setup differs.

### 16.9 Closure captures

Closures capture surrounding bindings based on the binding's type and mutability:

```
let count := 5
let f := { x in x + count }              // count is i32 (Copyable) → captured by copy

let mut acc := 0
let g := { x in acc := acc + x }         // captured by &mut acc

let owned := loadConfig()
let h := move { x in process(owned, x) } // `move` transfers ownership into the closure
```

Rules:

- **`Copyable` bindings** → captured by copy.
- **Non-`Copyable` immutable bindings** → captured by `&T`.
- **Non-`Copyable` mutable bindings** → captured by `&mut T`.
- **`move { params in body }`** — prefix keyword that forces capture-by-value (ownership transfer) for non-Copyable captures.

Same v1 caveat as references: a closure that escapes the lifetime of its captures and is later invoked is undefined behavior. The compiler does not statically prevent this. The capture-rule specifics (e.g., escape detection, partial captures, capture lists Swift-style) are flagged in §17 for revision before implementation.

`move` is a contextual keyword that appears only immediately before a closure brace.

### 16.10 `unsafe` blocks and `@unsafe`

Two complementary mechanisms:

```
// Block form — opt into unsafe operations within a scope.
unsafe {
    let raw := malloc(size) as *mut u8
    raw[0] := 0
    free(raw as *void)
}

// Attribute form — mark a declaration as requiring an unsafe block at call sites.
@unsafe
fun rawAdvance(_ p: *mut u8, by offset: usize) -> *mut u8 {
    return (p as usize + offset) as *mut u8
}

unsafe {
    let q := rawAdvance(p, by: 8)        // @unsafe function call requires unsafe block
}
```

Operations that require `unsafe`:
- Dereferencing or writing through a raw pointer (`*p`, `p[i] := v`).
- Pointer casts (`x as *T`, `p as usize`, `n as *mut T`).
- Calling an `@extern` function (FFI is always unsafe).
- Calling an `@unsafe`-marked function.
- Inline assembly via `#asm(...)`.

`unsafe { expr }` is an **expression** that evaluates to whatever `expr` evaluates to; it just lifts the safety restriction in scope. Granularity is block-level, not expression-level — write `unsafe { ... }` around the whole region rather than tagging individual sub-expressions.

### 16.11 `Box<T>` and other owning containers

`Box<T>` is the stdlib type for "owned heap pointer to T." Used when a value needs a stable address (trees with parent refs — §16.12), or when recursive structures need indirection:

```
type Tree {
    value: i32
    left:  Box<Tree>?
    right: Box<Tree>?
}

let b: Box<i32> := Box.new(42)
let n := b.value                         // auto-deref
// b.drop() at end of scope; the heap allocation is freed.
```

Other owning containers shipping in the stdlib (long-term, not all v1):

- **`Rc<T>`** — reference-counted shared ownership (single-threaded).
- **`Arc<T>`** — atomic refcount, thread-safe shared ownership.
- **`Cell<T>` / `RefCell<T>`** — interior mutability behind an immutable reference, when needed.

v1 ships `Box<T>` and the core collections (`List<T>`, `string`, `HashMap<K, V>`, `Set<T>`). The rest follow as the stdlib evolves.

### 16.12 Working with trees and graphs

Three idiomatic patterns; pick by use case.

**A. Inline children with parent reference** (simplest)
```
type Node {
    value: i32
    parent: &mut Node?               // optional unchecked parent reference
    children: List<Node>
}
```
Works for prototypes and small static trees. Gotcha: if `children` reallocates (pushing past capacity), nodes move in memory and parent refs go stale. Mitigate via `children.reserve(n)`, or use approach B.

**B. Boxed nodes — stable addresses**
```
type Node {
    value: i32
    parent: &mut Node?
    children: List<Box<Node>>        // each node has its own stable address
}
```
Growing `children` reallocates the list-of-pointers, not the nodes themselves. Parent refs stay valid. Slightly more allocations.

**C. Arena + indices** (best for hot or large structures)
```
type NodeId { value: usize }

type Node {
    value: i32
    parent: NodeId?
    children: List<NodeId>
}

type Tree {
    mut nodes: List<Node>

    fun mut addNode(under parent: NodeId?, _ value: i32) -> NodeId {
        let id := NodeId(value: self.nodes.byteCount)
        self.nodes.push(Node(value: value, parent: parent, children: List<NodeId>.new()))
        return id
    }

    fun nodeAt(_ id: NodeId) -> &Node {
        return &self.nodes[id.value]
    }
}
```
All nodes live in one flat list; cross-references are integer indices. No dangling possible; cache-friendly. The pattern compilers use internally for ASTs and symbol tables.

### 16.13 Future evolution path

All additive — none breaks v1 code:

- **v2: opt-in lifetime checking** — non-mandatory analysis that catches dangling references at compile time *without* Rust's aliasing restrictions. Catches use-after-free; trees with parent refs still compile.
- **v3+: full borrow checker** — only if v2 proves insufficient.
- **Data-race protection** — `Send`/`Sync`-style analog for concurrency; long-term.
- **`Hashable` upgrade** — `hashValue() -> u64` may evolve to streaming `hash(into: Hasher)` once references and `mut` parameters are well-exercised.

---

## 17. Open / deferred decisions

These are explicitly unsettled and will be locked in future rounds.

| Topic | Status |
|---|---|
| **Memory model** | Direction settled (ownership + RAII + unchecked refs + explicit allocators + `unsafe`); awaiting lock-in section in `LANGUAGE.md` |
| **Closure capture semantics** | Reference vs value capture, mutability propagation, explicit-capture form |
| **Associated-type ergonomics** | **Locked in spirit** (protocols carry `<T>` slots, no `T.Item` accessor, bind explicitly at use). **Needs revision before implementation** — use-site syntax (`<I: Iterator<T>, T>`), constraint shorthand, slot defaults, and behavior in multi-bound contexts all need another pass. |
| **Numeric cast `as`** | Widening/narrowing/checked semantics, overflow behavior |
| **Async / concurrency** | Long-term |
| **Module manifest** | Format, directory layout, dependency declaration |
| **Visibility scope** | `private` = file or module; type-level field visibility |
| **Range syntax** | `..`, `..=`, step? |
| **`Hashable` signature** | May evolve from `hashValue() -> u64` to streaming `hash(into: Hasher)` |
| **User-definable macros** | Post-self-host |
| **Regex literal** | Post-self-host (`/pat/flags`) |
| **`@extern(.rust)` / `.cpp`** | Future ABIs |
| **Inline asm mechanics** | Operand binding model, dialects |
| **C `int`/`long`/`size_t`** | Stdlib aliases module |
| **Stdlib design** | Whole subject, post-language-design |
| **`@bindings` header macro** | Future macro (referenced in §15.5) — needs `@` → `#` re-classification when user macros land |
