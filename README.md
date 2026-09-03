# x

`x` is a strongly-typed, performant systems programming language inspired by Rust, Swift, and Zig. It targets operating systems, native applications, games, and embeddable runtimes, and it is designed to compile itself. This repository holds the compiler: a bootstrap written in Rust whose only job is to compile the self-hosted compiler written in `x`.

The language itself is specified in [`LANGUAGE.md`](LANGUAGE.md). Engineering decisions and the compiler architecture are in [`PLAN.md`](PLAN.md).

## Install

```sh
mise install
```

This pulls the Rust toolchain and the LLVM tools the compiler shells out to (`llc` for code generation, `clang` for linking).

## Usage

```sh
cargo run -- lex path/to/file.x
```

## Development

```sh
mise run build
mise run test
```
