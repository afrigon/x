# `x`

`x` is a strongly-typed, performant, elegant systems programming language inspired by Rust, Swift, and Zig. It targets operating systems and kernels, native applications on Linux, Windows, and macOS, games with full C interop, and embeddable runtimes. Design priorities, in order: performance, elegance, speed to self-host. The compiler is self-hosting: a bootstrap written in Rust exists only to compile the compiler written in `x`.

## Repository layout

- This repository holds the compiler. The Rust bootstrap lives on `main` until the `x` compiler can compile itself. The `x` implementation is developed on branches and takes over `main` at cutover, at which point the Rust bootstrap is archived under a git tag.
- `x-lang` is a separate repository holding the documentation website.

## Where to find things

- `LANGUAGE.md` — the authoritative language design: syntax, type system, semantics, naming conventions, and the Open decisions table. Every question about what `x` looks like is answered here and nowhere else.
- The GitHub project [x-roadmap](https://github.com/users/afrigon/projects/6) holds the roadmap: milestones with their exit criteria, issues with acceptance criteria, and the project README with vision and risks.
- `README.md` — install and development commands.

## Working conventions

- Language design happens in conversation. Agreed decisions are written to `LANGUAGE.md`; unresolved ones are tabulated in its Open decisions section.
- Name the `LANGUAGE.md` section when proposing a language change, so the update lands in one place.
- The implementation follows `LANGUAGE.md` exactly. Where the specification is silent, the gap is recorded in Open decisions rather than decided in code.
- Roadmap work is tracked as GitHub issues. An issue's acceptance criteria name the `LANGUAGE.md` sections it implements.
- The language is specified in [`LANGUAGE.md`](LANGUAGE.md). Nothing in it is final: anything that does not make sense is worth flagging so the design improves. Quality is the first priority, and simplicity is never a reason to cut corners.

## Engineering decisions

| Decision | Choice | Why |
|---|---|---|
| **Bootstrap language** | Rust | Closest paradigm to `x`, so the self-hosting port is mechanical. Memory safety, ADTs, and pattern matching keep compiler code clean. |
| **Code generation** | Emit textual LLVM IR (`.ll`), shell out to `llc` and `clang` | Fastest path to native binaries with zero LLVM linking. The `llvm-c` API (via `llvm-sys` / `inkwell`) is adopted only when a JIT or finer control requires it. |
| **LLVM toolchain** | Installed through mise (`clang` and `conda:llvm-tools`), pinned in `mise.toml` | Same toolchain on every machine, locked in `mise.lock`. `llc` runs with `-relocation-model=pic` so its objects link into position-independent executables. |
| **C++ interop** | Out of scope | C interop is a pillar. C++-only LLVM features are reached through thin `extern "C"` shims. |
| **Architecture** | One frontend → shared typed IR → pluggable backends | LLVM AOT is the only backend until self-hosting; JIT and bytecode VM come after, written in `x`. |
| **Scope strategy** | Minimal compiler-complete subset first | Just enough language to write a compiler; grow features in `x` after self-hosting. If the bootstrap compiler can be written without a feature, the feature waits. |
| **Dependencies** | LLVM only, for `x` itself | The bootstrap may use Cargo crates freely. Everything else is re-implemented in `x`. |
| **Self-hosting strategy** | Stage-by-stage rewrite with cross-validation | Each stage is rewritten in `x` and validated against the Rust version before the next; both coexist during the transition. |

## Compiler architecture

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

## Self-hosting

- Each compiler stage is rewritten in `x` in pipeline order: lexer, parser, typechecker, IR, codegen, driver. The Rust and `x` versions coexist, and cross-validation tests compare their token streams, ASTs, IR, and `.ll` output.
- Cutover: the Rust bootstrap compiles the `x` compiler (stage 1); stage 1 compiles itself (stage 2); stage 2 compiles itself (stage 3). Stage 2 and stage 3 outputs must match byte for byte.
- After cutover the `x` compiler takes over `main` and the Rust bootstrap is archived under a git tag.

## Code style

- Full, descriptive names in code and in AST and type names alike: `Expression` not `Expr`, `declaration` not `decl`, `character` not `c`. Loop and tight-iteration variables may be short (`i`, `loop c in chars`). `src` and `dst` are accepted abbreviations.
- Code samples in `LANGUAGE.md` follow the same naming rules as compiler code.
