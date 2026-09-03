# x

`x` is a strongly-typed, performant systems programming language inspired by Rust, Swift, and Zig, designed to compile itself. This repository holds its compiler.

The language is specified in [`LANGUAGE.md`](LANGUAGE.md). Nothing in it is final: anything that does not make sense is worth flagging so the design improves. Quality is the first priority, and simplicity is never a reason to cut corners.

## Install

```sh
mise install
```

## Development

```sh
mise run build
mise run test
mise run x -- lex path/to/file.x
```
