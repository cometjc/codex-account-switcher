use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::usage::{UsageRateLimit, UsageResponse, UsageWindow};

const CLAUDE_OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const CLAUDE_OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RefreshedClaudeTokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
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

    /// Overwrite a saved profile with the current credentials (e.g. after token rotation).
    pub fn update_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_credentials_exist()?;
        fs::copy(self.paths.credentials_path(), self.account_file_path(&name))
            .with_context(|| format!("update saved profile {}", name))?;
        Ok(name)
    }

    /// Write the current profile name file without switching credentials.
    pub fn set_current_name(&self, raw_name: &str) -> Result<()> {
        let name = normalize_account_name(raw_name)?;
        fs::write(self.paths.current_name_path(), format!("{name}\n"))
            .with_context(|| format!("write {}", self.paths.current_name_path().display()))
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

// ── Usage fetching ─────────────────────────────────────────────────────────────

/// Map a raw Claude usage API JSON response + subscription_type to UsageResponse.
/// `now` is the current unix timestamp in seconds (injected for testability).
pub fn map_claude_usage_response(raw: &Value, subscription_type: &str, now: i64) -> Option<UsageResponse> {
    let five_hour = raw.get("five_hour")?;
    let seven_day = raw.get("seven_day")?;

    let five_h_utilization = five_hour.get("utilization")?.as_f64()?;
    let five_h_resets_at = five_hour.get("resets_at")?.as_str()?;
    let seven_d_utilization = seven_day.get("utilization")?.as_f64()?;
    let seven_d_resets_at = seven_day.get("resets_at")?.as_str()?;

    // Round to nearest minute so sub-second jitter in the server's resets_at field
    // does not produce duplicate window entries across calls.
    let five_h_reset_at = parse_rfc3339_unix(five_h_resets_at).map(|t| (t + 30) / 60 * 60)?;
    let seven_d_reset_at = parse_rfc3339_unix(seven_d_resets_at).map(|t| (t + 30) / 60 * 60)?;

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
/// to a Unix timestamp in seconds. Only handles UTC / +00:00 offsets (Claude
/// always returns UTC). Returns None on any parse failure.
pub fn parse_rfc3339_unix(s: &str) -> Option<i64> {
    let s = s.trim();
    let t = s.find('T')?;
    let date = &s[..t];
    let rest = &s[t + 1..];
    let time_end = rest.find(|c| c == '.' || c == '+' || c == 'Z').unwrap_or(rest.len());
    let time = &rest[..time_end];

    let dp: Vec<i64> = date.split('-').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    let tp: Vec<i64> = time.split(':').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    if dp.len() < 3 || tp.len() < 3 { return None; }

    let (year, month, day) = (dp[0], dp[1], dp[2]);
    let (hour, minute, second) = (tp[0], tp[1], tp[2]);

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
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days)
}

/// HTTP fetcher for Claude usage. Signature matches UsageService::with_fetcher.
/// Caller passes "claude-<id>|<subscription_type>" as account_id so we can
/// extract subscription_type here without additional parameters.
pub fn fetch_claude_usage(account_id: &str, access_token: &str) -> anyhow::Result<UsageResponse> {
    let paths = ClaudePaths::detect();
    fetch_claude_usage_with_auto_refresh(
        &paths,
        account_id,
        access_token,
        fetch_claude_usage_once,
        refresh_claude_oauth_tokens,
    )
}

fn fetch_claude_usage_with_auto_refresh<FUsage, FRefresh>(
    paths: &ClaudePaths,
    account_id: &str,
    access_token: &str,
    mut usage_fetch: FUsage,
    mut refresh_tokens: FRefresh,
) -> anyhow::Result<UsageResponse>
where
    FUsage: FnMut(&str, &str) -> anyhow::Result<UsageResponse>,
    FRefresh: FnMut(&str) -> anyhow::Result<RefreshedClaudeTokens>,
{
    match usage_fetch(account_id, access_token) {
        Ok(usage) => Ok(usage),
        Err(error) if is_usage_refreshable_error(&error) => {
            let rotated_access_token =
                refresh_current_claude_credentials(paths, access_token, &mut refresh_tokens)?;
            usage_fetch(account_id, rotated_access_token.as_str())
        }
        Err(error) => Err(error),
    }
}

fn fetch_claude_usage_once(account_id: &str, access_token: &str) -> anyhow::Result<UsageResponse> {
    let subscription_type = account_id
        .rsplit('|')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let client = reqwest::blocking::Client::builder()
        .build()
        .context("build reqwest client")?;
    let response = build_claude_usage_request(&client, access_token)
        .send()
        .context("send Claude usage request")?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        bail!("Claude usage request failed: HTTP 429 Too Many Requests");
    }
    let raw: Value = response
        .error_for_status()
        .context("Claude usage request failed")?
        .json()
        .context("parse Claude usage JSON")?;

    map_claude_usage_response(&raw, subscription_type, now)
        .ok_or_else(|| anyhow::anyhow!("unexpected Claude usage response shape"))
}

fn build_claude_usage_request(
    client: &reqwest::blocking::Client,
    access_token: &str,
) -> reqwest::blocking::RequestBuilder {
    client
        .get("https://api.anthropic.com/api/oauth/usage")
        .bearer_auth(access_token)
        .header("User-Agent", "agent-switch")
        .header("anthropic-beta", CLAUDE_OAUTH_BETA_HEADER)
}

fn refresh_current_claude_credentials<FRefresh>(
    paths: &ClaudePaths,
    current_access_token: &str,
    refresh_tokens: &mut FRefresh,
) -> anyhow::Result<String>
where
    FRefresh: FnMut(&str) -> anyhow::Result<RefreshedClaudeTokens>,
{
    let mut snapshot = read_snapshot(paths.credentials_path())?;
    let oauth = snapshot
        .get_mut("claudeAiOauth")
        .and_then(Value::as_object_mut)
        .context("parse Claude credentials oauth payload")?;
    let stored_access_token = oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .context("Claude credentials missing accessToken")?
        .to_string();
    if stored_access_token != current_access_token {
        // Credentials were already rotated externally (e.g. by Claude Code) — use the fresher stored token
        return Ok(stored_access_token);
    }
    let stored_refresh_token = oauth
        .get("refreshToken")
        .and_then(Value::as_str)
        .context("Claude credentials missing refreshToken")?
        .to_string();

    let rotated = refresh_tokens(stored_refresh_token.as_str())?;
    oauth.insert(
        "accessToken".to_string(),
        Value::String(rotated.access_token.clone()),
    );
    oauth.insert(
        "refreshToken".to_string(),
        Value::String(rotated.refresh_token.clone()),
    );
    oauth.insert("expiresAt".to_string(), Value::from(rotated.expires_at));
    write_json(paths.credentials_path(), &snapshot)?;
    Ok(rotated.access_token)
}

fn refresh_claude_oauth_tokens(refresh_token: &str) -> anyhow::Result<RefreshedClaudeTokens> {
    let client = reqwest::blocking::Client::builder()
        .build()
        .context("build reqwest client")?;
    let raw: Value = client
        .post("https://console.anthropic.com/v1/oauth/token")
        .header("User-Agent", "agent-switch")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": CLAUDE_OAUTH_CLIENT_ID,
        }))
        .send()
        .context("send Claude token refresh request")?
        .error_for_status()
        .context("Claude token refresh failed")?
        .json()
        .context("parse Claude token refresh JSON")?;
    parse_refreshed_claude_tokens(&raw, current_unix_seconds())
}

fn parse_refreshed_claude_tokens(raw: &Value, now: i64) -> anyhow::Result<RefreshedClaudeTokens> {
    let access_token = raw
        .get("access_token")
        .or_else(|| raw.get("accessToken"))
        .and_then(Value::as_str)
        .context("Claude token refresh response missing access_token")?;
    let refresh_token = raw
        .get("refresh_token")
        .or_else(|| raw.get("refreshToken"))
        .and_then(Value::as_str)
        .context("Claude token refresh response missing refresh_token")?;
    let expires_at = raw
        .get("expires_at")
        .or_else(|| raw.get("expiresAt"))
        .and_then(Value::as_i64)
        .or_else(|| {
            raw.get("expires_in")
                .or_else(|| raw.get("expiresIn"))
                .and_then(Value::as_i64)
                .map(|seconds| now + seconds)
        })
        .context("Claude token refresh response missing expiry")?;
    Ok(RefreshedClaudeTokens {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        expires_at,
    })
}

fn is_usage_refreshable_error(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}").to_ascii_lowercase();
    message.contains("429")
        || message.contains("rate limited")
        || message.contains("401")
        || message.contains("unauthorized")
        || message.contains("invalid access token")
}

fn current_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir_pair() -> (PathBuf, PathBuf) {
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "claude-store-test-{}-{}",
            std::process::id(),
            unique
        ));
        let _ = fs::remove_dir_all(&base);
        let claude_dir = base.join("dot-claude");
        fs::create_dir_all(&claude_dir).unwrap();
        (claude_dir, base)
    }

    fn sample_creds_json() -> &'static str {
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-aaa","refreshToken":"sk-ant-ort01-bbb","expiresAt":9999999999,"subscriptionType":"pro","rateLimitTier":"x","scopes":[]}}"#
    }

    #[test]
    fn claude_store_list_save_use_roundtrip() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();
        let store = ClaudeStore::new(paths);

        assert!(store.list_account_names().unwrap().is_empty());

        let name = store.save_account("work").unwrap();
        assert_eq!(name, "work");
        assert_eq!(store.list_account_names().unwrap(), vec!["work"]);

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

    #[test]
    fn fetch_claude_usage_maps_response_correctly() {
        let now = 1_730_000_000_i64;
        let resets_at_5h = "2025-10-27T04:59:59.000000+00:00";
        let resets_at_7d = "2025-10-29T04:59:59.000000+00:00";
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

    #[test]
    fn usage_429_refreshes_current_token_once_and_persists_rotated_credentials() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();

        let mut seen_tokens = Vec::new();
        let usage = fetch_claude_usage_with_auto_refresh(
            &paths,
            "claude-bbb|pro",
            "sk-ant-oat01-aaa",
            |_, token: &str| {
                seen_tokens.push(token.to_string());
                match token {
                    "sk-ant-oat01-aaa" => Err(anyhow::anyhow!(
                        "Claude usage request failed: HTTP 429 Too Many Requests"
                    )),
                    "sk-ant-oat01-new" => Ok(UsageResponse {
                        email: None,
                        plan_type: Some("pro".to_string()),
                        rate_limit: Some(UsageRateLimit {
                            primary_window: Some(UsageWindow {
                                used_percent: 12.0,
                                limit_window_seconds: 18_000,
                                reset_at: 1_730_010_000,
                                reset_after_seconds: 600,
                            }),
                            secondary_window: Some(UsageWindow {
                                used_percent: 34.0,
                                limit_window_seconds: 604_800,
                                reset_at: 1_730_100_000,
                                reset_after_seconds: 9_000,
                            }),
                        }),
                    }),
                    other => Err(anyhow::anyhow!("unexpected token {other}")),
                }
            },
            |_refresh_token: &str| {
                Ok(RefreshedClaudeTokens {
                    access_token: "sk-ant-oat01-new".to_string(),
                    refresh_token: "sk-ant-ort01-new".to_string(),
                    expires_at: 1_730_200_000,
                })
            },
        )
        .unwrap();

        let persisted = read_credentials(paths.credentials_path()).unwrap();
        assert_eq!(usage.plan_type.as_deref(), Some("pro"));
        assert_eq!(
            seen_tokens,
            vec!["sk-ant-oat01-aaa".to_string(), "sk-ant-oat01-new".to_string()]
        );
        assert_eq!(persisted.access_token(), "sk-ant-oat01-new");
        assert_eq!(persisted.claude_ai_oauth.refresh_token, "sk-ant-ort01-new");
        assert_eq!(persisted.claude_ai_oauth.expires_at, 1_730_200_000);
    }

    #[test]
    fn usage_401_refreshes_current_token_once_and_persists_rotated_credentials() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();

        let mut seen_tokens = Vec::new();
        let usage = fetch_claude_usage_with_auto_refresh(
            &paths,
            "claude-bbb|pro",
            "sk-ant-oat01-aaa",
            |_, token: &str| {
                seen_tokens.push(token.to_string());
                match token {
                    "sk-ant-oat01-aaa" => {
                        Err(anyhow::anyhow!("Claude usage request failed: HTTP 401 Unauthorized"))
                    }
                    "sk-ant-oat01-new" => Ok(UsageResponse {
                        email: None,
                        plan_type: Some("pro".to_string()),
                        rate_limit: Some(UsageRateLimit {
                            primary_window: Some(UsageWindow {
                                used_percent: 12.0,
                                limit_window_seconds: 18_000,
                                reset_at: 1_730_010_000,
                                reset_after_seconds: 600,
                            }),
                            secondary_window: Some(UsageWindow {
                                used_percent: 34.0,
                                limit_window_seconds: 604_800,
                                reset_at: 1_730_100_000,
                                reset_after_seconds: 9_000,
                            }),
                        }),
                    }),
                    other => Err(anyhow::anyhow!("unexpected token {other}")),
                }
            },
            |_refresh_token: &str| {
                Ok(RefreshedClaudeTokens {
                    access_token: "sk-ant-oat01-new".to_string(),
                    refresh_token: "sk-ant-ort01-new".to_string(),
                    expires_at: 1_730_200_000,
                })
            },
        )
        .unwrap();

        let persisted = read_credentials(paths.credentials_path()).unwrap();
        assert_eq!(usage.plan_type.as_deref(), Some("pro"));
        assert_eq!(
            seen_tokens,
            vec!["sk-ant-oat01-aaa".to_string(), "sk-ant-oat01-new".to_string()]
        );
        assert_eq!(persisted.access_token(), "sk-ant-oat01-new");
        assert_eq!(persisted.claude_ai_oauth.refresh_token, "sk-ant-ort01-new");
        assert_eq!(persisted.claude_ai_oauth.expires_at, 1_730_200_000);
    }

    #[test]
    fn claude_usage_request_includes_oauth_beta_header() {
        let client = reqwest::blocking::Client::new();

        let request = build_claude_usage_request(&client, "test-access-token")
            .build()
            .unwrap();

        assert_eq!(
            request
                .headers()
                .get("anthropic-beta")
                .and_then(|value: &reqwest::header::HeaderValue| value.to_str().ok()),
            Some(CLAUDE_OAUTH_BETA_HEADER)
        );
    }
}
