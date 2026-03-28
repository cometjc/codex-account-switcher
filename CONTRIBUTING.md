# Contributing to agent-switch

## Prerequisites

- **Rust** toolchain with `cargo` (edition 2021).
- **Node.js** 18+ for contract tests (`node --test`) and npm metadata.

## Build

```sh
npm run build
```

This runs `cargo build --bin agent-switch`. The published npm package is a thin shim around the Rust binary.

## Test

```sh
npm test
```

Runs `cargo test`, then all `tests/*.test.js` files under `node --test`. Prefer this over `cargo build` alone when validating changes (see `CLAUDE.md`).

## Lint (Rust)

```sh
npm run lint:rust
```

Runs `cargo clippy --all-targets -- -D warnings`.

## Security audit (Rust dependencies)

```sh
npm run audit:rust
```

Requires [**cargo-audit**](https://github.com/rustsec/rustsec/tree/main/cargo-audit) installed (`cargo install cargo-audit`). Fails if the binary is missing or if the advisory database reports issues.

## One-shot local check

```sh
npm run check
```

Runs `lint:rust`, `test`, and `audit:rust` in sequence.

## Parallel lane dev (PLD)

Executor-backed multi-lane workflow: see [`plugins/parallel-lane-dev/README.md`](plugins/parallel-lane-dev/README.md).

First-time setup (when `plugins/parallel-lane-dev/scripts` is missing or broken):

```sh
chmod +x scripts/install-pld-plugin.sh
./scripts/install-pld-plugin.sh
```

Health check:

```sh
npm run pld:executor:audit -- --json
```

Requires `sqlite3` on your `PATH`.

## CLI exit codes and debug

See [docs/cli.md](docs/cli.md).
