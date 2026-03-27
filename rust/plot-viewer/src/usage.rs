use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageWindow {
    pub used_percent: f64,
    pub limit_window_seconds: i64,
    pub reset_after_seconds: i64,
    pub reset_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageRateLimit {
    pub primary_window: Option<UsageWindow>,
    pub secondary_window: Option<UsageWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageResponse {
    pub email: Option<String>,
    pub plan_type: Option<String>,
    pub rate_limit: Option<UsageRateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageCacheRecord {
    #[serde(rename = "fetchedAt")]
    pub fetched_at: i64,
    pub usage: UsageResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UsageCache {
    #[serde(rename = "byAccountId")]
    pub by_account_id: BTreeMap<String, UsageCacheRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageObservation {
    pub observed_at: i64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UsageWindowHistory {
    pub limit_window_seconds: i64,
    pub start_at: i64,
    pub end_at: i64,
    pub observations: Vec<UsageObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProfileUsageHistory {
    #[serde(default)]
    pub weekly_windows: Vec<UsageWindowHistory>,
    #[serde(default)]
    pub five_hour_windows: Vec<UsageWindowHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UsageHistoryCache {
    #[serde(rename = "byAccountId")]
    pub by_account_id: BTreeMap<String, ProfileUsageHistory>,
}

impl UsageCache {
    pub fn from_entries(
        entries: impl IntoIterator<Item = (String, i64, UsageResponse)>,
    ) -> Self {
        let by_account_id = entries
            .into_iter()
            .map(|(account_id, fetched_at, usage)| {
                (
                    account_id,
                    UsageCacheRecord {
                        fetched_at,
                        usage,
                    },
                )
            })
            .collect();
        Self { by_account_id }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSource {
    Api,
    Cache,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageReadResult {
    pub usage: Option<UsageResponse>,
    pub source: UsageSource,
    pub fetched_at: Option<i64>,
    pub stale: bool,
}

const CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_OAUTH_REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_REFRESH_INTERVAL_DAYS: i64 = 8;

#[derive(Debug, Clone, PartialEq)]
struct CodexAuthTokens {
    account_id: String,
    access_token: String,
    refresh_token: Option<String>,
    last_refresh: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
struct RefreshedCodexTokens {
    id_token: Option<String>,
    access_token: String,
    refresh_token: String,
}

type Fetcher = dyn Fn(&str, &str) -> Result<UsageResponse> + Send + Sync;
type Clock = dyn Fn() -> i64 + Send + Sync;

#[derive(Clone)]
pub struct UsageService {
    cache_path: PathBuf,
    history_path: PathBuf,
    ttl_seconds: i64,
    clock: Arc<Clock>,
    fetcher: Arc<Fetcher>,
}

impl UsageService {
    pub fn new(cache_path: PathBuf, history_path: PathBuf, ttl_seconds: i64) -> Self {
        Self {
            cache_path,
            history_path,
            ttl_seconds,
            clock: Arc::new(default_now_seconds),
            fetcher: Arc::new(fetch_usage_from_api),
        }
    }

    pub fn with_fetcher<F>(mut self, fetcher: F) -> Self
    where
        F: Fn(&str, &str) -> Result<UsageResponse> + Send + Sync + 'static,
    {
        self.fetcher = Arc::new(fetcher);
        self
    }

    pub fn with_now_seconds(mut self, now_seconds: i64) -> Self {
        self.clock = Arc::new(move || now_seconds);
        self
    }

    pub fn read_usage(
        &self,
        account_id: Option<&str>,
        access_token: Option<&str>,
        force_refresh: bool,
        cache_only: bool,
    ) -> Result<UsageReadResult> {
        let now_seconds = (self.clock)();
        let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            });
        };
        let Some(access_token) = access_token.filter(|value| !value.trim().is_empty()) else {
            return Ok(UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            });
        };

        let mut cache = self.read_cache()?;
        let cached = cache.by_account_id.get(account_id).cloned();
        let age = cached
            .as_ref()
            .map(|record| now_seconds - record.fetched_at)
            .unwrap_or(i64::MAX);

        if !force_refresh {
            if let Some(record) = cached.as_ref() {
                if cache_only || age <= self.ttl_seconds {
                    return Ok(UsageReadResult {
                        usage: Some(record.usage.clone()),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: false,
                    });
                }
            }
        }

        if cache_only {
            return Ok(cached
                .map(|record| UsageReadResult {
                    usage: Some(record.usage),
                    source: UsageSource::Cache,
                    fetched_at: Some(record.fetched_at),
                    stale: age > self.ttl_seconds,
                })
                .unwrap_or(UsageReadResult {
                    usage: None,
                    source: UsageSource::None,
                    fetched_at: None,
                    stale: false,
                }));
        }

        match (self.fetcher)(account_id, access_token) {
            Ok(usage) => {
                let fetched_at = now_seconds;
                cache.by_account_id.insert(
                    account_id.to_string(),
                    UsageCacheRecord {
                        fetched_at,
                        usage: usage.clone(),
                    },
                );
                self.write_cache(&cache)?;
                Ok(UsageReadResult {
                    usage: Some(usage),
                    source: UsageSource::Api,
                    fetched_at: Some(fetched_at),
                    stale: false,
                })
            }
            Err(error) => {
                if force_refresh {
                    return Err(error);
                }
                Ok(cached
                    .map(|record| UsageReadResult {
                        usage: Some(record.usage),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: true,
                    })
                    .unwrap_or(UsageReadResult {
                        usage: None,
                        source: UsageSource::None,
                        fetched_at: None,
                        stale: false,
                    }))
            }
        }
    }

    pub fn read_codex_usage(
        &self,
        auth_path: &Path,
        force_refresh: bool,
        cache_only: bool,
    ) -> Result<UsageReadResult> {
        let now_seconds = (self.clock)();
        let Some(tokens) = read_codex_auth_tokens(auth_path)? else {
            return Ok(UsageReadResult {
                usage: None,
                source: UsageSource::None,
                fetched_at: None,
                stale: false,
            });
        };

        let account_id = tokens.account_id.as_str();
        let mut cache = self.read_cache()?;
        let cached = cache.by_account_id.get(account_id).cloned();
        let age = cached
            .as_ref()
            .map(|record| now_seconds - record.fetched_at)
            .unwrap_or(i64::MAX);

        if !force_refresh {
            if let Some(record) = cached.as_ref() {
                if cache_only || age <= self.ttl_seconds {
                    return Ok(UsageReadResult {
                        usage: Some(record.usage.clone()),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: false,
                    });
                }
            }
        }

        if cache_only {
            return Ok(cached
                .map(|record| UsageReadResult {
                    usage: Some(record.usage),
                    source: UsageSource::Cache,
                    fetched_at: Some(record.fetched_at),
                    stale: age > self.ttl_seconds,
                })
                .unwrap_or(UsageReadResult {
                    usage: None,
                    source: UsageSource::None,
                    fetched_at: None,
                    stale: false,
                }));
        }

        match fetch_codex_usage_from_auth(auth_path, &tokens) {
            Ok(usage) => {
                let fetched_at = now_seconds;
                cache.by_account_id.insert(
                    account_id.to_string(),
                    UsageCacheRecord {
                        fetched_at,
                        usage: usage.clone(),
                    },
                );
                self.write_cache(&cache)?;
                Ok(UsageReadResult {
                    usage: Some(usage),
                    source: UsageSource::Api,
                    fetched_at: Some(fetched_at),
                    stale: false,
                })
            }
            Err(error) => {
                if force_refresh {
                    return Err(error);
                }
                Ok(cached
                    .map(|record| UsageReadResult {
                        usage: Some(record.usage),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: true,
                    })
                    .unwrap_or(UsageReadResult {
                        usage: None,
                        source: UsageSource::None,
                        fetched_at: None,
                        stale: false,
                    }))
            }
        }
    }

    pub fn record_usage_snapshot(&self, account_id: Option<&str>, usage: Option<&UsageResponse>) -> Result<()> {
        let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(());
        };
        let Some(usage) = usage else {
            return Ok(());
        };
        let Some(rate_limit) = usage.rate_limit.as_ref() else {
            return Ok(());
        };

        let mut history = self.read_history_cache()?;
        let entry = history.by_account_id.entry(account_id.to_string()).or_default();
        let observed_at = (self.clock)();

        for window in [&rate_limit.primary_window, &rate_limit.secondary_window]
            .into_iter()
            .flatten()
        {
            let start_at = window.reset_at - window.limit_window_seconds;
            let target_windows = match window.limit_window_seconds {
                604_800 => &mut entry.weekly_windows,
                18_000 => &mut entry.five_hour_windows,
                _ => continue,
            };
            let target = target_windows
                .iter_mut()
                .find(|candidate| {
                    candidate.limit_window_seconds == window.limit_window_seconds
                        && candidate.start_at == start_at
                        && candidate.end_at == window.reset_at
                });

            let observation = UsageObservation {
                observed_at,
                used_percent: window.used_percent.clamp(0.0, 100.0),
            };
            if let Some(target) = target {
                let duplicate = target
                    .observations
                    .last()
                    .is_some_and(|last| last.observed_at == observation.observed_at && (last.used_percent - observation.used_percent).abs() < f64::EPSILON);
                if !duplicate {
                    target.observations.push(observation);
                }
            } else {
                target_windows.push(UsageWindowHistory {
                    limit_window_seconds: window.limit_window_seconds,
                    start_at,
                    end_at: window.reset_at,
                    observations: vec![observation],
                });
            }
            let max_count = match window.limit_window_seconds {
                604_800 => 3,
                18_000 => 34,
                _ => 6,
            };
            trim_history_windows(target_windows, max_count);
        }

        self.write_history_cache(&history)
    }

    pub fn profile_history(&self, account_id: Option<&str>) -> Result<ProfileUsageHistory> {
        let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(ProfileUsageHistory::default());
        };
        let history = self.read_history_cache()?;
        Ok(history
            .by_account_id
            .get(account_id)
            .cloned()
            .unwrap_or_default())
    }

    pub fn merge_profile_history_aliases<'a>(
        &self,
        canonical_account_id: Option<&str>,
        alias_account_ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<()> {
        let Some(canonical_account_id) =
            canonical_account_id.filter(|value| !value.trim().is_empty())
        else {
            return Ok(());
        };
        let mut history = self.read_history_cache()?;
        let mut changed = false;
        for alias_account_id in alias_account_ids
            .into_iter()
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != canonical_account_id)
        {
            let Some(alias_history) = history.by_account_id.remove(alias_account_id) else {
                continue;
            };
            let canonical_history = history
                .by_account_id
                .entry(canonical_account_id.to_string())
                .or_default();
            merge_profile_usage_history(canonical_history, alias_history);
            changed = true;
        }
        if changed {
            self.write_history_cache(&history)?;
        }
        Ok(())
    }

    fn read_cache(&self) -> Result<UsageCache> {
        match fs::read_to_string(&self.cache_path) {
            Ok(raw) => serde_json::from_str(&raw).or(Ok(UsageCache::default())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(UsageCache::default()),
            Err(error) => Err(error).with_context(|| format!("read {}", self.cache_path.display())),
        }
    }

    fn write_cache(&self, cache: &UsageCache) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        fs::write(
            &self.cache_path,
            format!("{}\n", serde_json::to_string_pretty(cache)?),
        )
        .with_context(|| format!("write {}", self.cache_path.display()))
    }

    fn read_history_cache(&self) -> Result<UsageHistoryCache> {
        match fs::read_to_string(&self.history_path) {
            Ok(raw) => serde_json::from_str(&raw).or(Ok(UsageHistoryCache::default())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(UsageHistoryCache::default()),
            Err(error) => Err(error).with_context(|| format!("read {}", self.history_path.display())),
        }
    }

    fn write_history_cache(&self, history: &UsageHistoryCache) -> Result<()> {
        if let Some(parent) = self.history_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        fs::write(
            &self.history_path,
            format!("{}\n", serde_json::to_string_pretty(history)?),
        )
        .with_context(|| format!("write {}", self.history_path.display()))
    }
}

fn trim_history_windows(windows: &mut Vec<UsageWindowHistory>, max_count: usize) {
    windows.sort_by_key(|window| (window.start_at, window.end_at));
    while windows.len() > max_count {
        windows.remove(0);
    }
    for window in windows {
        window.observations.sort_by_key(|observation| observation.observed_at);
        window
            .observations
            .retain(|observation| observation.observed_at >= window.start_at && observation.observed_at <= window.end_at);
    }
}

fn merge_profile_usage_history(target: &mut ProfileUsageHistory, source: ProfileUsageHistory) {
    merge_usage_window_histories(&mut target.weekly_windows, source.weekly_windows, 3);
    merge_usage_window_histories(&mut target.five_hour_windows, source.five_hour_windows, 34);
}

fn merge_usage_window_histories(
    target: &mut Vec<UsageWindowHistory>,
    source: Vec<UsageWindowHistory>,
    max_count: usize,
) {
    for mut source_window in source {
        if let Some(existing) = target.iter_mut().find(|candidate| {
            candidate.limit_window_seconds == source_window.limit_window_seconds
                && candidate.start_at == source_window.start_at
                && candidate.end_at == source_window.end_at
        }) {
            existing.observations.append(&mut source_window.observations);
            dedupe_observations(&mut existing.observations);
        } else {
            dedupe_observations(&mut source_window.observations);
            target.push(source_window);
        }
    }
    trim_history_windows(target, max_count);
}

fn dedupe_observations(observations: &mut Vec<UsageObservation>) {
    observations.sort_by_key(|observation| observation.observed_at);
    observations.dedup_by(|right, left| {
        right.observed_at == left.observed_at
            && (right.used_percent - left.used_percent).abs() < f64::EPSILON
    });
}

fn fetch_usage_from_api(account_id: &str, access_token: &str) -> Result<UsageResponse> {
    fetch_usage_from_api_once(account_id, access_token)
}

fn fetch_codex_usage_from_auth(auth_path: &Path, initial_tokens: &CodexAuthTokens) -> Result<UsageResponse> {
    fetch_codex_usage_with_auto_refresh(
        auth_path,
        initial_tokens,
        fetch_usage_from_api_once,
        refresh_codex_oauth_tokens,
    )
}

fn fetch_codex_usage_with_auto_refresh<FUsage, FRefresh>(
    auth_path: &Path,
    initial_tokens: &CodexAuthTokens,
    mut usage_fetch: FUsage,
    mut refresh_tokens: FRefresh,
) -> Result<UsageResponse>
where
    FUsage: FnMut(&str, &str) -> Result<UsageResponse>,
    FRefresh: FnMut(&str) -> Result<RefreshedCodexTokens>,
{
    let mut active_tokens = initial_tokens.clone();
    if should_refresh_codex_auth(&active_tokens) {
        active_tokens =
            refresh_current_codex_auth(auth_path, Some(active_tokens.access_token.as_str()), &mut refresh_tokens)?;
    }

    match usage_fetch(&active_tokens.account_id, &active_tokens.access_token) {
        Ok(usage) => Ok(usage),
        Err(error) if is_codex_usage_refreshable_error(&error) => {
            let refreshed_tokens = refresh_current_codex_auth(
                auth_path,
                Some(active_tokens.access_token.as_str()),
                &mut refresh_tokens,
            )?;
            usage_fetch(&refreshed_tokens.account_id, &refreshed_tokens.access_token)
        }
        Err(error) => Err(error),
    }
}

fn fetch_usage_from_api_once(account_id: &str, access_token: &str) -> Result<UsageResponse> {
    let client = Client::builder().build().context("build reqwest client")?;
    let response = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .bearer_auth(access_token)
        .header("ChatGPT-Account-Id", account_id)
        .header("User-Agent", "agent-switch")
        .send()
        .context("send usage request")?
        .error_for_status()
        .context("usage request failed")?;
    response.json().context("parse usage response JSON")
}

fn read_codex_auth_tokens(auth_path: &Path) -> Result<Option<CodexAuthTokens>> {
    let raw = match fs::read_to_string(auth_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("read {}", auth_path.display())),
    };
    let snapshot: Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", auth_path.display()))?;
    Ok(extract_codex_auth_tokens(&snapshot))
}

fn extract_codex_auth_tokens(snapshot: &Value) -> Option<CodexAuthTokens> {
    let tokens = snapshot.get("tokens")?;
    let account_id = tokens.get("account_id")?.as_str()?.trim();
    let access_token = tokens.get("access_token")?.as_str()?.trim();
    if account_id.is_empty() || access_token.is_empty() {
        return None;
    }
    let refresh_token = tokens
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let last_refresh = snapshot
        .get("last_refresh")
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc));
    Some(CodexAuthTokens {
        account_id: account_id.to_string(),
        access_token: access_token.to_string(),
        refresh_token,
        last_refresh,
    })
}

fn refresh_current_codex_auth<FRefresh>(
    auth_path: &Path,
    current_access_token: Option<&str>,
    refresh_tokens: &mut FRefresh,
) -> Result<CodexAuthTokens>
where
    FRefresh: FnMut(&str) -> Result<RefreshedCodexTokens>,
{
    let raw = fs::read_to_string(auth_path)
        .with_context(|| format!("read {}", auth_path.display()))?;
    let mut snapshot: Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", auth_path.display()))?;
    let stored_tokens = extract_codex_auth_tokens(&snapshot)
        .with_context(|| format!("parse Codex auth tokens from {}", auth_path.display()))?;
    if current_access_token.is_some_and(|token| token != stored_tokens.access_token) {
        return Ok(stored_tokens);
    }

    let refresh_token = stored_tokens
        .refresh_token
        .as_deref()
        .context("Codex auth snapshot missing refresh_token")?;
    let rotated = refresh_tokens(refresh_token)?;
    let tokens = snapshot
        .get_mut("tokens")
        .and_then(Value::as_object_mut)
        .context("parse Codex auth token payload")?;
    tokens.insert(
        "access_token".to_string(),
        Value::String(rotated.access_token.clone()),
    );
    tokens.insert(
        "refresh_token".to_string(),
        Value::String(rotated.refresh_token.clone()),
    );
    if let Some(id_token) = rotated.id_token.clone() {
        tokens.insert("id_token".to_string(), Value::String(id_token));
    }
    let refreshed_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
    snapshot["last_refresh"] = Value::String(refreshed_at);
    write_json(auth_path, &snapshot)?;
    extract_codex_auth_tokens(&snapshot)
        .with_context(|| format!("parse refreshed Codex auth tokens from {}", auth_path.display()))
}

fn refresh_codex_oauth_tokens(refresh_token: &str) -> Result<RefreshedCodexTokens> {
    let client = Client::builder().build().context("build reqwest client")?;
    let response = client
        .post(CODEX_OAUTH_REFRESH_URL)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "client_id": CODEX_OAUTH_CLIENT_ID,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }))
        .send()
        .context("send Codex token refresh request")?;
    let status = response.status();
    let raw: Value = response
        .json()
        .with_context(|| format!("parse Codex token refresh JSON ({status})"))?;
    if !status.is_success() {
        let message = raw
            .get("error_description")
            .or_else(|| raw.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("unknown Codex token refresh failure");
        anyhow::bail!("Codex token refresh failed: HTTP {status} {message}");
    }
    parse_refreshed_codex_tokens(&raw)
}

fn parse_refreshed_codex_tokens(raw: &Value) -> Result<RefreshedCodexTokens> {
    let access_token = raw
        .get("access_token")
        .and_then(Value::as_str)
        .context("Codex token refresh response missing access_token")?;
    let refresh_token = raw
        .get("refresh_token")
        .and_then(Value::as_str)
        .context("Codex token refresh response missing refresh_token")?;
    let id_token = raw
        .get("id_token")
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(RefreshedCodexTokens {
        id_token,
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
    })
}

fn should_refresh_codex_auth(tokens: &CodexAuthTokens) -> bool {
    if parse_jwt_expiration(&tokens.access_token).is_some_and(|expires_at| expires_at <= Utc::now()) {
        return true;
    }
    tokens.last_refresh.is_some_and(|refreshed_at| {
        refreshed_at < Utc::now() - Duration::days(CODEX_REFRESH_INTERVAL_DAYS)
    })
}

fn parse_jwt_expiration(token: &str) -> Option<DateTime<Utc>> {
    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let raw: Value = serde_json::from_slice(&decoded).ok()?;
    let exp = raw.get("exp")?.as_i64()?;
    DateTime::<Utc>::from_timestamp(exp, 0)
}

fn is_codex_usage_refreshable_error(error: &anyhow::Error) -> bool {
    format!("{error:#}")
        .to_ascii_lowercase()
        .contains("401 unauthorized")
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("write {}", path.display()))
}

fn default_now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time drift")
        .as_secs() as i64
}

pub fn pick_five_hour_window(usage: &UsageResponse) -> Option<&UsageWindow> {
    let rate_limit = usage.rate_limit.as_ref()?;
    if rate_limit
        .primary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 18_000)
    {
        return rate_limit.primary_window.as_ref();
    }
    if rate_limit
        .secondary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 18_000)
    {
        return rate_limit.secondary_window.as_ref();
    }
    None
}

pub fn pick_weekly_window(usage: &UsageResponse) -> Option<&UsageWindow> {
    let rate_limit = usage.rate_limit.as_ref()?;
    if rate_limit
        .secondary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 604_800)
    {
        return rate_limit.secondary_window.as_ref();
    }
    if rate_limit
        .primary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds == 604_800)
    {
        return rate_limit.primary_window.as_ref();
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Mutex};

    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "agent-switch-usage-tests-{}-{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_file(&base);
        base
    }

    fn sample_usage(plan: &str) -> UsageResponse {
        UsageResponse {
            email: Some("alpha@example.com".to_string()),
            plan_type: Some(plan.to_string()),
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 12.0,
                    limit_window_seconds: 18_000,
                    reset_after_seconds: 3_600,
                    reset_at: 18_000,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 41.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 300_000,
                    reset_at: 604_800,
                }),
            }),
        }
    }

    fn write_codex_auth(
        path: &Path,
        access_token: &str,
        refresh_token: &str,
        last_refresh: chrono::DateTime<chrono::Utc>,
    ) {
        let snapshot = serde_json::json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {
                "id_token": "id-old",
                "access_token": access_token,
                "refresh_token": refresh_token,
                "account_id": "acct-alpha"
            },
            "last_refresh": last_refresh.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        });
        write_json(path, &snapshot).unwrap();
    }

    #[test]
    fn usage_history_records_observations_in_active_windows() {
        let cache_path = temp_file("cache.json");
        let history_path = temp_file("history.json");
        let service = UsageService::new(cache_path, history_path.clone(), 300).with_now_seconds(500);
        let usage = UsageResponse {
            email: Some("alpha@example.com".to_string()),
            plan_type: Some("plus".to_string()),
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 12.0,
                    limit_window_seconds: 18_000,
                    reset_after_seconds: 3_600,
                    reset_at: 18_000,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 41.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 300_000,
                    reset_at: 604_800,
                }),
            }),
        };

        service
            .record_usage_snapshot(Some("acct-alpha"), Some(&usage))
            .unwrap();

        let history = service.profile_history(Some("acct-alpha")).unwrap();
        assert_eq!(history.weekly_windows.len(), 1);
        assert_eq!(history.five_hour_windows.len(), 1);
        assert_eq!(history.weekly_windows[0].observations.len(), 1);
        assert_eq!(history.weekly_windows[0].observations[0].observed_at, 500);
        assert_eq!(history.weekly_windows[0].observations[0].used_percent, 41.0);
        assert_eq!(history.five_hour_windows[0].start_at, 0);
    }

    #[test]
    fn force_refresh_without_cache_surfaces_fetch_error() {
        let cache_path = temp_file("force-refresh-cache.json");
        let history_path = temp_file("force-refresh-history.json");
        let service = UsageService::new(cache_path, history_path, 300)
            .with_fetcher(|_, _| Err(anyhow::anyhow!("fetch failed")));

        let result = service.read_usage(Some("acct-alpha"), Some("token"), true, false);

        assert!(result.is_err(), "force refresh should surface fetch failures");
    }

    #[test]
    fn usage_history_separates_reset_windows() {
        let cache_path = temp_file("cache-b.json");
        let history_path = temp_file("history-b.json");
        let service = UsageService::new(cache_path, history_path, 300).with_now_seconds(750);

        let usage_before_reset = UsageResponse {
            email: None,
            plan_type: None,
            rate_limit: Some(UsageRateLimit {
                primary_window: None,
                secondary_window: Some(UsageWindow {
                    used_percent: 10.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 10,
                    reset_at: 604_800,
                }),
            }),
        };
        let usage_after_reset = UsageResponse {
            email: None,
            plan_type: None,
            rate_limit: Some(UsageRateLimit {
                primary_window: None,
                secondary_window: Some(UsageWindow {
                    used_percent: 4.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 500_000,
                    reset_at: 1_209_600,
                }),
            }),
        };

        service
            .record_usage_snapshot(Some("acct-alpha"), Some(&usage_before_reset))
            .unwrap();
        service
            .record_usage_snapshot(Some("acct-alpha"), Some(&usage_after_reset))
            .unwrap();

        let history = service.profile_history(Some("acct-alpha")).unwrap();
        assert_eq!(history.weekly_windows.len(), 2);
        assert_eq!(history.weekly_windows[0].start_at, 0);
        assert_eq!(history.weekly_windows[1].start_at, 604_800);
    }

    #[test]
    fn trim_history_windows_keeps_full_week_of_observations() {
        let mut windows = vec![UsageWindowHistory {
            limit_window_seconds: 604_800,
            start_at: 0,
            end_at: 604_800,
            observations: (0..300)
                .map(|idx| UsageObservation {
                    observed_at: idx * 2_000,
                    used_percent: idx as f64 % 100.0,
                })
                .collect(),
        }];

        trim_history_windows(&mut windows, 3);

        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].observations.len(), 300);
    }

    #[test]
    fn trim_history_windows_removes_observations_outside_window_range() {
        let mut windows = vec![UsageWindowHistory {
            limit_window_seconds: 18_000,
            start_at: 100,
            end_at: 200,
            observations: vec![
                UsageObservation {
                    observed_at: 90,
                    used_percent: 1.0,
                },
                UsageObservation {
                    observed_at: 150,
                    used_percent: 2.0,
                },
                UsageObservation {
                    observed_at: 210,
                    used_percent: 3.0,
                },
            ],
        }];

        trim_history_windows(&mut windows, 34);

        assert_eq!(windows[0].observations.len(), 1);
        assert_eq!(windows[0].observations[0].observed_at, 150);
    }

    #[test]
    fn merge_profile_history_aliases_moves_alias_history_into_canonical_key() {
        let cache_path = temp_file("merge-history-cache.json");
        let history_path = temp_file("merge-history.json");
        let service = UsageService::new(cache_path, history_path, 300);
        let mut history = UsageHistoryCache::default();
        history.by_account_id.insert(
            "canonical".to_string(),
            ProfileUsageHistory {
                weekly_windows: vec![UsageWindowHistory {
                    limit_window_seconds: 604_800,
                    start_at: 0,
                    end_at: 604_800,
                    observations: vec![UsageObservation {
                        observed_at: 100,
                        used_percent: 10.0,
                    }],
                }],
                five_hour_windows: vec![],
            },
        );
        history.by_account_id.insert(
            "alias".to_string(),
            ProfileUsageHistory {
                weekly_windows: vec![UsageWindowHistory {
                    limit_window_seconds: 604_800,
                    start_at: 0,
                    end_at: 604_800,
                    observations: vec![
                        UsageObservation {
                            observed_at: 100,
                            used_percent: 10.0,
                        },
                        UsageObservation {
                            observed_at: 200,
                            used_percent: 20.0,
                        },
                    ],
                }],
                five_hour_windows: vec![UsageWindowHistory {
                    limit_window_seconds: 18_000,
                    start_at: 10,
                    end_at: 20,
                    observations: vec![UsageObservation {
                        observed_at: 15,
                        used_percent: 30.0,
                    }],
                }],
            },
        );
        service.write_history_cache(&history).unwrap();

        service
            .merge_profile_history_aliases(Some("canonical"), ["alias"])
            .unwrap();

        let canonical = service.profile_history(Some("canonical")).unwrap();
        let alias = service.profile_history(Some("alias")).unwrap();
        assert_eq!(canonical.weekly_windows.len(), 1);
        assert_eq!(canonical.weekly_windows[0].observations.len(), 2);
        assert_eq!(canonical.five_hour_windows.len(), 1);
        assert!(alias.weekly_windows.is_empty());
        assert!(alias.five_hour_windows.is_empty());
    }

    #[test]
    fn stale_codex_auth_refreshes_before_usage_request() {
        let auth_path = temp_file("codex-stale-auth.json");
        write_codex_auth(
            &auth_path,
            "access-old",
            "refresh-old",
            chrono::Utc::now() - chrono::Duration::days(9),
        );
        let seen_tokens = Arc::new(Mutex::new(Vec::<String>::new()));
        let usage_tokens = Arc::clone(&seen_tokens);

        let initial_tokens = read_codex_auth_tokens(&auth_path).unwrap().unwrap();
        let usage = fetch_codex_usage_with_auto_refresh(
            &auth_path,
            &initial_tokens,
            move |account_id, access_token| {
                assert_eq!(account_id, "acct-alpha");
                usage_tokens.lock().unwrap().push(access_token.to_string());
                Ok(sample_usage("team"))
            },
            |refresh_token| {
                assert_eq!(refresh_token, "refresh-old");
                Ok(RefreshedCodexTokens {
                    id_token: Some("id-new".to_string()),
                    access_token: "access-new".to_string(),
                    refresh_token: "refresh-new".to_string(),
                })
            },
        )
        .unwrap();

        assert_eq!(usage.plan_type.as_deref(), Some("team"));
        assert_eq!(&*seen_tokens.lock().unwrap(), &["access-new".to_string()]);

        let refreshed = read_codex_auth_tokens(&auth_path).unwrap().unwrap();
        assert_eq!(refreshed.access_token, "access-new");
        assert_eq!(refreshed.refresh_token.as_deref(), Some("refresh-new"));
    }

    #[test]
    fn codex_usage_401_refreshes_and_retries_once() {
        let auth_path = temp_file("codex-401-auth.json");
        write_codex_auth(
            &auth_path,
            "access-old",
            "refresh-old",
            chrono::Utc::now(),
        );
        let seen_tokens = Arc::new(Mutex::new(Vec::<String>::new()));
        let usage_tokens = Arc::clone(&seen_tokens);

        let initial_tokens = read_codex_auth_tokens(&auth_path).unwrap().unwrap();
        let usage = fetch_codex_usage_with_auto_refresh(
            &auth_path,
            &initial_tokens,
            move |account_id, access_token| {
                assert_eq!(account_id, "acct-alpha");
                usage_tokens.lock().unwrap().push(access_token.to_string());
                if access_token == "access-old" {
                    Err(anyhow::anyhow!(
                        "usage request failed: HTTP status client error (401 Unauthorized)"
                    ))
                } else {
                    Ok(sample_usage("team"))
                }
            },
            |refresh_token| {
                assert_eq!(refresh_token, "refresh-old");
                Ok(RefreshedCodexTokens {
                    id_token: None,
                    access_token: "access-new".to_string(),
                    refresh_token: "refresh-new".to_string(),
                })
            },
        )
        .unwrap();

        assert_eq!(usage.plan_type.as_deref(), Some("team"));
        assert_eq!(
            &*seen_tokens.lock().unwrap(),
            &["access-old".to_string(), "access-new".to_string()]
        );

        let refreshed = read_codex_auth_tokens(&auth_path).unwrap().unwrap();
        assert_eq!(refreshed.access_token, "access-new");
        assert_eq!(refreshed.refresh_token.as_deref(), Some("refresh-new"));
    }
}
