# ABI Compatible Data Types

Tooling and common libraries for _ABI compatible data types_.

## Components

- `Cargo.toml`: Rust/Cargo workspace which includes almost all crates in this repository.
- `src/cli/`: Command-line tool to parse ABI type descriptions and generate Rust and C++ code.
- `src/codegen/`: Code generator to turn ABI type descriptions into executable code.
  isn't part of the top-level workspace.
- `src/parser/`: Parseer for ABI type description files.

## Building

The command line tool can be built with Bazel or with Cargo.

### Bazel

If `bazel` isn't already available on your system, go to
[Installing Bazel](https://bazel.build/install) and follow the instructions there.
Then you can build the command line tool with:

```sh
bazel build -c opt //src/cli:abi-types
```

The built executable is placed at `bazel-bin/src/cli/abi-types`.

### Cargo

If `rustup` isn't already available on your system, go to
[rustup.rs](https://rustup.rs/) and follow the instructions there.
Then you can build the command line tool with:

```sh
cargo build --release --package abi-types
```

The built executable is placed at `target/release/abi-types`.
