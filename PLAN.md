# `x` — Project Plan

The engineering roadmap for `x`, a modern systems programming language built from scratch.

**Companion documents:**
- **`LANGUAGE.md`** — authoritative language design (every locked syntactic and semantic decision)
- **`CLAUDE.md`** — project navigation, locked-at-a-glance, naming conventions

If you're resuming this work cold, read this file for *what we're building and how*, `LANGUAGE.md` for *what the code looks like*, and `CLAUDE.md` for *where things are and how design happens*.

---

## 1. Vision

`x` is a strongly-typed, performant, elegant systems language inspired by Rust, Swift, and Zig. It aims to be one language that spans:

- Operating systems and kernels (freestanding, no mandatory runtime)
- Native applications on Linux, Windows, and macOS
- Games (full C interop with OpenGL, SDL, etc.)
- An embeddable runtime, like Lua or JavaScript embed in other apps

Design priorities, in order:

1. **Performance.**
2. **Fun and elegant to write.**
3. **Self-host as quickly as possible.**

Source files use `.x`.

---

## 2. Locked decisions

### Engineering

| Decision | Choice | Why |
|---|---|---|
| **Bootstrap language** | Rust | Closest paradigm to `x` → easiest mechanical port at self-host. Mature LLVM bindings via `inkwell`/`llvm-sys`. Memory safety + ADTs + pattern matching make compiler code clean. |
| **LLVM access** | C API (`llvm-c`) | Stable, covers ~95% of compiler needs. Self-hosted `x` uses the same API. No C++ interop required. |
| **C++ interop** | Out of scope (long-term stretch) | C interop is a pillar. C++-only LLVM features handled via thin `extern "C"` shims. |
| **Architecture** | One frontend → shared IR → pluggable backends | LLVM AOT is the only backend until self-host; VM/JIT post-self-host, written in `x`. |
| **First backend** | LLVM AOT, native binaries (initially emit textual `.ll`, shell to `llc`) | Fastest path to a runnable language, zero LLVM linking up front. |
| **Scope strategy** | Minimal compiler-complete subset first | Just enough language to write a compiler; grow features in `x` after self-hosting. |
| **Dependencies** | LLVM only (for `x` itself) | The bootstrap compiler may use Cargo crates freely (it's throwaway). Everything else gets re-implemented in `x`. |
| **Self-hosting strategy** | Stage-by-stage rewrite + byte-equivalence cross-validation | Rewrite lexer first, then parser, etc.; bootstrap and `x` versions coexist during transition. |

### Language design (full detail in `LANGUAGE.md`)

| Area | Choice |
|---|---|
| Function keyword | `fun` (postfix modifiers: `fun mut`, `fun static`) |
| Type keyword | `type` (unified for records and enums — body shape decides) |
| Protocol keyword | `proto` (inline conformance via `:`; no extensions in v1) |
| Bindings | `let`, `let mut`; `:=` for bind/mutate; `=` for equality |
| Argument labels | Full Swift external/internal names (`_` to suppress) |
| Optional / Result | `T?`, `T!E` sugar; `?` propagation, `?.` chain, `??` coalesce; auto-wrap into `some(_)` / `ok(_)` |
| Pattern matching | `match` with guards (`pat if cond -> result`); `if` and `match` are expressions |
| Loops | `loop`, `loop until cond`, `loop x in iter`; `@name` for named loops; `break @name` / `continue @name` |
| Memory model | Ownership + moves + RAII; `Copyable`/`Droppable` protocols; `&T`/`&mut T` references (lifetimes **not** checked in v1); `*T`/`*mut T` raw pointers; `unsafe { }` blocks |
| Allocators | First-class `Allocator` protocol; explicit `in: alloc` named param with build-target default |
| Polymorphism | Generics `<T: P + Q>` + opaque types `some P`. No existentials in v1. Static dispatch only. |
| Operators | Boolean `& \| !` (short-circuit), bitwise `&& \|\| !!`, shifts `<< >>`. No custom operators; overloading via stdlib protocols (`Addable`, etc.). |
| Macros / comptime | Hybrid `@`/`#` split. `@name(...)` for declaration attributes (`@extern`, `@inline`, `@unsafe`, `@test`, `@deprecated`). `#name(...)` for verb macros (`#if`, `#match`, `#format`, `#asm`, `#panic`, `#assert`). |
| FFI | Unified `@extern(.c, link: ..., symbol: ..., callconv: ...)` for functions, opaque types, and locally-defined types with foreign layout. |
| Comments | `//` line, `///` doc (markdown). No `/* */`. |
| Lexical | No semicolons (newline-terminated); camelCase values; PascalCase types/protocols. |

### Out of scope for v1 (locked-not)

No type aliases. No inheritance. No extensions. No custom operators. No `++` / `--`. No force-unwrap. No ternary. No existential types. No implicit numeric conversion. No reference lifetime checking (deferred to v2 as opt-in).

---

## 3. Open / deferred decisions

Most are smaller items that don't gate Phase 0 or Phase 1. The full table lives in `LANGUAGE.md` §17.

| Topic | Status |
|---|---|
| Associated-type ergonomics | Locked in spirit (`<T>` slots on protocols, no `T.Item` accessor); use-site syntax needs revision before implementation. |
| Closure capture specifics | Escape detection, partial captures, capture-list shape. |
| Numeric cast `as` semantics | Widening/narrowing/checked behavior. |
| Module manifest | File format, directory layout, dependency declaration. |
| Visibility scope | `private` = file or module; type-field visibility. |
| Range syntax | `..`, `..=`, step? |
| `Hashable` signature | May evolve from `hashValue() -> u64` to streaming `hash(into: Hasher)`. |
| String escape grammar | Exact rules for `\x..`, `\u{...}`, etc. |
| Async / concurrency | Long-term. |
| Stdlib organization | Whole subject, post-language-design. |

---

## 4. Compiler architecture

```
                 source.x
                    │
                    ▼
                 ┌──────┐
                 │ Lex  │
                 └──┬───┘
                    ▼
                 ┌──────┐
                 │ Parse│
                 └──┬───┘
                    ▼
                 ┌──────┐
                 │ AST  │
                 └──┬───┘
                    ▼
              ┌───────────┐
              │ Typecheck │
              └─────┬─────┘
                    ▼
                ┌───────┐
                │ x-IR  │ ◄── single, shared, typed IR
                └───┬───┘
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
   ┌─────────┐ ┌────────┐ ┌──────────┐
   │LLVM AOT │ │  JIT   │ │bytecodeVM│
   │(native) │ │ (LLVM) │ │(embedded)│
   └─────────┘ └────────┘ └──────────┘
   ── phase 1 ── ── post-self-host ──
```

Single frontend produces a typed IR; backends are pluggable. Only LLVM AOT exists initially. JIT and bytecode VM are post-self-host work, written in `x` itself.

---

## 5. Phased roadmap

Each phase ends with a concrete, demonstrable artifact.

### Phase 0 — Project setup

- Initialize repo; `cargo new x-bootstrap`.
- LLVM access: **emit textual LLVM IR (`.ll`)** and shell out to `llc`/`clang` — zero LLVM linking. Switch to `inkwell` later when JIT/finer control is needed.
- Skeleton CLI: `x build <file>`, `x run <file>`.
- **Artifact:** `cargo run -- build empty.x` produces an empty (but valid) ELF/Mach-O/PE.

### Phase 1 — Hello world to native binary

Targets the smallest possible end-to-end loop: lex → parse → typecheck → emit `.ll` → link → run.

- Hand-written lexer (UTF-8, position tracking, comments).
- Recursive-descent parser for a tiny grammar: `fun` decls, integer/string literals, calls, basic operators, `@extern` attribute, `unsafe` blocks.
- AST with source spans.
- Minimal typechecker (just enough to reject obvious nonsense, validate `@extern` signatures).
- Codegen for: integer arithmetic, function calls, raw pointer arguments, string literal → `*u8` for FFI.

Target program — the actual file that has to compile and run:

```
@extern(.c)
fun printf(_ fmt: *u8, ...) -> i32

fun main() {
    unsafe {
        printf(c"hello, world\n")
    }
}
```

- **Artifact:** `x build hello.x && ./hello` prints `hello, world`.

### Phase 2 — Compiler-complete subset

The minimum needed to write a real compiler *in* `x`. Each item lands behind tests, with the language surface matching `LANGUAGE.md` exactly. See §6 below for the full subset inventory.

Target program — something representative of "the compiler can express compiler-shaped code":

```
type Token {
    ident(string)
    number(i64)
    plus
    minus
    eof
}

type LexError {
    unexpectedChar(char, usize)
    unterminated(usize)
}

fun lex(_ src: string) -> List<Token>!LexError {
    let mut out := List<Token>.new()
    let mut i: usize := 0
    loop until i >= src.byteCount {
        let c := src[i]
        match c {
            '+' -> { out.push(.plus); i := i + 1 }
            '-' -> { out.push(.minus); i := i + 1 }
            c if c >= '0' & c <= '9' -> {
                let start := i
                loop until i >= src.byteCount or !src[i].isDigit() {
                    i := i + 1
                }
                let n := src.slice(from: start, to: i).parseInt()?
                out.push(.number(n))
            }
            c if c.isWhitespace() -> i := i + 1
            _ -> return err(.unexpectedChar(c, i))
        }
    }
    out.push(.eof)
    return ok(out)
}
```

- **Artifact:** a non-trivial `x` program (JSON parser, small interpreter, the lexer above) compiles and runs.

### Phase 3 — Self-hosting

- Rewrite each compiler stage in `x`, stage by stage. Bootstrap and `x` versions coexist; cross-validation tests compare ASTs / IR / `.ll` output between the two.
- Final cutover: the Rust bootstrap compiles the `x` compiler; the `x` compiler compiles itself; verify the second-stage and third-stage outputs match byte-for-byte (or as close as feasible).
- Bootstrap is archived but kept for cross-compilation and platform bringup.
- **Artifact:** `x` compiles `x`. Self-hosted.

### Phase 4 — Language and ecosystem expansion (post-self-host)

These get built *in* `x`, exercising the language and shaking out bugs. Rough priority, not strict sequence:

- Opt-in lifetime checking (catches use-after-free without Rust's aliasing rules).
- Richer pattern matching (or-patterns, range patterns).
- User-definable macros (`@` and `#` forms).
- `@bindings(header: "...")` macro for auto-generating FFI bindings from C headers.
- Regex literals (`/pat/flags`).
- Async / concurrency (model TBD).
- **JIT backend** via LLVM ORC.
- **Bytecode VM backend** for embedded use.
- Tooling: formatter, language server, package manager, debugger integration.
- Platform support: Linux/Win/Mac all primary; embedded later.

---

## 6. The minimal subset, at a glance

What must exist before self-hosting is attempted. See `LANGUAGE.md` for syntax detail; section references in parentheses.

- **Types and literals** (§3, §6) — all primitives; records and enums via `type`; `[T; N]`, `[T]`, `T?`, `T!E`; generic types; references and raw pointers. (No tuples.)
- **Functions** (§5) — `fun` declarations, full Swift-style argument labels, default args, generics with bounds, closures and trailing-closure form, the three method modifiers (`fun`, `fun mut`, `fun static`).
- **Control flow** (§9) — `if` / `else` and `match` (with guards) as expressions; `loop` / `loop until` / `loop x in iter`; `guard ... else`; named loops via `@name`.
- **Optional & Result** (§10) — `T?` and `T!E` sugar, the three operators (`?`, `?.`, `??`), one-level auto-wrap (ok-side only).
- **Protocols** (§7, §11) — `proto` declarations with inline conformance and refinement; protocol `<T>` slots; the operator-protocol family (`Addable`, `Comparable`, etc.); `Displayable` and `Debugable`; compiler synthesis for `Equatable`, `Hashable`, `Comparable`, `Copyable`, `Droppable`, `Debugable`, `Default`.
- **Memory** (§16) — ownership + moves, RAII via `Droppable`, `&T` / `&mut T` (lifetimes unchecked in v1), raw pointers via `unsafe`, `Box<T>`, the `Allocator` protocol with `defaultAllocator` + override.
- **FFI** (§15) — `@extern(.c, ...)` covering functions, opaque types, and locally-defined C-layout types; `string.cstr()` for boundary conversion.
- **Macros** (§14) — `@`-attributes (`@extern`, `@inline`, `@deprecated`, `@test`, `@unsafe`) and `#`-verbs (`#if`, `#match`, `#format`, `#asm`, `#panic`, `#assert`).
- **Modules** (§13) — `import`, default-public visibility, `private` restrictor (manifest format TBD).

**Initial stdlib:** `List<T>`, `HashMap<K, V>`, `Set<T>`, `string`, `Box<T>`; `GeneralPurposeAllocator` plus `ArenaAllocator`, `PoolAllocator`, `FixedBufferAllocator`; libc-backed I/O for early bring-up.

**Out of scope before self-host:** advanced type inference, async, user-definable macros, reflection, regex literals, JIT, bytecode VM, opt-in lifetime checking, full standard library.

---

## 7. Risks and unknowns

- **Reference lifetimes unchecked in v1.** The biggest safety carve-out. Mitigated by RAII + idioms (arena + indices for graph data) + debug-mode runtime sanitizers (planned). v2 path: opt-in lifetime checking.
- **Self-hosting stage transitions.** Calling conventions, layout compatibility, miscompiles. Mitigation: aggressive byte-equivalence + behavioral testing between stages; transition lexer-first, then parser, then later stages.
- **LLVM API churn.** Pin a specific LLVM version (probably 17 or 18). The C API is more stable than C++ — extra reason it's the right target.
- **Associated-type ergonomics may need rework.** Locked in spirit but flagged for revision before implementation; if it turns out unworkable, the fallback is a separate `type Item` syntax inside protocols (Swift/Rust-style).
- **Scope creep.** Constant temptation to add features before self-hosting. **Discipline rule:** *if the bootstrap compiler can be written without it, it waits.*
- **Allocator-aware code is more verbose than GC.** Mitigated by `defaultAllocator`; revisit ergonomics if it becomes painful in real code.

---

## 8. What comes next

Language design is **complete enough to begin implementation.** The remaining open items (§3) don't block Phase 0 or Phase 1.

Immediate next steps:

1. **Phase 0 (project setup)** — `cargo new x-bootstrap`, skeleton CLI, `.ll` emission pipeline, an empty-binary test.
2. **Phase 1 (hello world)** — get the target program in §5 Phase 1 to compile and run.
3. **As Phase 2 approaches:** revisit associated-type ergonomics; firm up the module manifest; firm up `as` cast semantics. None block earlier work.

When iterating on language design in future sessions, **update `LANGUAGE.md` first** (it's authoritative), then `CLAUDE.md` at-a-glance, then this file if a phase or risk shifts.
