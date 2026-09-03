# `x` — Engineering Plan

The engineering decisions behind the `x` compiler. `LANGUAGE.md` is the authoritative reference for the language itself and is never restated here; the roadmap lives in the GitHub project [x-roadmap](https://github.com/users/afrigon/projects/6).

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

## 2. Engineering decisions

| Decision | Choice | Why |
|---|---|---|
| **Bootstrap language** | Rust | Closest paradigm to `x`, so the self-hosting port is mechanical. Memory safety, ADTs, and pattern matching keep compiler code clean. |
| **Code generation** | Emit textual LLVM IR (`.ll`), shell out to `llc` and `clang` | Fastest path to native binaries with zero LLVM linking. The `llvm-c` API (via `llvm-sys` / `inkwell`) is adopted only when a JIT or finer control requires it. |
| **LLVM toolchain** | Installed through mise (`clang` and `conda:llvm-tools`), pinned in `mise.toml` | Same toolchain on every machine, locked in `mise.lock`. `llc` runs with `-relocation-model=pic` so its objects link into position-independent executables. |
| **C++ interop** | Out of scope | C interop is a pillar. C++-only LLVM features are reached through thin `extern "C"` shims. |
| **Architecture** | One frontend → shared typed IR → pluggable backends | LLVM AOT is the only backend until self-hosting; JIT and bytecode VM come after, written in `x`. |
| **Scope strategy** | Minimal compiler-complete subset first | Just enough language to write a compiler; grow features in `x` after self-hosting. |
| **Dependencies** | LLVM only, for `x` itself | The bootstrap may use Cargo crates freely. Everything else is re-implemented in `x`. |
| **Self-hosting strategy** | Stage-by-stage rewrite with cross-validation | Each stage is rewritten in `x` and validated against the Rust version before the next; both coexist during the transition. |
| **Bootstrap fate** | Archived under a git tag at cutover | The tag stays usable for cross-compilation and platform bring-up. |

Language decisions, including everything out of scope for v1 and every open question, are in `LANGUAGE.md` (§17 for the open table).

---

## 3. Compiler architecture

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
   ─ bootstrap ─ ── after self-host ──
```

A single frontend produces a typed IR; backends are pluggable. Only LLVM AOT exists in the bootstrap. JIT and bytecode VM are written in `x` after self-hosting.

---

## 4. Self-hosting

- Each compiler stage is rewritten in `x` in pipeline order: lexer, parser, typechecker, IR, codegen, driver. The Rust and `x` versions coexist, and cross-validation tests compare their token streams, ASTs, IR, and `.ll` output.
- Cutover: the Rust bootstrap compiles the `x` compiler (stage 1); stage 1 compiles itself (stage 2); stage 2 compiles itself (stage 3). Stage 2 and stage 3 outputs must match byte for byte.
- After cutover the `x` compiler takes over `main` and the Rust bootstrap is archived under a git tag.

---

## 5. Minimal subset

What must exist before self-hosting is attempted. Section references point into `LANGUAGE.md`.

- **Types and literals** (§3, §6) — all primitives; records and enums via `type`; `[T; N]`, `[T]`, `T?`, `T!E`; generic types; references and raw pointers. No tuples.
- **Functions** (§5) — `fun` declarations, Swift-style argument labels, default arguments, generics with bounds, closures and trailing-closure form, the three method modifiers.
- **Control flow** (§9) — `if` / `else` and `match` (with guards) as expressions; `loop` / `loop until` / `loop x in iter`; `guard ... else`. Named loops are deferred (§9.4).
- **Optional and Result** (§10) — `T?` and `T!E` sugar, the `?`, `?.`, `??` operators, one-level auto-wrap.
- **Protocols** (§7, §11) — `proto` declarations with inline conformance and refinement; protocol `<T>` slots; the operator-protocol family; `Displayable` and `Debugable`; compiler synthesis (§11.5).
- **Memory** (§16) — ownership and moves, RAII via `Droppable`, `&T` / `&mut T` (lifetimes unchecked), raw pointers via `unsafe`, `Box<T>`, the `Allocator` protocol with `defaultAllocator` and override.
- **FFI** (§15) — `@extern(.c, ...)` for functions, opaque types, and C-layout types; `string.cstr()` at the boundary.
- **Macros** (§14) — the built-in `@` attributes and `#` verbs.
- **Modules** (§13) — `import`, default-public visibility, `private`.

**Initial stdlib:** `List<T>`, `HashMap<K, V>`, `Set<T>`, `string`, `Box<T>`; `GeneralPurposeAllocator` plus `ArenaAllocator`, `PoolAllocator`, `FixedBufferAllocator`; libc-backed I/O.

**Out of scope before self-host:** advanced type inference, async, user-definable macros, reflection, regex literals, JIT, bytecode VM, opt-in lifetime checking, the full standard library.

**Discipline rule:** if the bootstrap compiler can be written without a feature, the feature waits.

---

## 6. Risks

- **Reference lifetimes unchecked in v1.** The biggest safety carve-out. Mitigated by RAII, the arena-and-indices idiom for graph data (§16.12), and debug-mode runtime sanitizers. The v2 path is opt-in lifetime checking (§16.13).
- **Self-hosting stage transitions.** Calling conventions, layout compatibility, miscompiles. Mitigated by byte-equivalence and behavioral testing between stages, one stage at a time.
- **LLVM API churn.** The toolchain is pinned through `mise.lock`; upgrades are deliberate commits. Textual IR is more stable than the C API, which is more stable than C++.
- **Associated-type ergonomics.** Locked in spirit but flagged for revision (§17). If unworkable, the fallback is a separate `type Item` syntax inside protocols, Swift- and Rust-style.
- **Scope creep.** Constant temptation to add features before self-hosting. The discipline rule in §5 is the answer.
- **Allocator-aware code is more verbose than GC.** Mitigated by `defaultAllocator`; ergonomics are revisited if real code proves painful.
