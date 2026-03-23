# Claude Code Integration Implementation Plan

> **Status:** Archived as shipped. Claude profile loading, save/use/rename/delete, refresh-all support, service tags, and chart data support are now implemented in `rust/plot-viewer`.
>
> **Roadmap note:** This document is kept as historical implementation context. Treat unchecked boxes below as superseded planning artifacts unless they describe a still-observable gap in the current product.

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Claude Code account management and usage display alongside Codex in the existing plot-viewer TUI, mirroring the `~/.codex/auth.json` save/switch pattern for `~/.claude/.credentials.json`.

**Architecture:** Add a new `claude.rs` module that mirrors `store.rs` and provides Claude-specific credential parsing + usage fetching. Extend `app.rs` with `ProfileKind` on `ProfileEntry` so Codex and Claude profiles coexist in one list; branch `activate/save/rename/delete` on kind. No changes to `render/chart.rs`.

**Tech Stack:** Rust, Ratatui, reqwest blocking, serde_json — no new dependencies (RFC3339 parsed manually).

---

## Shipped outcome summary

- Claude credentials are sourced from `~/.claude/.credentials.json`.
- Claude profiles participate in the same list UI as Codex profiles, with `[cl]` / `[cx]` tags.
- `--refresh-all` records Claude usage snapshots and chart history.
- Chart generation for Claude now lives behind the extracted loader flow (`src/loader.rs` / `src/app_data.rs`), not the original monolithic `app.rs` design assumed by this plan.

---

## Confirmed file formats

**`~/.claude/.credentials.json`** (Linux/Windows):
```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1774256835250,
    "scopes": [...],
    "subscriptionType": "team",
    "rateLimitTier": "default_raven"
  }
}
```

**Claude usage API response** (`GET https://api.anthropic.com/api/oauth/usage`):
```json
{
  "five_hour": { "utilization": 40.0, "resets_at": "2025-11-04T04:59:59.943648+00:00" },
  "seven_day":  { "utilization": 28.0, "resets_at": "2025-11-06T03:59:59.943679+00:00" }
}
```

---

## File map

| File | Action | Responsibility |
|------|--------|----------------|
| `rust/plot-viewer/src/claude.rs` | **Create** | Credential model, path detection, account store, usage fetcher |
| `rust/plot-viewer/src/lib.rs` | Modify | Add `pub mod claude;` |
| `rust/plot-viewer/src/app.rs` | Modify | `ProfileKind`, load Claude profiles, branch activate/save/delete/rename |
| `rust/plot-viewer/src/main.rs` | Modify | Instantiate `ClaudeStore` + `UsageService` for Claude, pass to `App::load` |

---

## Chunk 1: claude.rs — new module

### Task 1: ClaudeCredentials + ClaudePaths + lib.rs

**Files:**
- Create: `rust/plot-viewer/src/claude.rs`
- Modify: `rust/plot-viewer/src/lib.rs`

- [ ] **Step 1: Add `pub mod claude;` to lib.rs**

```rust
// lib.rs — add after existing pub mod lines:
pub mod claude;
```

- [ ] **Step 2: Create `src/claude.rs` with credential model + paths**

```rust
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Credential model ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeOauthToken {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: i64,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeCredentials {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: ClaudeOauthToken,
}

impl ClaudeCredentials {
    /// Stable identifier derived from the refresh token (first 20 chars after prefix).
    /// Stays consistent between access-token rotations.
    pub fn account_id(&self) -> String {
        let token = &self.claude_ai_oauth.refresh_token;
        let body = token.strip_prefix("sk-ant-ort01-").unwrap_or(token.as_str());
        format!("claude-{}", &body[..body.len().min(20)])
    }

    pub fn access_token(&self) -> &str {
        &self.claude_ai_oauth.access_token
    }

    pub fn subscription_type(&self) -> &str {
        &self.claude_ai_oauth.subscription_type
    }
}

// ── Path helpers ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudePaths {
    claude_dir: PathBuf,
    credentials_path: PathBuf,
    accounts_dir: PathBuf,
    current_name_path: PathBuf,
    limit_cache_path: PathBuf,
    usage_history_path: PathBuf,
}

impl ClaudePaths {
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::from_claude_dir(home.join(".claude"))
    }

    pub fn from_claude_dir(claude_dir: PathBuf) -> Self {
        Self {
            credentials_path: claude_dir.join(".credentials.json"),
            accounts_dir: claude_dir.join("accounts"),
            current_name_path: claude_dir.join("current"),
            limit_cache_path: claude_dir.join("claude-auth-limit-cache.json"),
            usage_history_path: claude_dir.join("claude-auth-usage-history.json"),
            claude_dir,
        }
    }

    pub fn claude_dir(&self) -> &Path { &self.claude_dir }
    pub fn credentials_path(&self) -> &Path { &self.credentials_path }
    pub fn accounts_dir(&self) -> &Path { &self.accounts_dir }
    pub fn current_name_path(&self) -> &Path { &self.current_name_path }
    pub fn limit_cache_path(&self) -> &Path { &self.limit_cache_path }
    pub fn usage_history_path(&self) -> &Path { &self.usage_history_path }
}
```

- [ ] **Step 3: Confirm compilation**

```bash
cd rust/plot-viewer && cargo build 2>&1
```
Expected: 0 errors.

---

### Task 2: ClaudeStore

**Files:**
- Modify: `rust/plot-viewer/src/claude.rs`

- [ ] **Step 1: Write failing test for ClaudeStore list/save/use**

Add to the bottom of `claude.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;  // NOTE: no tempfile dep — use std::env::temp_dir() instead

    fn temp_dir_pair() -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "claude-store-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        let claude_dir = base.join("dot-claude");
        fs::create_dir_all(&claude_dir).unwrap();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        (claude_dir, base)
    }

    fn sample_creds_json() -> &'static str {
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-aaa","refreshToken":"sk-ant-ort01-bbb","expiresAt":9999999999,"subscriptionType":"pro","rateLimitTier":"x","scopes":[]}}"#
    }

    #[test]
    fn claude_store_list_save_use_roundtrip() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        // Write fake credentials file
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();
        let store = ClaudeStore::new(paths);

        // Initially no saved accounts
        assert!(store.list_account_names().unwrap().is_empty());

        // Save current
        let name = store.save_account("work").unwrap();
        assert_eq!(name, "work");
        assert_eq!(store.list_account_names().unwrap(), vec!["work"]);

        // Switch to saved account (use_account writes credentials back)
        store.use_account("work").unwrap();
        let current = store.get_current_credentials().unwrap();
        assert_eq!(current.claude_ai_oauth.subscription_type, "pro");
    }

    #[test]
    fn claude_store_rename_delete() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();
        let store = ClaudeStore::new(paths);
        store.save_account("work").unwrap();
        store.rename_account("work", "personal").unwrap();
        assert_eq!(store.list_account_names().unwrap(), vec!["personal"]);
        store.delete_account("personal").unwrap();
        assert!(store.list_account_names().unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd rust/plot-viewer && cargo test claude 2>&1
```
Expected: compile error (ClaudeStore not defined).

- [ ] **Step 3: Implement ClaudeStore**

Add to `claude.rs` (before the `#[cfg(test)]` block):

```rust
// ── Account store ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClaudeStore {
    paths: ClaudePaths,
}

impl ClaudeStore {
    pub fn new(paths: ClaudePaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &ClaudePaths { &self.paths }

    pub fn get_current_credentials(&self) -> Result<ClaudeCredentials> {
        self.ensure_credentials_exist()?;
        read_credentials(self.paths.credentials_path())
    }

    pub fn get_current_snapshot(&self) -> Result<Value> {
        self.ensure_credentials_exist()?;
        read_snapshot(self.paths.credentials_path())
    }

    pub fn get_current_account_name(&self) -> Result<Option<String>> {
        read_current_name_file(self.paths.current_name_path())
    }

    pub fn list_account_names(&self) -> Result<Vec<String>> {
        if !self.paths.accounts_dir().exists() {
            return Ok(Vec::new());
        }
        let mut names = fs::read_dir(self.paths.accounts_dir())
            .with_context(|| format!("read {}", self.paths.accounts_dir().display()))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_file() { return None; }
                let name = path.file_name()?.to_str()?;
                if !name.ends_with(".json") { return None; }
                Some(name.trim_end_matches(".json").to_string())
            })
            .collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }

    pub fn list_saved_profiles(&self) -> Result<Vec<ClaudeSavedProfile>> {
        self.list_account_names()?
            .into_iter()
            .map(|name| {
                let file_path = self.account_file_path(&name);
                Ok(ClaudeSavedProfile {
                    name,
                    snapshot: read_snapshot(&file_path)?,
                    file_path,
                })
            })
            .collect()
    }

    pub fn save_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_credentials_exist()?;
        self.ensure_accounts_dir()?;
        fs::copy(self.paths.credentials_path(), self.account_file_path(&name))
            .with_context(|| format!("copy credentials to {}", self.account_file_path(&name).display()))?;
        Ok(name)
    }

    pub fn save_snapshot(&self, raw_name: &str, snapshot: &Value) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_accounts_dir()?;
        let destination = self.account_file_path(&name);
        if destination.exists() {
            bail!("saved profile already exists: {}", name);
        }
        write_json(&destination, snapshot)?;
        Ok(name)
    }

    pub fn use_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        let source = self.account_file_path(&name);
        if !source.exists() {
            bail!("saved profile not found: {}", name);
        }
        fs::create_dir_all(self.paths.claude_dir())
            .with_context(|| format!("create {}", self.paths.claude_dir().display()))?;
        // Overwrite current credentials (copy, no symlink — credentials file is hidden)
        fs::copy(&source, self.paths.credentials_path())
            .with_context(|| format!("copy {} -> {}", source.display(), self.paths.credentials_path().display()))?;
        fs::write(self.paths.current_name_path(), format!("{name}\n"))
            .with_context(|| format!("write {}", self.paths.current_name_path().display()))?;
        Ok(name)
    }

    pub fn delete_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        let target = self.account_file_path(&name);
        if !target.exists() {
            bail!("saved profile not found: {}", name);
        }
        fs::remove_file(&target)
            .with_context(|| format!("delete {}", target.display()))?;
        Ok(name)
    }

    pub fn rename_account(&self, raw_current: &str, raw_next: &str) -> Result<String> {
        let current = normalize_account_name(raw_current)?;
        let next = normalize_account_name(raw_next)?;
        let current_path = self.account_file_path(&current);
        if !current_path.exists() {
            bail!("saved profile not found: {}", current);
        }
        let next_path = self.account_file_path(&next);
        if current != next && next_path.exists() {
            bail!("saved profile already exists: {}", next);
        }
        if current != next {
            fs::rename(&current_path, &next_path)
                .with_context(|| format!("rename {} -> {}", current_path.display(), next_path.display()))?;
        }
        if self.get_current_account_name()?.as_deref() == Some(current.as_str()) {
            fs::write(self.paths.current_name_path(), format!("{next}\n"))
                .with_context(|| format!("write {}", self.paths.current_name_path().display()))?;
        }
        Ok(next)
    }

    fn ensure_credentials_exist(&self) -> Result<()> {
        if !self.paths.credentials_path().exists() {
            bail!("no Claude credentials file found at {}", self.paths.credentials_path().display());
        }
        Ok(())
    }

    fn ensure_accounts_dir(&self) -> Result<()> {
        fs::create_dir_all(self.paths.accounts_dir())
            .with_context(|| format!("create {}", self.paths.accounts_dir().display()))
    }

    fn account_file_path(&self, name: &str) -> PathBuf {
        self.paths.accounts_dir().join(format!("{name}.json"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeSavedProfile {
    pub name: String,
    pub file_path: PathBuf,
    pub snapshot: Value,
}

// ── Helpers (private) ──────────────────────────────────────────────────────────

fn read_credentials(path: impl AsRef<Path>) -> Result<ClaudeCredentials> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse Claude credentials at {}", path.display()))
}

fn read_snapshot(path: impl AsRef<Path>) -> Result<Value> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse JSON at {}", path.display()))
}

fn read_current_name_file(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let trimmed = contents.trim();
            Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("write {}", path.display()))
}

fn normalize_account_name(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_end_matches(".json");
    if trimmed.is_empty() {
        bail!("invalid account name");
    }
    if !trimmed.chars().enumerate().all(|(i, ch)| {
        if i == 0 { ch.is_ascii_alphanumeric() }
        else { ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') }
    }) {
        bail!("invalid account name");
    }
    Ok(trimmed.to_string())
}
```

- [ ] **Step 4: Run tests**

```bash
cd rust/plot-viewer && cargo test claude 2>&1
```
Expected: `claude_store_list_save_use_roundtrip` and `claude_store_rename_delete` pass.

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/claude.rs rust/plot-viewer/src/lib.rs
git commit -m "feat(claude): ClaudeCredentials, ClaudePaths, ClaudeStore"
```

---

### Task 3: fetch_claude_usage + UsageService wiring

**Files:**
- Modify: `rust/plot-viewer/src/claude.rs`

The Claude usage API returns `utilization` (%) + `resets_at` (RFC3339). We map these to the existing `UsageResponse` / `UsageWindow` structs used by the Codex flow — no new structs needed.

Mapping:
- `five_hour` → `primary_window`, `limit_window_seconds = 18_000`
- `seven_day` → `secondary_window`, `limit_window_seconds = 604_800`
- `reset_at` = parsed `resets_at` unix timestamp
- `reset_after_seconds = reset_at - now`
- `email = None`, `plan_type = credentials.subscription_type` (passed in by caller)

- [ ] **Step 1: Write failing test**

Add to the `#[cfg(test)]` block:

```rust
    #[test]
    fn fetch_claude_usage_maps_response_correctly() {
        // Test the mapping function directly (no HTTP)
        let now = 1_730_000_000_i64;
        let resets_at_5h = "2025-10-27T04:59:59.000000+00:00";
        let resets_at_7d = "2025-10-29T04:59:59.000000+00:00";
        // Manually compute expected unix timestamps
        // 2025-10-27T04:59:59Z  ≈ 1730005199
        // 2025-10-29T04:59:59Z  ≈ 1730177999
        let raw = serde_json::json!({
            "five_hour": { "utilization": 40.0, "resets_at": resets_at_5h },
            "seven_day":  { "utilization": 28.0, "resets_at": resets_at_7d }
        });
        let response = map_claude_usage_response(&raw, "team", now).unwrap();
        assert_eq!(response.plan_type.as_deref(), Some("team"));
        let rl = response.rate_limit.as_ref().unwrap();
        let five_h = rl.primary_window.as_ref().unwrap();
        assert_eq!(five_h.limit_window_seconds, 18_000);
        assert!((five_h.used_percent - 40.0).abs() < 0.01);
        assert!(five_h.reset_at > 0);
        let seven_d = rl.secondary_window.as_ref().unwrap();
        assert_eq!(seven_d.limit_window_seconds, 604_800);
        assert!((seven_d.used_percent - 28.0).abs() < 0.01);
    }
```

Run: `cargo test fetch_claude_usage 2>&1` — Expected: compile error.

- [ ] **Step 2: Implement fetch helpers**

Add to `claude.rs` (before `#[cfg(test)]`):

```rust
use crate::usage::{UsageRateLimit, UsageResponse, UsageWindow};

/// Map a raw Claude usage API JSON response + subscription_type to UsageResponse.
/// `now` is the current unix timestamp in seconds (injected for testability).
pub fn map_claude_usage_response(raw: &Value, subscription_type: &str, now: i64) -> Option<UsageResponse> {
    let five_hour = raw.get("five_hour")?;
    let seven_day = raw.get("seven_day")?;

    let five_h_utilization = five_hour.get("utilization")?.as_f64()?;
    let five_h_resets_at = five_hour.get("resets_at")?.as_str()?;
    let seven_d_utilization = seven_day.get("utilization")?.as_f64()?;
    let seven_d_resets_at = seven_day.get("resets_at")?.as_str()?;

    let five_h_reset_at = parse_rfc3339_unix(five_h_resets_at)?;
    let seven_d_reset_at = parse_rfc3339_unix(seven_d_resets_at)?;

    Some(UsageResponse {
        email: None,
        plan_type: Some(subscription_type.to_string()),
        rate_limit: Some(UsageRateLimit {
            primary_window: Some(UsageWindow {
                used_percent: five_h_utilization.clamp(0.0, 100.0),
                limit_window_seconds: 18_000,
                reset_at: five_h_reset_at,
                reset_after_seconds: (five_h_reset_at - now).max(0),
            }),
            secondary_window: Some(UsageWindow {
                used_percent: seven_d_utilization.clamp(0.0, 100.0),
                limit_window_seconds: 604_800,
                reset_at: seven_d_reset_at,
                reset_after_seconds: (seven_d_reset_at - now).max(0),
            }),
        }),
    })
}

/// Parse an RFC3339 datetime string like "2025-11-04T04:59:59.943648+00:00"
/// to a Unix timestamp in seconds.  Only handles UTC / +00:00 offsets (Claude
/// always returns UTC).  Returns None on any parse failure.
pub fn parse_rfc3339_unix(s: &str) -> Option<i64> {
    // Strip sub-seconds and timezone:  "2025-11-04T04:59:59"
    let s = s.trim();
    let t = s.find('T')?;
    let date = &s[..t];        // "2025-11-04"
    let rest = &s[t + 1..];    // "04:59:59.943648+00:00"
    // Stop at '.' or '+' or 'Z'
    let time_end = rest.find(|c| c == '.' || c == '+' || c == 'Z').unwrap_or(rest.len());
    let time = &rest[..time_end]; // "04:59:59"

    let dp: Vec<i64> = date.split('-').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let tp: Vec<i64> = time.split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    if dp.len() < 3 || tp.len() < 3 { return None; }

    let (year, month, day) = (dp[0], dp[1], dp[2]);
    let (hour, minute, second) = (tp[0], tp[1], tp[2]);

    // Days from 1970-01-01 using civil_from_days / Gregorian calendar
    let days = days_since_epoch(year, month, day)?;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Compute days since Unix epoch (1970-01-01) for a Gregorian date.
fn days_since_epoch(year: i64, month: i64, day: i64) -> Option<i64> {
    if month < 1 || month > 12 || day < 1 || day > 31 { return None; }
    // Algorithm: days_from_civil from http://howardhinnant.github.io/date_algorithms.html
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 9 } else { month - 3 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;          // [0, 399]
    let doy = (153 * m + 2) / 5 + day - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days)
}

/// HTTP fetcher for Claude usage. Signature matches UsageService::with_fetcher.
/// account_id is ignored (Claude has no such concept); access_token is the OAuth token.
/// subscription_type is embedded in the access token but we pass it via account_id workaround:
/// caller passes "claude-<id>|<subscription_type>" as account_id.
pub fn fetch_claude_usage(account_id: &str, access_token: &str) -> anyhow::Result<UsageResponse> {
    let subscription_type = account_id
        .rsplit('|')
        .next()
        .unwrap_or("unknown");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let client = reqwest::blocking::Client::builder()
        .build()
        .context("build reqwest client")?;
    let raw: Value = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(access_token)
        .header("User-Agent", "codex-auth")
        .send()
        .context("send Claude usage request")?
        .error_for_status()
        .context("Claude usage request failed")?
        .json()
        .context("parse Claude usage JSON")?;

    map_claude_usage_response(&raw, subscription_type, now)
        .ok_or_else(|| anyhow::anyhow!("unexpected Claude usage response shape"))
}
```

Add `use anyhow::Context;` at the top of claude.rs (next to existing imports).

- [ ] **Step 3: Add reqwest import at top of claude.rs**

The file already has `use anyhow::{...}` and `use serde::{...}` — add:
```rust
use reqwest;
```
(reqwest is already in Cargo.toml — no new dependency needed.)

- [ ] **Step 4: Run tests**

```bash
cd rust/plot-viewer && cargo test claude 2>&1
```
Expected: all 3 claude tests pass.

Note: `fetch_claude_usage_maps_response_correctly` tests `map_claude_usage_response` directly (no HTTP), so it passes without network.

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/claude.rs
git commit -m "feat(claude): usage fetcher, RFC3339 parser, map to UsageResponse"
```

---

## Chunk 2: app.rs + main.rs integration

### Task 4: ProfileKind + load Claude profiles

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`
- Modify: `rust/plot-viewer/src/main.rs`

**Context:**
- `ProfileEntry` currently has no service indicator; we add `kind: ProfileKind`
- `App::load()` currently takes `(store, usage_service, cron_status)` — add `claude_store: Option<ClaudeStore>` + `claude_usage_service: Option<UsageService>`
- `load_profiles()` is a free function; extend it to also load Claude entries

**Note on Claude account_id convention:**
Claude's `UsageService` doesn't get a real `account_id` from the API. We store in `ProfileEntry.account_id` a composite key `"claude-<stable_prefix>|<subscription_type>"`. The `|` separator lets `fetch_claude_usage` extract `subscription_type` from the `account_id` field (see Task 3). This is an internal convention; never shown to the user.

- [ ] **Step 1: Add `ProfileKind` enum and `kind` field to `ProfileEntry`**

In `app.rs`, after the `PaneFocus` enum (around line 38):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileKind {
    Codex,
    Claude,
}
```

In `ProfileEntry` struct (around line 40), add one field:
```rust
struct ProfileEntry {
    kind: ProfileKind,          // ← add
    saved_name: Option<String>,
    profile_name: String,
    // ... rest unchanged
}
```

Fix all existing `ProfileEntry { ... }` constructors:
- In `from_profile_names()` (line ~171): add `kind: ProfileKind::Codex,`
- In `build_saved_entry()` (line ~862): add `kind: ProfileKind::Codex,`
- In `load_profiles()` current-unsaved branch (line ~823): add `kind: ProfileKind::Codex,`

- [ ] **Step 2: Add Claude fields to `App` struct**

```rust
pub struct App {
    // ... existing fields ...
    claude_store: Option<crate::claude::ClaudeStore>,
    claude_usage_service: Option<UsageService>,
}
```

Update `App::load()` signature:

```rust
pub fn load(
    store: AccountStore,
    usage_service: UsageService,
    cron_status: CronStatus,
    claude_store: Option<crate::claude::ClaudeStore>,
    claude_usage_service: Option<UsageService>,
) -> Result<Self> {
    let profiles = load_profiles(&store, &usage_service, false, None,
                                 claude_store.as_ref(), claude_usage_service.as_ref())?;
    Ok(Self {
        // ... existing fields ...
        claude_store,
        claude_usage_service,
    })
}
```

Update `App::from_profile_names()` (test constructor):
```rust
Self {
    // ... existing ...
    claude_store: None,
    claude_usage_service: None,
}
```

- [ ] **Step 3: Extend `load_profiles` to also load Claude entries**

Change `load_profiles` signature:

```rust
fn load_profiles(
    store: &AccountStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    claude_store: Option<&crate::claude::ClaudeStore>,
    claude_usage_service: Option<&UsageService>,
) -> Result<Vec<ProfileEntry>> {
    // ... existing Codex loading code unchanged ...

    // --- Claude entries ---
    if let (Some(cs), Some(cu)) = (claude_store, claude_usage_service) {
        let claude_entries = load_claude_profiles(cs, cu, force_refresh, refresh_account_id)?;
        profiles.extend(claude_entries);
    }

    profiles.sort_by(|left, right| left.profile_name.cmp(&right.profile_name));
    Ok(profiles)
}
```

Add the new `load_claude_profiles` function:

```rust
fn load_claude_profiles(
    store: &crate::claude::ClaudeStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
) -> Result<Vec<ProfileEntry>> {
    let saved = store.list_saved_profiles()?;
    let current_creds = store.get_current_credentials().ok();
    let current_account_id = current_creds.as_ref().map(|c| c.account_id());
    let current_access_token = current_creds.as_ref().map(|c| c.access_token().to_string());
    let current_sub_type = current_creds.as_ref().map(|c| c.subscription_type().to_string());

    // Helper: composite key for UsageService cache
    let composite_id = |creds: &crate::claude::ClaudeCredentials| {
        format!("{}|{}", creds.account_id(), creds.subscription_type())
    };

    let mut profiles: Vec<ProfileEntry> = saved
        .into_iter()
        .map(|saved_profile| {
            let snapshot = &saved_profile.snapshot;
            let creds: Option<crate::claude::ClaudeCredentials> =
                serde_json::from_value(snapshot.clone()).ok();
            let (acct_id, access_tok, comp_id) = match &creds {
                Some(c) => (
                    Some(c.account_id()),
                    Some(c.access_token().to_string()),
                    Some(composite_id(c)),
                ),
                None => (None, None, None),
            };
            let force_this = force_refresh
                || refresh_account_id.is_some_and(|t| acct_id.as_deref() == Some(t));
            let usage_view = usage_service.read_usage(
                comp_id.as_deref(),
                access_tok.as_deref(),
                force_this,
                false,
            )?;
            usage_service.record_usage_snapshot(comp_id.as_deref(), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                comp_id.as_deref(),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            Ok(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: Some(saved_profile.name.clone()),
                profile_name: saved_profile.name,
                snapshot: saved_profile.snapshot,
                usage_view,
                account_id: acct_id,
                is_current: current_account_id.as_deref() == acct_id.as_deref(),
                chart_data,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Add unsaved current Claude profile if not already in saved list
    if let (Some(creds), Some(access_tok)) = (&current_creds, &current_access_token) {
        let acct_id = creds.account_id();
        let comp_id = composite_id(creds);
        let sub_type = current_sub_type.as_deref().unwrap_or("unknown");
        let already_saved = profiles.iter().any(|p| p.account_id.as_deref() == Some(&acct_id));
        if !already_saved {
            let force_current = force_refresh
                || refresh_account_id.is_some_and(|t| t == acct_id.as_str());
            let snapshot = store.get_current_snapshot().unwrap_or(serde_json::json!({}));
            let usage_view = usage_service.read_usage(
                Some(&comp_id),
                Some(access_tok.as_str()),
                force_current,
                false,
            )?;
            usage_service.record_usage_snapshot(Some(&comp_id), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                Some(&comp_id),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            profiles.push(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: None,
                profile_name: format!("{} [cl-unsaved]", sub_type),
                snapshot,
                usage_view,
                account_id: Some(acct_id),
                is_current: true,
                chart_data,
            });
        }
    }

    Ok(profiles)
}
```

- [ ] **Step 4: Fix all call sites of load_profiles in app.rs**

`reload_profiles` calls `load_profiles`. Update it:

```rust
fn reload_profiles(&mut self, force_refresh: bool, refresh_account_id: Option<String>) -> Result<()> {
    let (Some(store), Some(usage_service)) = (self.store.as_ref(), self.usage_service.as_ref()) else {
        return Ok(());
    };
    self.profiles = load_profiles(
        store,
        usage_service,
        force_refresh,
        refresh_account_id.as_deref(),
        self.claude_store.as_ref(),
        self.claude_usage_service.as_ref(),
    )?;
    self.selected_profile_index = self
        .selected_profile_index
        .min(self.profiles.len().saturating_sub(1));
    if !self.y_zoom_user_adjusted {
        self.y_zoom_lower = auto_y_lower(&self.profiles);
    }
    Ok(())
}
```

- [ ] **Step 5: Update main.rs to instantiate Claude store + service**

```rust
use codex_auth::claude::{ClaudePaths, ClaudeStore};

fn main() -> Result<()> {
    // ... existing args + cron + codex setup unchanged ...

    // Claude setup
    let claude_paths = ClaudePaths::detect();
    let (claude_store, claude_usage_service) = if claude_paths.credentials_path().exists() {
        let store = ClaudeStore::new(claude_paths.clone());
        let usage = UsageService::new(
            claude_paths.limit_cache_path().to_path_buf(),
            claude_paths.usage_history_path().to_path_buf(),
            300,
        ).with_fetcher(crate::claude::fetch_claude_usage);
        // NOTE: with_fetcher is on UsageService; import path is codex_auth::usage::UsageService
        (Some(store), Some(usage))
    } else {
        (None, None)
    };

    let store = AccountStore::new(paths.clone(), StorePlatform::detect());
    let usage = UsageService::new(
        paths.limit_cache_path().to_path_buf(),
        paths.usage_history_path().to_path_buf(),
        300,
    );
    let mut app = App::load(store, usage, cron_status, claude_store, claude_usage_service)?;
    app.run()
}
```

Also update `run_refresh_all()` in main.rs — it doesn't need Claude refresh (out of scope), so just ensure it still compiles by not passing Claude args to App::load (it doesn't call App::load; it's fine).

- [ ] **Step 6: Confirm build + tests pass**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: all existing tests pass; 0 errors.

- [ ] **Step 7: Commit**

```bash
git add rust/plot-viewer/src/app.rs rust/plot-viewer/src/main.rs
git commit -m "feat(app): ProfileKind, load Claude profiles alongside Codex"
```

---

### Task 5: activate / save / rename / delete for Claude

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

- [ ] **Step 1: Branch `activate_selected_profile` on kind**

Current code (around line 469) unconditionally calls `store.use_account`. Replace:

```rust
fn activate_selected_profile(&mut self) -> Result<()> {
    let Some(profile) = self.selected_profile().cloned() else {
        return Ok(());
    };

    match profile.kind {
        ProfileKind::Claude => {
            if let Some(saved_name) = profile.saved_name.as_deref() {
                if let Some(cs) = self.claude_store.as_ref() {
                    let activated = cs.use_account(saved_name)?;
                    self.status_message = Some(format!("Switched Claude auth to \"{activated}\"."));
                    self.reload_profiles(false, profile.account_id.clone())?;
                }
            } else {
                // Unsaved Claude profile → open SaveCurrent dialog
                self.dialog = Some(DialogState {
                    mode: DialogMode::SaveCurrent,
                    input: build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot),
                });
            }
        }
        ProfileKind::Codex => {
            if let Some(saved_name) = profile.saved_name.as_deref() {
                if let Some(store) = self.store.as_ref() {
                    let activated = store.use_account(saved_name)?;
                    self.status_message = Some(format!("Switched Codex auth to \"{activated}\"."));
                    self.reload_profiles(false, profile.account_id.clone())?;
                }
            } else {
                self.dialog = Some(DialogState {
                    mode: DialogMode::SaveCurrent,
                    input: build_default_name(profile.usage_view.usage.as_ref(), &profile.snapshot),
                });
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Branch `confirm_dialog` SaveCurrent on kind**

In `confirm_dialog`, the `DialogMode::SaveCurrent` arm currently calls `store.save_snapshot(...)`. We need to know which kind the selected profile is.

Store the kind in `DialogMode::SaveCurrent`:

```rust
enum DialogMode {
    SaveCurrent(ProfileKind),   // ← add kind
    RenameSaved(String),
    ConfirmDelete(String),
}
```

Update `activate_selected_profile` to pass the kind:
```rust
self.dialog = Some(DialogState {
    mode: DialogMode::SaveCurrent(profile.kind),
    input: ...,
});
```

Update `confirm_dialog`:
```rust
DialogMode::SaveCurrent(kind) => {
    let name = match kind {
        ProfileKind::Claude => {
            let Some(cs) = self.claude_store.as_ref() else { return Ok(()); };
            cs.save_snapshot(&dialog.input, &profile.snapshot)
                .with_context(|| format!("save Claude snapshot {}", dialog.input))?
        }
        ProfileKind::Codex => {
            let Some(store) = self.store.as_ref() else { return Ok(()); };
            store.save_snapshot(&dialog.input, &profile.snapshot)
                .with_context(|| format!("save snapshot {}", dialog.input))?
        }
    };
    self.status_message = Some(format!("Saved current profile as \"{name}\"."));
    self.dialog = None;
    self.reload_profiles(false, profile.account_id.clone())?;
}
```

Update `render_dialog` title match — `DialogMode::SaveCurrent(_)` (add `_` to ignore kind).

- [ ] **Step 3: Branch `rename_account` in `confirm_dialog`**

In `DialogMode::RenameSaved(current_name)` arm, detect kind from selected profile:

```rust
DialogMode::RenameSaved(current_name) => {
    let name = match self.selected_profile().map(|p| p.kind) {
        Some(ProfileKind::Claude) => {
            let Some(cs) = self.claude_store.as_ref() else { return Ok(()); };
            cs.rename_account(&current_name, &dialog.input)?
        }
        _ => {
            let Some(store) = self.store.as_ref() else { return Ok(()); };
            store.rename_account(&current_name, &dialog.input)?
        }
    };
    self.status_message = Some(format!("Renamed to \"{name}\"."));
    self.dialog = None;
    self.reload_profiles(false, None)?;
}
```

- [ ] **Step 4: Branch `delete_account` in `confirm_dialog`**

```rust
DialogMode::ConfirmDelete(target_name) => {
    if dialog.input.trim().eq_ignore_ascii_case("y")
        || dialog.input.trim().eq_ignore_ascii_case("yes")
    {
        match self.selected_profile().map(|p| p.kind) {
            Some(ProfileKind::Claude) => {
                let Some(cs) = self.claude_store.as_ref() else { return Ok(()); };
                cs.delete_account(&target_name)?;
            }
            _ => {
                let Some(store) = self.store.as_ref() else { return Ok(()); };
                store.delete_account(&target_name)?;
            }
        }
        self.status_message = Some(format!("Deleted \"{target_name}\"."));
        self.reload_profiles(false, None)?;
    }
    self.dialog = None;
}
```

- [ ] **Step 5: Build + test**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: all pass, 0 errors.

- [ ] **Step 6: Commit**

```bash
git add rust/plot-viewer/src/app.rs
git commit -m "feat(app): Claude branch in activate/save/rename/delete"
```

---

### Task 6: Render — kind labels in profile list + detail panel

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

- [ ] **Step 1: Add `[cl]`/`[cx]` service tag to list items**

In `render_left_pane`, change the badge line:

```rust
// Before:
Span::raw(format!(" {usage}")),

// After:
let svc_tag = match profile.kind {
    ProfileKind::Claude => "[cl]",
    ProfileKind::Codex  => "[cx]",
};
Span::raw(format!(" {svc_tag} {usage}")),
```

- [ ] **Step 2: Add kind label to detail panel**

In `render_account_detail`, change the "State:" line area to also show service:

```rust
// After the "State: {state_label}" line, add:
lines.push(Line::from(format!(
    "Service: {}",
    match profile.kind {
        ProfileKind::Claude => "Claude Code",
        ProfileKind::Codex  => "Codex",
    }
)));
```

(Add this after the `State:` line, before `Updated:`.)

Also update `render_account_detail` signature to accept `kind` — actually it already takes `Option<&ProfileEntry>` which has `kind`, so just read `profile.kind` directly inside.

- [ ] **Step 3: Update "No profile loaded" fallback text**

Change `"No Codex auth profile loaded."` to `"No profile loaded."` since we now serve both.

- [ ] **Step 4: Update footer to mention Claude when both panes active**

No change needed — footer is generic enough.

- [ ] **Step 5: Build + test**

```bash
cd rust/plot-viewer && cargo test 2>&1 && cargo build 2>&1
```
Expected: 0 errors, 0 warnings.

- [ ] **Step 6: Commit**

```bash
git add rust/plot-viewer/src/app.rs
git commit -m "feat(app): [cl]/[cx] service tags in profile list and detail panel"
```

---

## Verification Checklist

```bash
cd rust/plot-viewer
cargo build
cargo test
cargo run
```

目視確認：
- [ ] Profiles list shows both Codex and Claude entries with `[cx]`/`[cl]` badges
- [ ] Details panel shows `Service: Claude Code` or `Service: Codex`
- [ ] Tab to Accounts pane, select a Claude profile, Enter → switches `~/.claude/.credentials.json`
- [ ] Enter on unsaved Claude profile → opens save dialog → saves to `~/.claude/accounts/`
- [ ] `n` renames a Claude profile
- [ ] `d` deletes a Claude profile
- [ ] `u` refreshes Claude usage from API (hits `api.anthropic.com`)
- [ ] Chart shows Claude 7d usage curve alongside Codex curves
- [ ] If `~/.claude/.credentials.json` absent, only Codex entries appear (no crash)
