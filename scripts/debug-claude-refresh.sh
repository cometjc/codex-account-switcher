#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
claude_dir="${HOME}/.claude"
credentials_path="${claude_dir}/.credentials.json"
cache_path="${claude_dir}/claude-auth-limit-cache.json"
history_path="${claude_dir}/claude-auth-usage-history.json"
log_dir="${repo_root}/.tmp"
log_path="${log_dir}/debug-claude-refresh.log"

if [[ ! -f "${credentials_path}" ]]; then
  echo "Missing Claude credentials: ${credentials_path}" >&2
  exit 1
fi

mkdir -p "${log_dir}"

before_cache_mtime="$(stat -c %Y "${cache_path}" 2>/dev/null || echo missing)"
before_history_mtime="$(stat -c %Y "${history_path}" 2>/dev/null || echo missing)"

echo "Running Claude live refresh via agent-switch (debug payload summaries enabled)..."
set +e
AGENT_SWITCH_DEBUG_CLAUDE_USAGE=1 cargo run --quiet --manifest-path "${repo_root}/Cargo.toml" --bin agent-switch -- --refresh-all 2>&1 | tee "${log_path}"
refresh_status=${PIPESTATUS[0]}
set -e

if [[ "${refresh_status}" -ne 0 ]]; then
  echo "agent-switch --refresh-all exited with ${refresh_status}. Debug payload summaries above may indicate which profile responses were unexpected." >&2
fi

if [[ ! -f "${cache_path}" ]]; then
  echo "Claude cache file was not created: ${cache_path}" >&2
  exit 1
fi

if [[ ! -f "${history_path}" ]]; then
  echo "Claude history file was not created: ${history_path}" >&2
  exit 1
fi

after_cache_mtime="$(stat -c %Y "${cache_path}")"
after_history_mtime="$(stat -c %Y "${history_path}")"

python - "${cache_path}" "${history_path}" "${log_path}" "${before_cache_mtime}" "${before_history_mtime}" "${after_cache_mtime}" "${after_history_mtime}" <<'PY'
from pathlib import Path
import json
import sys

cache_path = Path(sys.argv[1])
history_path = Path(sys.argv[2])
log_path = sys.argv[3]
before_cache = sys.argv[4]
before_history = sys.argv[5]
after_cache = int(sys.argv[6])
after_history = int(sys.argv[7])

cache = json.loads(cache_path.read_text())
history = json.loads(history_path.read_text())

by_account_cache = cache.get("byAccountId", {})
by_account_history = history.get("byAccountId", {})

weekly_accounts = []
five_hour_accounts = []
for account_id, profile_history in by_account_history.items():
    if profile_history.get("weekly_windows"):
        weekly_accounts.append(account_id)
    if profile_history.get("five_hour_windows"):
        five_hour_accounts.append(account_id)

print(f"Cache accounts: {len(by_account_cache)}")
print(f"History accounts: {len(by_account_history)}")
print(f"Accounts with weekly history: {len(weekly_accounts)}")
print(f"Accounts with 5h history: {len(five_hour_accounts)}")
print(f"Debug output logged to: {log_path}")
print(f"Cache mtime: {before_cache} -> {after_cache}")
print(f"History mtime: {before_history} -> {after_history}")

if not by_account_cache:
    raise SystemExit("Claude cache is empty after refresh.")

if not by_account_history:
    raise SystemExit("Claude history is empty after refresh.")

if after_cache == before_cache and after_history == before_history:
    raise SystemExit("Claude cache/history mtimes did not change during refresh.")
PY

cat <<'EOF'

Next manual TUI smoke check:
  1. cargo run --manifest-path Cargo.toml --bin agent-switch
  2. Confirm a Claude profile appears with the [cl] tag.
  3. Press `u` or `a` if needed, then verify the Details pane shows Claude metadata.
  4. Press `Tab` to move to the plot pane and confirm weekly / 5h chart data is visible when history exists.
EOF

if [[ "${refresh_status}" -ne 0 ]]; then
  exit "${refresh_status}"
fi
