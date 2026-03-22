# codex-auth

A command-line tool that lets you manage, switch, and inspect multiple Codex accounts from a unified Rust TUI.

> [!WARNING]
> Not affiliated with OpenAI or Codex. Not an official tool.

## How it Works

Codex stores your authentication session in a single `auth.json` file. This tool works by creating named snapshots of that file for each of your accounts. When you want to switch, `codex-auth` swaps the active `~/.codex/auth.json` with the snapshot you select, instantly changing your logged-in account.

## Requirements

- Rust toolchain with Cargo
- Node.js 18 or newer for repo automation, NLSDD scripts, and legacy development helpers

## Rust TUI

`codex-auth` now has a Rust-first runtime for auth/profile management and plot rendering.

- Auth snapshot storage, saved profile switching, usage cache reads, and the plot view all live in the same Rust app.
- Plot is no longer treated as a separate external viewer truth; it is a built-in view of the main TUI.
- `plot:viewer:*` and `rust:auth:*` scripts in `package.json` both target the same Rust `codex-auth` binary during local development.

## Build

```sh
cargo build --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth
```

## Install (npm)

```sh
npm i -g codex-auth
```

The npm package still exists for repository/distribution continuity, but the product runtime is now the Rust binary.

## Usage

```sh
# start the Rust interactive profile manager
cargo run --manifest-path rust/plot-viewer/Cargo.toml --bin codex-auth --
```

### Interactive controls

- `Up` / `Down` – Move between saved or current-unsaved profiles.
- `Enter` – Save current unsaved profile, or switch to the selected saved profile.
- `N` – Rename the selected saved profile.
- `D` or `Del` – Delete the selected saved profile after confirmation.
- `U` – Refresh usage for the selected profile.
- `A` – Refresh usage for all visible profiles.
- `P` or `B` – Toggle between the account list and the built-in plot view.
- `Left` / `Right` in plot view – Cycle profiles without leaving the plot.
- `Tab` – Switch chart/panel focus inside plot view.
- `Q` – Quit.

### Limits and plot

- Weekly and, when available, 5-hour limits are fetched from the ChatGPT usage endpoint and cached locally.
- The built-in plot view renders a 7-day usage line and a 5-hour band from the same Rust-side state used by the account list.
- If no separate 5-hour window exists for an account, the plot stays truthful and marks the band as unavailable.

Notes:

- Works on macOS/Linux with symlink switching and on Windows with file copy switching.
- Node remains in the repo for executor automation and legacy tooling, not as the primary auth runtime.
