# agent-switch

A command-line tool that lets you manage, switch, and inspect multiple Codex accounts from a unified Rust TUI.

> [!WARNING]
> Not affiliated with OpenAI or Codex. Not an official tool.

## How it Works

Codex stores your authentication session in a single `auth.json` file. This tool works by creating named snapshots of that file for each of your accounts. When you want to switch, `agent-switch` swaps the active `~/.codex/auth.json` with the snapshot you select, instantly changing your logged-in account.

## Requirements

- Rust toolchain with Cargo
- Node.js 18 or newer for repo contract tests (`node --test`) and npm metadata

## Rust TUI

`agent-switch` now has a Rust-first runtime for auth/profile management and plot rendering.

- Auth snapshot storage, saved profile switching, usage cache reads, and the plot view all live in the same Rust app.
- Plot is no longer treated as a separate external viewer truth; it is a built-in view of the main TUI.
- The npm package is now a thin shim that forwards `agent-switch` into the single Rust `agent-switch` binary entrypoint.

## Build

```sh
cargo build --bin agent-switch
```

## Install (npm)

```sh
npm i -g agent-switch
```

The npm package is only a thin shim; the product runtime lives in the Rust binary.

## Usage

```sh
# start the Rust interactive profile manager
agent-switch
```

### Interactive controls

- `Up` / `Down` – Move between saved or current-unsaved profiles.
- `Enter` – Save current unsaved profile, or switch to the selected saved profile.
- `R` – Rename the selected saved profile.
- `D` or `Del` – Delete the selected saved profile after confirmation.
- `U` – Refresh usage for the selected profile.
- `A` – Refresh usage for all visible profiles.
- `P` or `B` – Toggle between the account list and the built-in plot view.
- `Left` / `Right` in plot view – Move the chart cursor without leaving the plot.
- `Tab` – Switch chart/panel focus inside plot view.
- `Q` – Quit.

### Limits and plot

- Weekly and, when available, 5-hour limits are fetched from the ChatGPT usage endpoint and cached locally.
- The built-in plot view renders a 7-day usage line and a 5-hour band from the same Rust-side state used by the account list.
- If no separate 5-hour window exists for an account, the plot stays truthful and marks the band as unavailable.

### Claude live verification

If you want to verify Claude live refresh and chart generation on a machine that can reach the Claude usage API, run:

```sh
./scripts/verify-claude-live.sh
```

The script will:

- run `agent-switch --refresh-all`
- assert that Claude cache/history files were created or updated under `~/.claude/`
- summarize whether weekly / 5h history entries exist
- print the final manual TUI smoke-check steps

Notes:

- If Claude usage returns HTTP `429` or `401`, `agent-switch` does **not** retry with the same access token. It refreshes the current Claude OAuth credentials in `~/.claude/.credentials.json`, then makes one follow-up usage request with the rotated token.
- Works on macOS/Linux with symlink switching and on Windows with file copy switching.
- Node remains in the repo for contract tests and npm metadata, not as the primary auth runtime. **PLD** (parallel-lane) tooling is wired under [`plugins/parallel-lane-dev/`](plugins/parallel-lane-dev/) (see [CONTRIBUTING.md](CONTRIBUTING.md)).

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for build, test, Clippy, and `cargo-audit`. Exit codes and optional Claude usage debug: [docs/cli.md](docs/cli.md).
