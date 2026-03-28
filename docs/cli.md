# CLI behavior

## Exit codes

| Code | When |
|------|------|
| `0` | Normal exit: interactive TUI finished, or `--refresh-all` completed with no reported client errors. |
| Non-zero | Any `anyhow::Error` from `main` (typically **`1`** on common platforms). Includes I/O failures, invalid usage of internal helpers, and API errors surfaced as errors. |

### `--refresh-all` (cron-style refresh)

- On success, writes a cron status report and exits **`0`**.
- If any Codex / Claude / Copilot refresh failed, [`CronRunReport::has_errors`](src/main.rs) is true and the process exits with an **error** (non-zero) after writing the report.

### Interactive mode

- `App::load` / `app.run()` failures propagate to `main` and yield a non-zero exit code.

## Debug logging (Claude usage shape mismatch)

Set environment variable:

- **`AGENT_SWITCH_DEBUG_CLAUDE_USAGE`**: when set to `1`, `true`, `yes`, or `on` (case-insensitive), extra diagnostics may be printed to **stderr** if the Claude usage response shape does not match expectations.

Properties:

- JSON payloads are passed through **`sanitize_debug_payload`**: known sensitive keys (tokens, secrets, etc.) are replaced with **`[redacted]`**. See unit tests in `src/claude.rs`.
- **`account_id`** and **subscription** labels in debug lines are **truncated** (prefix + suffix) so logs are less identifying than full strings.
- This path is only used for **shape mismatch** debugging; it is not enabled by default.

## Related

- [Contributing](../CONTRIBUTING.md) — local `npm test`, `lint:rust`, `audit:rust`.
