# `x`

`x` is a strongly-typed, performant, elegant systems programming language inspired by Rust, Swift, and Zig. It targets operating systems, native applications, games, and embeddable runtimes. The compiler is self-hosting: a bootstrap written in Rust exists only to compile the compiler written in `x`.

## Repository layout

- This repository holds the compiler. The Rust bootstrap lives on `main` until the `x` compiler can compile itself. The `x` implementation is developed on branches and takes over `main` at cutover, at which point the Rust bootstrap is archived under a git tag.
- `x-lang` is a separate repository holding the documentation website.

## Where to find things

- `LANGUAGE.md` — the authoritative language design: syntax, type system, semantics, naming conventions, and the table of open questions (§17). Every question about what `x` looks like is answered here and nowhere else.
- `PLAN.md` — engineering decisions, compiler architecture, self-hosting strategy, and risks. It references `LANGUAGE.md` and never restates it.
- The roadmap is the GitHub project [x-roadmap](https://github.com/users/afrigon/projects/6): issues grouped by milestone, each carrying its own acceptance criteria.
- `README.md` — install, usage, and development commands.

## Working conventions

- Language design happens in conversation. Agreed decisions are written to `LANGUAGE.md`; unresolved ones are tabulated in `LANGUAGE.md` §17.
- Reference the `LANGUAGE.md` section number when proposing a language change, so the update lands in one place.
- The implementation follows `LANGUAGE.md` exactly. Where the specification is silent, the gap is recorded in §17 rather than decided in code.
- Roadmap work is tracked as GitHub issues. An issue's acceptance criteria reference the `LANGUAGE.md` sections it implements.

## Code style

- Full, descriptive names in code and in AST and type names alike: `Expression` not `Expr`, `declaration` not `decl`, `character` not `c`. Loop and tight-iteration variables may be short (`i`, `loop c in chars`). `src` and `dst` are accepted abbreviations.
- Code samples in `LANGUAGE.md` follow the same naming rules as compiler code.
