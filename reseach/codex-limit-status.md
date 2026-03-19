# Codex Limit Status Research (5h + Weekly)

Date: 2026-03-19

## Scope
Only two status values:
- five-hour limit
- weekly limit

## Key Finding
These values are **not** stored directly in `~/.codex/auth.json`.

Codex fetches them from ChatGPT backend using auth token + account id:
- Endpoint: `https://chatgpt.com/backend-api/wham/usage`
- Headers:
  - `Authorization: Bearer <tokens.access_token>`
  - `ChatGPT-Account-Id: <tokens.account_id>`
  - `User-Agent: codex-cli`

## Response Mapping
Use `rate_limit` from response JSON.

Five-hour limit:
- window source: `rate_limit.primary_window`
- condition: `limit_window_seconds == 18000`
- left percent: `100 - used_percent`
- reset time: `reset_at` (unix timestamp)

Weekly limit:
- preferred source: `rate_limit.secondary_window` when present and `limit_window_seconds == 604800`
- fallback source: `rate_limit.primary_window` when `secondary_window == null` and primary is `604800`
- left percent: `100 - used_percent`
- reset time: `reset_at` (unix timestamp)

## Quick PowerShell Probe (single auth)
```powershell
$j = Get-Content "$HOME/.codex/auth.json" -Raw | ConvertFrom-Json
$headers = @{
  Authorization      = "Bearer $($j.tokens.access_token)"
  "ChatGPT-Account-Id" = "$($j.tokens.account_id)"
  "User-Agent"       = "codex-cli"
}
$u = Invoke-RestMethod -Uri "https://chatgpt.com/backend-api/wham/usage" -Headers $headers
$u.rate_limit | ConvertTo-Json -Depth 8
```

## Verified on Local Snapshots
Observed from 3 account snapshots in `~/.codex/accounts`:
- plus account: has both 5h + weekly windows
- free account: weekly only (no separate 5h window returned)
- team account: has both 5h + weekly windows
