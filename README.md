# codex-auth

A command-line tool that lets you manage and switch between multiple Codex accounts instantly, no more constant logins and logouts.

> [!WARNING]
> Not affiliated with OpenAI or Codex. Not an official tool.

## How it Works

Codex stores your authentication session in a single `auth.json` file. This tool works by creating named snapshots of that file for each of your accounts. When you want to switch, `codex-auth` swaps the active `~/.codex/auth.json` with the snapshot you select, instantly changing your logged-in account.

## Requirements

- Node.js 18 or newer

## Plot Mode

Plot mode is an experimental, in-progress phase-1 migration toward a Rust TUI viewer.

- Node remains the source of truth for auth, cache, and API access in this phase.
- Rust is intended to own rendering for the plot viewer only.
- Any `plot:viewer:*` scripts in `package.json` should be treated as developer scaffolding, not stable end-user commands.
- Those scripts now point at cargo-backed Rust viewer commands, but the viewer itself is still being fleshed out.

## Install (npm)

```sh
npm i -g codex-auth
```

## Usage

```sh
# start interactive profile manager
codex-auth
```

### Interactive controls

- `Enter` on `[CURRENT][UNSAVED]` – Save current `~/.codex/auth.json` with editable default name (`email-plan`).
- `Enter` on `[SAVED]` – If current profile is unsaved, prompts to save first, then switches to selected saved profile.
- `D` or `Del` on `[SAVED]` – Confirm and delete saved snapshot.
- `R` on `[SAVED]` – Rename saved snapshot.
- `U` – Refresh 5h/weekly limits immediately.

### Limits shown per profile

- Weekly and (if available) 5-hour limits are fetched from ChatGPT usage endpoint and cached locally.
- If no separate 5-hour window exists for an account, only Weekly is shown.
- Line format is aligned and includes rate hint:
  - `Weekly limit:         [███████████████████░] 94% left (resets 10:38 on 26 Mar) · Can use 1.2%/hour for next 78.0 hours`

Notes:

- Works on macOS/Linux (symlink) and Windows (copy).
- Requires Node 18+.
