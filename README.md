# wasm-pack-test-all

Wrapper for `wasm-pack test` that runs tests for all crates in a workspace or directory.

## Installation

### Build from source

```sh
cargo install wasm-pack wasm-pack-test-all
```

### Standalone pre-built binaries

Download standalone pre-built binaries from [releases page].

### Cargo binaries

Install from pre-built binaries using [cargo-binstall]:

```sh
cargo binstall wasm-pack-test-all
```

### With pre-commit

Use it with [pre-commit] by adding the hook to your _.pre-commit-config.yaml_:

```yaml
repos:
  - repo: https://github.com/mondeja/wasm-pack-test-all
    rev: vX.Y.Z
    hooks:
      - id: wasm-pack-test-all
        args: [tests/end2end, --chrome]
```

## Usage

```sh
wasm-pack-test-all [PATH] [WASM_PACK_TEST_OPTIONS]... -- [CARGO_TEST_OPTIONS]...
```

When running the command without any arguments, if there is a `Cargo.toml` file in the current directory with a workspace configured, it will run `wasm-pack test` for all crates in the workspace. When there it is not a workspace, it will run `wasm-pack test` for all crates in the current directory and subdirectories.

Providing a path, it will run `wasm-pack test` for all crates in that path. Providing extra options, they will be passed to `wasm-pack test` for each crate. To pass options to `cargo test`, use the `--` separator.

Don't pass a path to `wasm-pack test` options, as it will be interpreted as a crate path. If you want to test a crate individually, use `wasm-pack test` directly.

### Examples

```sh
wasm-pack-test-all --node
```

```sh
wasm-pack-test-all tests/end2end --chrome
```

```sh
wasm-pack-test-all tests/end2end --firefox --release -- --offline
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

[cargo-binstall]: https://github.com/cargo-bins/cargo-binstall
[pre-commit]: https://pre-commit.com
[releases page]: https://github.com/mondeja/wasm-pack-test-all/releases
