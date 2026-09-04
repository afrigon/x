# x

`x` is a strongly-typed, performant systems programming language inspired by Rust, Swift, and Zig, designed to compile itself. This repository holds its compiler.

## Install

```sh
mise install
```

## Development

```sh
mise run build
mise run test
mise run spec
mise run x -- build path/to/file.x
mise run x -- run path/to/file.x
```

`build` accepts `--emit <kind>` to stop at an intermediate form (`tokens`, `ast`, `llvm-ir`, `assembly`, `object`) and `--save-temps` to keep the `.ll` and `.o` files.

End-to-end programs live under `tests/programs/`: each `.x` file is compiled and run, its stdout and stderr compared with sibling `.stdout` and `.stderr` snapshots and its exit code with an `//@ exit-code: N` directive at the top of the source. A missing snapshot means empty output. `X_RECORD_SNAPSHOTS=1 mise run test` rewrites the snapshots from actual output.
