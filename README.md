# x

Compiler for the x programming language, an experimental language with type
inference, `type`, `enum`, and `proto` declarations, `match` expressions, and a
C foreign function interface. The compiler is written in Rust and lowers source
files to native object files through LLVM, passing them through a lexer, parser,
analysis, and type-checking pipeline before code generation. A tour of the
language lives in [doc/overview.md](doc/overview.md).

## Quick start

Building requires Rust and LLVM 18.

```sh
mise run build
```

## Usage

The `compile` command turns each `.x` source file into an object file written
next to it:

```sh
mise run run -- compile main.x
```

Link the resulting object file with a C toolchain to produce an executable:

```sh
clang -o main main.o
```
