# `x` — Project Overview

`x` is a strongly-typed, performant, elegant systems programming language being built from scratch. Inspired by Rust, Swift, and Zig. Designed for OS dev, native apps, games, and embeddable runtimes. Self-hosting is the early goal.

## Project Structure

- x: the self-hosted implementation of the compiler
- x-rust: The bootstrap rust implementation of the compiler
- x-lang: The promotional website for x

## Status

**Pre-implementation.** The language is being designed conversationally. No compiler code exists yet. The next implementation milestone is Phase 0 of `PLAN.md` (project scaffold + emit-`.ll`-via-`llc` pipeline), to be started once the language design is fully locked.

## Where to find things

- **`LANGUAGE.md`** — Authoritative language design spec. Syntax, type system, semantics, and every currently-locked surface decision. Always check here first when answering "what does `x` look like?"
- **`PLAN.md`** — Engineering roadmap. Phased milestones (Phase 0 setup → Phase 1 hello world → Phase 2 compiler-complete subset → Phase 3 self-host → Phase 4 ecosystem), compiler architecture, locked bootstrap/LLVM decisions. **Note:** the `Phase 1`–`Phase 2` syntax sketches in this doc predate the surface-design rounds and use placeholder syntax (`fn`, `Option<T>` etc.) — they'll be rewritten with locked `x` syntax once language design is complete.
- **`CLAUDE.md`** — This file. Project nav + working conventions.

## Working conventions

- Design happens via conversation. **Locks are written to `LANGUAGE.md`** once agreed; open questions are explicitly tabulated in `LANGUAGE.md`.
- The engineering plan (`PLAN.md`) will be revised to reflect locked syntax after the language design round completes.
- When proposing changes, reference the section number in `LANGUAGE.md` so updates land in one place.

## Code style

- **Always prefer full, descriptive names over abbreviations** — in code and in AST/type names alike: `Expression` not `Expr`, `formatter` not `f`, `character` not `c`, `declaration` not `decl`. Applies to types, fields, functions, and locals.
- **Exception: loop / tight-iteration variables**, where short names are fine (`i`, `for c in chars`, `loop x in items`).

## Naming conventions

- **Protocol names prefer the `-able` suffix when the protocol expresses a capability** ("a type that can be X-ed"): `Equatable`, `Hashable`, `Comparable`, `Displayable`, `Debugable`, `Addable`, `Subtractable`, `Multipliable`, `Divisible`, `Modable`, `Negatable`, `Shiftable`, `Indexable`, `Iterable`, `Droppable`, `Copyable`.
- **Drop the `-able` when it reads awkwardly or the name is a noun**:
  - Actor / role nouns: `Iterator`, `Allocator`, `Hasher`.
  - Bundle / category protocols: `Numeric`, `Integer`, `FloatingPoint`, `Bitwise`.
  - Bitwise operation protocols where `-able` would be ugly: `BitwiseAnd`, `BitwiseOr`, `BitwiseXor`, `BitwiseNot`.
  - Markers / specific abilities where the noun reads cleaner: `Default`.
- **Method names pair with the protocol** when natural: `Displayable.display()`, `Debugable.debug()`, `Droppable.drop()`, `Iterable.iterator()`, `Iterator.next()`, `Addable.add()`, `Negatable.negate()`, `Hashable.hashValue()`. Avoid `toX`-style method names except where they're universally recognized idioms (e.g., `toString` on a string-conversion helper).
- Marker protocols (no methods) use the capability noun directly: `Copyable {}`.

## Locked at a glance (full detail in `LANGUAGE.md`)

| Area | Choice |
|---|---|
| Bootstrap compiler | Rust |
| LLVM access | C API (`llvm-c`) |
| First backend | LLVM AOT, native binaries |
| Self-host strategy | Minimal compiler-complete subset first |
| Function keyword | `fun` (with postfix `mut` / `static` modifiers) |
| Type keyword | `type` (unified for records and enums — body shape decides) |
| Protocol keyword | `proto` |
| Binding | `let` / `let mut`; `:=` for bind/mutate; `=` for equality |
| Argument labels | Full Swift external/internal names |
| Optional / Result | `T?` / `T!E`; sugar over `Option`/`Result`; `?` `?.` `??` operators |
| Pattern matching | `match` with guards; `if` and `match` are expressions |
| Loops | One keyword: `loop`, `loop until`, `loop x in iter`; `@name` for named loops |
| Polymorphism | Generics `<T: P + Q>` + opaque types `some P`. No existentials in v1. |
| Macros / comptime | Hybrid `@`/`#` split — `@name(...)` attribute macros on declarations (`@extern`, `@inline`, `@unsafe`, `@test`, `@deprecated`); `#name(...)` verb macros in expression/statement position (`#if`, `#match`, `#format`, `#asm`, `#panic`, `#assert`) |
| FFI | Unified under `@extern(.c, link: ..., symbol: ..., callconv: ...)` |
| Comments | `//` line, `///` doc; no `/* */` |
| Other | No type aliases, no inheritance, no extensions (v1), no custom operators, no `++`/`--`, no force-unwrap, no ternary |

## What's open (key items)

- Module manifest format and `private` scope details
- Numeric cast (`as`) semantics
- Async / concurrency
- Stdlib organization (broadly)
- **Associated-type ergonomics** — locked in spirit (protocol `<T>` slots, no `T.Item` accessor); use-site syntax needs revision before implementation
- Closure capture specifics (escape detection, partial captures, capture-list shape)

Full list in `LANGUAGE.md` §17.

## How to read the docs in order

1. Skim `LANGUAGE.md` §1 (overview + sample) for the visual feel.
2. Then `LANGUAGE.md` §2–§16 for any specific area you need (memory model is §16).
3. `PLAN.md` for engineering roadmap and Phase boundaries.
4. `LANGUAGE.md` §17 for what's still open.
