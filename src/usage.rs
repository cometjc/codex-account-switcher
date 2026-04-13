use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use fs2::FileExt;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::db::{
    FiveHourCycleSummaryRow, ProfileWindowRow, SqliteStore, UsageCacheRow, UsageObservationRow,
};
use crate::paths::agent_switch_config_dir;

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
pub struct ProfileUsageObservation {
    pub observed_at_local: i64,
    pub weekly_used_percent: Option<f64>,
    pub five_hour_used_percent: Option<f64>,
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
    pub observations: Vec<ProfileUsageObservation>,
    #[serde(default)]
    pub weekly_reset_at: Option<i64>,
    #[serde(default)]
    pub five_hour_reset_at: Option<i64>,
    #[serde(default = "default_weekly_window_seconds")]
    pub weekly_window_seconds: i64,
    #[serde(default = "default_five_hour_window_seconds")]
    pub five_hour_window_seconds: i64,
    // Legacy fields kept for backward compatibility with old cache files.
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

fn default_weekly_window_seconds() -> i64 {
    604_800
}

fn default_five_hour_window_seconds() -> i64 {
    18_000
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
    db: SqliteStore,
    service_namespace: String,
    ttl_seconds: i64,
    clock: Arc<Clock>,
    fetcher: Arc<Fetcher>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FiveHourCycleSummary {
    pub cycle_start_at: i64,
    pub cycle_end_at: i64,
    pub first_observed_at: i64,
    pub last_observed_at: i64,
    pub start_weekly_used_percent: Option<f64>,
    pub end_weekly_used_percent: Option<f64>,
    pub start_five_hour_used_percent: Option<f64>,
    pub end_five_hour_used_percent: f64,
    pub active_seconds: i64,
    pub idle_seconds: i64,
    pub suspected_cap_stall: bool,
}

impl UsageService {
    pub fn new(cache_path: PathBuf, history_path: PathBuf, ttl_seconds: i64) -> Self {
        let db_path = resolve_usage_db_path(&history_path);
        Self {
            service_namespace: service_namespace_from_cache_path(&cache_path),
            cache_path,
            history_path,
            db: SqliteStore::new(db_path),
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
                if age <= self.ttl_seconds {
                    return Ok(UsageReadResult {
                        usage: Some(record.usage.clone()),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: false,
                    });
                }
                if cache_only {
                    return Ok(UsageReadResult {
                        usage: Some(record.usage.clone()),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: true,
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
                if age <= self.ttl_seconds {
                    return Ok(UsageReadResult {
                        usage: Some(record.usage.clone()),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: false,
                    });
                }
                if cache_only {
                    return Ok(UsageReadResult {
                        usage: Some(record.usage.clone()),
                        source: UsageSource::Cache,
                        fetched_at: Some(record.fetched_at),
                        stale: true,
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

    /// Appends one observation per rate-limit window. `observed_at` matches the usage snapshot:
    /// - [`UsageSource::Api`]: wall clock when the fetch completed (same moment as cache `fetched_at`).
    /// - [`UsageSource::Cache`]: [`UsageReadResult::fetched_at`] so observations stay time-aligned with
    ///   embedded `reset_at` values. Using wall clock for cache would append "now" with stale quotas.
    pub fn record_usage_snapshot(&self, account_id: Option<&str>, read: &UsageReadResult) -> Result<()> {
        let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(());
        };
        let Some(usage) = read.usage.as_ref() else {
            return Ok(());
        };
        if usage.rate_limit.is_none() {
            return Ok(());
        }

        let observed_at = match read.source {
            UsageSource::None => return Ok(()),
            UsageSource::Api => (self.clock)(),
            UsageSource::Cache => read.fetched_at.unwrap_or_else(|| (self.clock)()),
        };
        let weekly_window = pick_weekly_window(usage);
        let five_hour_window = pick_five_hour_window(usage);
        self.mutate_history_cache(|history| {
            let entry = history.by_account_id.entry(account_id.to_string()).or_default();
            let before = entry.clone();
            let previous_observations = entry.observations.clone();
            if let Some(window) = weekly_window {
                entry.weekly_reset_at = Some(window.reset_at);
                entry.weekly_window_seconds = window.limit_window_seconds;
            }
            if let Some(window) = five_hour_window {
                entry.five_hour_reset_at = Some(window.reset_at);
                entry.five_hour_window_seconds = window.limit_window_seconds;
            }

            upsert_profile_observation(
                &mut entry.observations,
                observed_at,
                weekly_window.map(|w| w.used_percent),
                five_hour_window.map(|w| w.used_percent),
            );
            entry.observations.sort_by_key(|obs| obs.observed_at_local);
            prune_profile_observations_to_active_windows(entry, (self.clock)());
            if entry.observations.is_empty() && !previous_observations.is_empty() {
                // Guard against destructive refresh writes (e.g. reset/window boundary skew).
                entry.observations = previous_observations;
            }
            Ok(*entry != before)
        })
    }

    pub fn profile_history(&self, account_id: Option<&str>) -> Result<ProfileUsageHistory> {
        let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(ProfileUsageHistory::default());
        };
        // Keep on-disk schema normalized before serving history to callers.
        self.mutate_history_cache(|_| Ok(false))?;
        let history = self.read_history_cache()?;
        let entry = history
            .by_account_id
            .get(account_id)
            .cloned()
            .map(|entry| normalize_profile_history(entry).0)
            .unwrap_or_default();
        self.backfill_cycle_summaries_from_history(account_id, &entry)?;
        Ok(entry)
    }

    pub fn five_hour_cycle_summaries(
        &self,
        account_id: Option<&str>,
    ) -> Result<Vec<FiveHourCycleSummary>> {
        let Some(account_id) = account_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(Vec::new());
        };
        self.db
            .read_five_hour_cycle_summaries(&self.service_namespace, account_id)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| FiveHourCycleSummary {
                        cycle_start_at: row.cycle_start_at,
                        cycle_end_at: row.cycle_end_at,
                        first_observed_at: row.first_observed_at,
                        last_observed_at: row.last_observed_at,
                        start_weekly_used_percent: row.start_weekly_used_percent,
                        end_weekly_used_percent: row.end_weekly_used_percent,
                        start_five_hour_used_percent: row.start_five_hour_used_percent,
                        end_five_hour_used_percent: row.end_five_hour_used_percent,
                        active_seconds: row.active_seconds,
                        idle_seconds: row.idle_seconds,
                        suspected_cap_stall: row.suspected_cap_stall,
                    })
                    .collect()
            })
    }

    pub fn history_account_ids(&self) -> Result<Vec<String>> {
        self.mutate_history_cache(|_| Ok(false))?;
        let history = self.read_history_cache()?;
        Ok(history.by_account_id.keys().cloned().collect())
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
        self.mutate_history_cache(|history| {
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
            Ok(changed)
        })
    }

    fn read_cache(&self) -> Result<UsageCache> {
        if should_backfill_from_legacy(&self.cache_path, self.db.path()) {
            self.backfill_cache_from_legacy_json()?;
        }
        self.read_cache_from_db()
    }

    fn write_cache(&self, cache: &UsageCache) -> Result<()> {
        let rows = cache
            .by_account_id
            .iter()
            .map(|(account_key, record)| UsageCacheRow {
                account_key: account_key.clone(),
                fetched_at: record.fetched_at,
                payload_json: serde_json::to_string(&record.usage).unwrap_or_else(|_| "{}".to_string()),
            })
            .collect::<Vec<_>>();
        self.db.write_usage_cache_rows(&self.service_namespace, &rows)
    }

    fn read_history_cache(&self) -> Result<UsageHistoryCache> {
        if should_backfill_from_legacy(&self.history_path, self.db.path()) {
            self.backfill_history_from_legacy_json()?;
        }
        self.read_history_from_db()
    }

    fn write_history_cache(&self, history: &UsageHistoryCache) -> Result<()> {
        let mut windows = Vec::new();
        let mut observations = Vec::new();
        let mut normalized_by_account = BTreeMap::new();
        for (account_key, entry) in &history.by_account_id {
            let (normalized, _) = normalize_profile_history(entry.clone());
            normalized_by_account.insert(account_key.clone(), normalized.clone());
            windows.push(ProfileWindowRow {
                account_key: account_key.clone(),
                weekly_reset_at: normalized.weekly_reset_at,
                weekly_window_seconds: normalized.weekly_window_seconds,
                five_hour_reset_at: normalized.five_hour_reset_at,
                five_hour_window_seconds: normalized.five_hour_window_seconds,
            });
            observations.extend(normalized.observations.iter().map(|obs| UsageObservationRow {
                account_key: account_key.clone(),
                observed_at: obs.observed_at_local,
                weekly_used_percent: obs.weekly_used_percent,
                five_hour_used_percent: obs.five_hour_used_percent,
            }));
        }
        self.db
            .upsert_usage_history(&self.service_namespace, &windows, &observations)?;
        if let Some(parent) = self.history_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let legacy = UsageHistoryCache {
            by_account_id: normalized_by_account,
        };
        fs::write(
            &self.history_path,
            format!("{}\n", serde_json::to_string_pretty(&legacy)?),
        )
        .with_context(|| format!("write {}", self.history_path.display()))
    }

    fn mutate_history_cache<F>(&self, mutator: F) -> Result<()>
    where
        F: FnOnce(&mut UsageHistoryCache) -> Result<bool>,
    {
        let _lock = self.lock_history_file()?;
        let mut history = self.read_history_cache()?;
        let mut changed = false;
        for value in history.by_account_id.values_mut() {
            let (normalized, entry_changed) = normalize_profile_history(std::mem::take(value));
            *value = normalized;
            changed |= entry_changed;
        }
        changed |= mutator(&mut history)?;
        if changed {
            self.write_history_cache(&history)?;
        }
        Ok(())
    }

    fn lock_history_file(&self) -> Result<std::fs::File> {
        let lock_path = self.history_path.with_extension(format!(
            "{}.lock",
            self.history_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("json")
        ));
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("open {}", lock_path.display()))?;
        file.lock_exclusive()
            .with_context(|| format!("lock {}", lock_path.display()))?;
        Ok(file)
    }

    fn read_cache_from_db(&self) -> Result<UsageCache> {
        let rows = self.db.read_usage_cache(&self.service_namespace)?;
        let mut by_account_id = BTreeMap::new();
        for row in rows {
            let usage = serde_json::from_str::<UsageResponse>(&row.payload_json).unwrap_or(UsageResponse {
                email: None,
                plan_type: None,
                rate_limit: None,
            });
            by_account_id.insert(
                row.account_key,
                UsageCacheRecord {
                    fetched_at: row.fetched_at,
                    usage,
                },
            );
        }
        Ok(UsageCache { by_account_id })
    }

    fn read_history_from_db(&self) -> Result<UsageHistoryCache> {
        let (observations, windows) = self.db.read_usage_history(&self.service_namespace)?;
        let mut by_account_id = BTreeMap::<String, ProfileUsageHistory>::new();
        for window in windows {
            let entry = by_account_id
                .entry(window.account_key)
                .or_insert_with(default_profile_usage_history_entry);
            entry.weekly_reset_at = window.weekly_reset_at;
            entry.weekly_window_seconds = window.weekly_window_seconds;
            entry.five_hour_reset_at = window.five_hour_reset_at;
            entry.five_hour_window_seconds = window.five_hour_window_seconds;
        }
        for obs in observations {
            let entry = by_account_id
                .entry(obs.account_key)
                .or_insert_with(default_profile_usage_history_entry);
            entry.observations.push(ProfileUsageObservation {
                observed_at_local: obs.observed_at,
                weekly_used_percent: obs.weekly_used_percent,
                five_hour_used_percent: obs.five_hour_used_percent,
            });
        }
        for entry in by_account_id.values_mut() {
            entry.observations.sort_by_key(|o| o.observed_at_local);
        }
        Ok(UsageHistoryCache { by_account_id })
    }

    fn backfill_cache_from_legacy_json(&self) -> Result<()> {
        let cache = match fs::read_to_string(&self.cache_path) {
            Ok(raw) => serde_json::from_str::<UsageCache>(&raw).unwrap_or_default(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => UsageCache::default(),
            Err(error) => return Err(error).with_context(|| format!("read {}", self.cache_path.display())),
        };
        self.write_cache(&cache)?;
        Ok(())
    }

    fn backfill_history_from_legacy_json(&self) -> Result<()> {
        let mut history = match fs::read_to_string(&self.history_path) {
            Ok(raw) => serde_json::from_str::<UsageHistoryCache>(&raw).unwrap_or_default(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => UsageHistoryCache::default(),
            Err(error) => return Err(error).with_context(|| format!("read {}", self.history_path.display())),
        };
        for value in history.by_account_id.values_mut() {
            let (normalized, _) = normalize_profile_history(std::mem::take(value));
            *value = normalized;
        }
        self.write_history_cache(&history)?;
        fs::write(
            &self.history_path,
            format!("{}\n", serde_json::to_string_pretty(&history)?),
        )
        .with_context(|| format!("write {}", self.history_path.display()))?;
        Ok(())
    }

    fn backfill_cycle_summaries_from_history(
        &self,
        account_id: &str,
        history: &ProfileUsageHistory,
    ) -> Result<()> {
        let summaries = derive_five_hour_cycle_summaries(history);
        let rows = summaries
            .into_iter()
            .map(|summary| FiveHourCycleSummaryRow {
                account_key: account_id.to_string(),
                cycle_start_at: summary.cycle_start_at,
                cycle_end_at: summary.cycle_end_at,
                first_observed_at: summary.first_observed_at,
                last_observed_at: summary.last_observed_at,
                start_weekly_used_percent: summary.start_weekly_used_percent,
                end_weekly_used_percent: summary.end_weekly_used_percent,
                start_five_hour_used_percent: summary.start_five_hour_used_percent,
                end_five_hour_used_percent: summary.end_five_hour_used_percent,
                active_seconds: summary.active_seconds,
                idle_seconds: summary.idle_seconds,
                suspected_cap_stall: summary.suspected_cap_stall,
            })
            .collect::<Vec<_>>();
        self.db
            .replace_five_hour_cycle_summaries(&self.service_namespace, account_id, &rows)
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

const WEEKLY_WINDOW_SECONDS: i64 = 604_800;
const WEEKLY_JITTER_TOLERANCE_SECONDS: i64 = 120;

fn is_weekly_window(window: &UsageWindowHistory) -> bool {
    window.limit_window_seconds == WEEKLY_WINDOW_SECONDS
}

fn windows_match_with_weekly_jitter(
    existing: &UsageWindowHistory,
    incoming: &UsageWindowHistory,
) -> bool {
    if existing.limit_window_seconds != incoming.limit_window_seconds {
        return false;
    }
    if existing.start_at == incoming.start_at && existing.end_at == incoming.end_at {
        return true;
    }
    if !is_weekly_window(existing) {
        return false;
    }
    (existing.end_at - incoming.end_at).abs() <= WEEKLY_JITTER_TOLERANCE_SECONDS
}

fn canonicalize_weekly_window(existing: &mut UsageWindowHistory, incoming: &UsageWindowHistory) {
    if !is_weekly_window(existing) {
        return;
    }
    let canonical_end = existing.end_at.max(incoming.end_at);
    existing.end_at = canonical_end;
    existing.start_at = canonical_end - existing.limit_window_seconds;
}

fn merge_profile_usage_history(target: &mut ProfileUsageHistory, source: ProfileUsageHistory) {
    target.observations.extend(source.observations);
    target.observations.sort_by_key(|obs| obs.observed_at_local);
    target.observations.dedup_by(|right, left| {
        right.observed_at_local == left.observed_at_local
            && right.weekly_used_percent == left.weekly_used_percent
            && right.five_hour_used_percent == left.five_hour_used_percent
    });
    target.weekly_reset_at = match (target.weekly_reset_at, source.weekly_reset_at) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    };
    target.five_hour_reset_at = match (target.five_hour_reset_at, source.five_hour_reset_at) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    };
    if target.weekly_window_seconds <= 0 {
        target.weekly_window_seconds = default_weekly_window_seconds();
    }
    if target.five_hour_window_seconds <= 0 {
        target.five_hour_window_seconds = default_five_hour_window_seconds();
    }

    // Merge legacy windows for backward compatibility migration.
    merge_usage_window_histories(&mut target.weekly_windows, source.weekly_windows, 3);
    merge_usage_window_histories(&mut target.five_hour_windows, source.five_hour_windows, 34);
}

fn upsert_profile_observation(
    observations: &mut Vec<ProfileUsageObservation>,
    observed_at_local: i64,
    weekly_used_percent: Option<f64>,
    five_hour_used_percent: Option<f64>,
) {
    if let Some(existing) = observations
        .iter_mut()
        .find(|obs| obs.observed_at_local == observed_at_local)
    {
        if let Some(value) = weekly_used_percent {
            existing.weekly_used_percent = Some(value.clamp(0.0, 100.0));
        }
        if let Some(value) = five_hour_used_percent {
            existing.five_hour_used_percent = Some(value.clamp(0.0, 100.0));
        }
        return;
    }
    observations.push(ProfileUsageObservation {
        observed_at_local,
        weekly_used_percent: weekly_used_percent.map(|v| v.clamp(0.0, 100.0)),
        five_hour_used_percent: five_hour_used_percent.map(|v| v.clamp(0.0, 100.0)),
    });
}

fn prune_profile_observations_to_active_windows(history: &mut ProfileUsageHistory, now_seconds: i64) {
    let anchor_seconds = history
        .observations
        .iter()
        .map(|obs| obs.observed_at_local)
        .max()
        .unwrap_or(now_seconds)
        .max(now_seconds);
    let weekly_start = anchor_seconds - history.weekly_window_seconds;
    let five_start = anchor_seconds - history.five_hour_window_seconds;

    history.observations.retain(|obs| {
        let in_weekly = match obs.weekly_used_percent {
            Some(_) => obs.observed_at_local > weekly_start && obs.observed_at_local <= anchor_seconds,
            None => false,
        };
        let in_five_hour = match obs.five_hour_used_percent {
            Some(_) => obs.observed_at_local > five_start && obs.observed_at_local <= anchor_seconds,
            None => false,
        };
        in_weekly || in_five_hour
    });
}

fn normalize_profile_history(mut history: ProfileUsageHistory) -> (ProfileUsageHistory, bool) {
    let before = history.clone();
    if history.weekly_window_seconds <= 0 {
        history.weekly_window_seconds = default_weekly_window_seconds();
    }
    if history.five_hour_window_seconds <= 0 {
        history.five_hour_window_seconds = default_five_hour_window_seconds();
    }

    // Legacy migration: fold per-window observations into profile-level observations.
    for window in &history.weekly_windows {
        if window.limit_window_seconds == history.weekly_window_seconds {
            history.weekly_reset_at = Some(
                history
                    .weekly_reset_at
                    .unwrap_or(window.end_at)
                    .max(window.end_at),
            );
            for obs in &window.observations {
                upsert_profile_observation(
                    &mut history.observations,
                    obs.observed_at,
                    Some(obs.used_percent),
                    None,
                );
            }
        }
    }
    for window in &history.five_hour_windows {
        if window.limit_window_seconds == history.five_hour_window_seconds {
            history.five_hour_reset_at = Some(
                history
                    .five_hour_reset_at
                    .unwrap_or(window.end_at)
                    .max(window.end_at),
            );
            for obs in &window.observations {
                upsert_profile_observation(
                    &mut history.observations,
                    obs.observed_at,
                    None,
                    Some(obs.used_percent),
                );
            }
        }
    }

    history.observations.sort_by_key(|obs| obs.observed_at_local);
    history.observations.dedup_by(|right, left| {
        right.observed_at_local == left.observed_at_local
            && right.weekly_used_percent == left.weekly_used_percent
            && right.five_hour_used_percent == left.five_hour_used_percent
    });
    // Old fields are no longer authoritative after migration.
    history.weekly_windows.clear();
    history.five_hour_windows.clear();
    let changed = history != before;
    (history, changed)
}

fn merge_usage_window_histories(
    target: &mut Vec<UsageWindowHistory>,
    source: Vec<UsageWindowHistory>,
    max_count: usize,
) {
    for mut source_window in source {
        if let Some(existing) = target
            .iter_mut()
            .find(|candidate| windows_match_with_weekly_jitter(candidate, &source_window))
        {
            existing.observations.append(&mut source_window.observations);
            canonicalize_weekly_window(existing, &source_window);
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
    let serialized = format!("{}\n", serde_json::to_string_pretty(value)?);
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|ext| ext.to_str()).unwrap_or("json")
    ));
    fs::write(&tmp_path, serialized).with_context(|| format!("write {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), path.display()))
}

fn default_now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time drift")
        .as_secs() as i64
}

fn service_namespace_from_cache_path(cache_path: &Path) -> String {
    cache_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "usage".to_string())
}

fn resolve_usage_db_path(history_path: &Path) -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);
    if let Some(home) = home {
        let codex_dir = home.join(".codex");
        let config_dir = agent_switch_config_dir();
        if history_path.starts_with(&codex_dir) || history_path.starts_with(&config_dir) {
            return config_dir.join("agent-switch.db");
        }
    }
    history_path.with_extension("sqlite.db")
}

fn default_profile_usage_history_entry() -> ProfileUsageHistory {
    ProfileUsageHistory {
        observations: Vec::new(),
        weekly_reset_at: None,
        five_hour_reset_at: None,
        weekly_window_seconds: default_weekly_window_seconds(),
        five_hour_window_seconds: default_five_hour_window_seconds(),
        weekly_windows: Vec::new(),
        five_hour_windows: Vec::new(),
    }
}

fn should_backfill_from_legacy(legacy_path: &Path, db_path: &Path) -> bool {
    let Ok(legacy_meta) = fs::metadata(legacy_path) else {
        return false;
    };
    let Ok(legacy_modified) = legacy_meta.modified() else {
        return false;
    };
    let Ok(db_meta) = fs::metadata(db_path) else {
        return true;
    };
    let Ok(db_modified) = db_meta.modified() else {
        return true;
    };
    legacy_modified > db_modified
}

fn derive_five_hour_cycle_summaries(history: &ProfileUsageHistory) -> Vec<FiveHourCycleSummary> {
    let Some(reset_at) = history.five_hour_reset_at else {
        return Vec::new();
    };
    let window_seconds = history.five_hour_window_seconds;
    if window_seconds <= 0 {
        return Vec::new();
    }

    let mut grouped = BTreeMap::<i64, Vec<&ProfileUsageObservation>>::new();
    for obs in history
        .observations
        .iter()
        .filter(|obs| obs.five_hour_used_percent.is_some())
    {
        if obs.observed_at_local > reset_at {
            continue;
        }
        let delta = reset_at - obs.observed_at_local;
        let cycle_index = delta.div_euclid(window_seconds);
        let cycle_end_at = reset_at - cycle_index * window_seconds;
        grouped.entry(cycle_end_at).or_default().push(obs);
    }

    grouped
        .into_iter()
        .filter_map(|(cycle_end_at, mut observations)| {
            observations.sort_by_key(|obs| obs.observed_at_local);
            let first = *observations.first()?;
            let last = *observations.last()?;
            let cycle_start_at = cycle_end_at - window_seconds;
            let active_seconds = (last.observed_at_local - first.observed_at_local).max(0);
            let idle_seconds = (window_seconds - active_seconds).max(0);
            let end_five_hour_used_percent = last.five_hour_used_percent?;
            Some(FiveHourCycleSummary {
                cycle_start_at,
                cycle_end_at,
                first_observed_at: first.observed_at_local,
                last_observed_at: last.observed_at_local,
                start_weekly_used_percent: first.weekly_used_percent,
                end_weekly_used_percent: last.weekly_used_percent,
                start_five_hour_used_percent: first.five_hour_used_percent,
                end_five_hour_used_percent,
                active_seconds,
                idle_seconds,
                suspected_cap_stall: end_five_hour_used_percent >= 95.0,
            })
        })
        .collect()
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
    if rate_limit
        .secondary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds != 18_000)
    {
        return rate_limit.secondary_window.as_ref();
    }
    if rate_limit
        .primary_window
        .as_ref()
        .is_some_and(|window| window.limit_window_seconds != 18_000)
    {
        return rate_limit.primary_window.as_ref();
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Mutex};

    use rusqlite::{OptionalExtension, params};

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

    fn sqlite_table_exists(service: &UsageService, table: &str) -> bool {
        service
            .db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map(|value| value.is_some())
                .context("query sqlite_master")
            })
            .unwrap()
    }

    fn sqlite_row_count(service: &UsageService, table: &str, account_key: &str) -> i64 {
        let sql = match table {
            "usage_cache" => {
                "SELECT COUNT(*)
                 FROM usage_cache c
                 JOIN profiles p ON p.id = c.profile_id
                 WHERE p.service = ?1 AND p.account_key = ?2"
            }
            "usage_observations" => {
                "SELECT COUNT(*)
                 FROM usage_observations o
                 JOIN profiles p ON p.id = o.profile_id
                 WHERE p.service = ?1 AND p.account_key = ?2"
            }
            "five_hour_cycle_summaries" => {
                "SELECT COUNT(*)
                 FROM five_hour_cycle_summaries s
                 JOIN profiles p ON p.id = s.profile_id
                 WHERE p.service = ?1 AND p.account_key = ?2"
            }
            other => panic!("unexpected table: {other}"),
        };
        service
            .db
            .with_conn(|conn| {
                conn.query_row(sql, params![service.service_namespace, account_key], |row| {
                    row.get::<_, i64>(0)
                })
                .context("count table rows")
            })
            .unwrap()
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

        let read = UsageReadResult {
            usage: Some(usage),
            source: UsageSource::Api,
            fetched_at: Some(500),
            stale: false,
        };
        service
            .record_usage_snapshot(Some("acct-alpha"), &read)
            .unwrap();

        let history = service.profile_history(Some("acct-alpha")).unwrap();
        assert_eq!(history.weekly_reset_at, Some(604_800));
        assert_eq!(history.five_hour_reset_at, Some(18_000));
        assert_eq!(history.observations.len(), 1);
        assert_eq!(history.observations[0].observed_at_local, 500);
        assert_eq!(history.observations[0].weekly_used_percent, Some(41.0));
        assert_eq!(history.observations[0].five_hour_used_percent, Some(12.0));
    }

    #[test]
    fn cache_snapshot_uses_fetched_at_not_wall_clock_for_observed_at() {
        let cache_path = temp_file("cache-obs-time.json");
        let history_path = temp_file("history-obs-time.json");
        let service = UsageService::new(cache_path, history_path.clone(), 300).with_now_seconds(50_000);
        let usage = sample_usage("plus");
        let read = UsageReadResult {
            usage: Some(usage),
            source: UsageSource::Cache,
            fetched_at: Some(5_000),
            stale: true,
        };
        service.record_usage_snapshot(Some("acct-alpha"), &read).unwrap();
        let history = service.profile_history(Some("acct-alpha")).unwrap();
        assert_eq!(
            history.observations[0].observed_at_local,
            5_000,
            "cached usage must stamp observations at fetched_at, not current clock"
        );
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
        let service_before = UsageService::new(cache_path.clone(), history_path.clone(), 300)
            .with_now_seconds(604_790);
        let service_after =
            UsageService::new(cache_path, history_path, 300).with_now_seconds(1_209_590);

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

        let read_before = UsageReadResult {
            usage: Some(usage_before_reset),
            source: UsageSource::Api,
            fetched_at: Some(750),
            stale: false,
        };
        let read_after = UsageReadResult {
            usage: Some(usage_after_reset),
            source: UsageSource::Api,
            fetched_at: Some(750),
            stale: false,
        };
        service_before
            .record_usage_snapshot(Some("acct-alpha"), &read_before)
            .unwrap();
        service_after
            .record_usage_snapshot(Some("acct-alpha"), &read_after)
            .unwrap();

        let history = service_after.profile_history(Some("acct-alpha")).unwrap();
        assert_eq!(history.weekly_reset_at, Some(1_209_600));
        assert_eq!(history.observations.len(), 1);
        assert_eq!(history.observations[0].weekly_used_percent, Some(4.0));
    }

    #[test]
    fn record_usage_snapshot_keeps_last_7d_window_with_10min_samples() {
        let cache_path = temp_file("history-2000-cache.json");
        let history_path = temp_file("history-2000.json");
        let account = "acct-alpha";

        // Use a dense stream of 10-minute samples across less than a full 7d span.
        // In this regime pruning should keep all observations (no window truncation yet).
        let iterations = 400_i64;
        for i in 0..iterations {
            let now = i as i64 * 600;
            let service =
                UsageService::new(cache_path.clone(), history_path.clone(), 300).with_now_seconds(now);
            let usage = UsageResponse {
                email: None,
                plan_type: None,
                rate_limit: Some(UsageRateLimit {
                    primary_window: Some(UsageWindow {
                        used_percent: (i % 100) as f64,
                        limit_window_seconds: 18_000,
                        reset_after_seconds: 18_000,
                        reset_at: now,
                    }),
                    secondary_window: Some(UsageWindow {
                        used_percent: (i % 100) as f64,
                        limit_window_seconds: 604_800,
                        reset_after_seconds: 604_800,
                        reset_at: now,
                    }),
                }),
            };
            let read = UsageReadResult {
                usage: Some(usage),
                source: UsageSource::Api,
                fetched_at: Some(now),
                stale: false,
            };
            service.record_usage_snapshot(Some(account), &read).unwrap();
        }

        let service = UsageService::new(cache_path, history_path, 300).with_now_seconds(0);
        let history = service.profile_history(Some(account)).unwrap();
        assert_eq!(
            history.observations.len() as i64,
            iterations,
            "when total span is < 7d worth of 10-minute samples, pruning should not drop observations",
        );
        let expected_end = (iterations - 1) * 600;
        assert!(history
            .observations
            .last()
            .is_some_and(|o| o.observed_at_local == expected_end));
    }

    #[test]
    fn profile_history_point_count_never_shrinks_for_same_account_except_truncate() {
        let cache_path = temp_file("history-monotonic-cache.json");
        let history_path = temp_file("history-monotonic.json");
        let account = "acct-monotonic";

        let mut previous_len = 0usize;
        // Use fewer iterations than the 2000-sample stress test to keep runtime
        // reasonable, but still long enough to exercise 7d pruning behaviour.
        for i in 0..240_i64 {
            let now = i * 600;
            let service =
                UsageService::new(cache_path.clone(), history_path.clone(), 300).with_now_seconds(now);
            let usage = UsageResponse {
                email: None,
                plan_type: None,
                rate_limit: Some(UsageRateLimit {
                    primary_window: Some(UsageWindow {
                        used_percent: (i % 100) as f64,
                        limit_window_seconds: 18_000,
                        reset_after_seconds: 18_000,
                        reset_at: now,
                    }),
                    secondary_window: Some(UsageWindow {
                        used_percent: (i % 100) as f64,
                        limit_window_seconds: 604_800,
                        reset_after_seconds: 604_800,
                        reset_at: now,
                    }),
                }),
            };
            let read = UsageReadResult {
                usage: Some(usage),
                source: UsageSource::Api,
                fetched_at: Some(now),
                stale: false,
            };
            service
                .record_usage_snapshot(Some(account), &read)
                .unwrap();

            // Only sample every few steps to reduce overhead of profile_history().
            if i % 4 == 0 {
                let history = service.profile_history(Some(account)).unwrap();
                let len = history.observations.len();
                // Allow length to shrink only when we've already reached a full 7d window
                // (truncate case). In that situation we expect the count to stay near the
                // theoretical maximum of 1008 points.
                if len < previous_len {
                    assert!(
                        previous_len >= 1008,
                        "history length shrank before reaching full 7d window (prev={previous_len}, now={len})"
                    );
                    assert!(
                        len >= 900,
                        "truncate should not drop history length far below 7d capacity (prev={previous_len}, now={len})"
                    );
                }
                previous_len = len;
            }
        }
    }

    #[test]
    fn record_usage_snapshot_does_not_drop_existing_observations_to_empty() {
        let cache_path = temp_file("history-non-destructive-cache.json");
        let history_path = temp_file("history-non-destructive.json");
        let account = "acct-alpha";

        let seed_service =
            UsageService::new(cache_path.clone(), history_path.clone(), 300).with_now_seconds(1_001);
        let seed_usage = UsageResponse {
            email: None,
            plan_type: None,
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 0.0,
                    limit_window_seconds: 18_000,
                    reset_after_seconds: 18_000,
                    reset_at: 19_000,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 1.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 604_800,
                    reset_at: 605_800,
                }),
            }),
        };
        seed_service
            .record_usage_snapshot(
                Some(account),
                &UsageReadResult {
                    usage: Some(seed_usage),
                    source: UsageSource::Api,
                    fetched_at: Some(1_001),
                    stale: false,
                },
            )
            .unwrap();

        let service = UsageService::new(cache_path, history_path, 300).with_now_seconds(2_000);
        let skewed_usage = UsageResponse {
            email: None,
            plan_type: None,
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 0.0,
                    limit_window_seconds: 18_000,
                    reset_after_seconds: 18_000,
                    reset_at: 20_000,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 2.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 604_800,
                    // weekly start = 2_000, while observed_at=2_000 is excluded by lower-bound rule
                    reset_at: 606_800,
                }),
            }),
        };
        service
            .record_usage_snapshot(
                Some(account),
                &UsageReadResult {
                    usage: Some(skewed_usage),
                    source: UsageSource::Api,
                    fetched_at: Some(2_000),
                    stale: false,
                },
            )
            .unwrap();

        let history = service.profile_history(Some(account)).unwrap();
        assert!(
            !history.observations.is_empty(),
            "refresh must not destructively wipe existing observations"
        );
    }

    #[test]
    fn concurrent_app_and_cron_writers_do_not_clobber_history() {
        use std::thread;

        let cache_path = temp_file("history-concurrent-cache.json");
        let history_path = temp_file("history-concurrent.json");
        let account = "acct-concurrent";
        let base = 1_800_000_000_i64;
        let iterations = 80_i64;

        let app_history_path = history_path.clone();
        let app_cache_path = cache_path.clone();
        let app = thread::spawn(move || {
            for i in 0..iterations {
                let now = base + i * 30;
                let service = UsageService::new(
                    app_cache_path.clone(),
                    app_history_path.clone(),
                    300,
                )
                .with_now_seconds(now);
                let usage = UsageResponse {
                    email: None,
                    plan_type: None,
                    rate_limit: Some(UsageRateLimit {
                        primary_window: Some(UsageWindow {
                            used_percent: (i % 100) as f64,
                            limit_window_seconds: 18_000,
                            reset_after_seconds: 18_000,
                            reset_at: now + 18_000,
                        }),
                        secondary_window: Some(UsageWindow {
                            used_percent: (i % 100) as f64,
                            limit_window_seconds: 604_800,
                            reset_after_seconds: 604_800,
                            reset_at: now + 604_800,
                        }),
                    }),
                };
                let read = UsageReadResult {
                    usage: Some(usage),
                    source: UsageSource::Api,
                    fetched_at: Some(now),
                    stale: false,
                };
                service.record_usage_snapshot(Some(account), &read).unwrap();
            }
        });

        let cron_history_path = history_path.clone();
        let cron_cache_path = cache_path.clone();
        let cron = thread::spawn(move || {
            for i in 0..iterations {
                // Offset by 15s so app/cron observations do not dedupe by timestamp.
                let now = base + i * 600 + 15;
                let service = UsageService::new(
                    cron_cache_path.clone(),
                    cron_history_path.clone(),
                    300,
                )
                .with_now_seconds(now);
                let usage = UsageResponse {
                    email: None,
                    plan_type: None,
                    rate_limit: Some(UsageRateLimit {
                        primary_window: Some(UsageWindow {
                            used_percent: (i % 100) as f64,
                            limit_window_seconds: 18_000,
                            reset_after_seconds: 18_000,
                            reset_at: now + 18_000,
                        }),
                        secondary_window: Some(UsageWindow {
                            used_percent: ((i + 10) % 100) as f64,
                            limit_window_seconds: 604_800,
                            reset_after_seconds: 604_800,
                            reset_at: now + 604_800,
                        }),
                    }),
                };
                let read = UsageReadResult {
                    usage: Some(usage),
                    source: UsageSource::Api,
                    fetched_at: Some(now),
                    stale: false,
                };
                service.record_usage_snapshot(Some(account), &read).unwrap();
            }
        });

        app.join().unwrap();
        cron.join().unwrap();

        let verify = UsageService::new(cache_path, history_path, 300).with_now_seconds(base + 604_790);
        let history = verify.profile_history(Some(account)).unwrap();
        let weekly_points = history
            .observations
            .iter()
            .filter(|obs| obs.weekly_used_percent.is_some())
            .count();
        let five_hour_points = history
            .observations
            .iter()
            .filter(|obs| obs.five_hour_used_percent.is_some())
            .count();

        assert!(
            weekly_points >= 120,
            "concurrent writers should preserve appended weekly observations (got {weekly_points})"
        );
        assert!(
            five_hour_points >= 120,
            "concurrent writers should preserve appended 5h observations (got {five_hour_points})"
        );
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
    fn merge_usage_window_histories_merges_weekly_windows_with_small_reset_jitter() {
        let mut target = vec![UsageWindowHistory {
            limit_window_seconds: WEEKLY_WINDOW_SECONDS,
            start_at: 100,
            end_at: 100 + WEEKLY_WINDOW_SECONDS,
            observations: vec![UsageObservation {
                observed_at: 200,
                used_percent: 10.0,
            }],
        }];
        let source = vec![UsageWindowHistory {
            limit_window_seconds: WEEKLY_WINDOW_SECONDS,
            start_at: 102,
            end_at: 102 + WEEKLY_WINDOW_SECONDS,
            observations: vec![
                UsageObservation {
                    observed_at: 300,
                    used_percent: 20.0,
                },
                UsageObservation {
                    observed_at: 300,
                    used_percent: 20.0,
                },
            ],
        }];

        merge_usage_window_histories(&mut target, source, 3);

        assert_eq!(target.len(), 1, "weekly jitter windows should merge");
        assert_eq!(
            target[0].end_at,
            102 + WEEKLY_WINDOW_SECONDS,
            "merged weekly window uses latest canonical end"
        );
        assert_eq!(
            target[0].start_at,
            target[0].end_at - WEEKLY_WINDOW_SECONDS,
            "start_at should be derived from canonical weekly end"
        );
        assert_eq!(
            target[0].observations.len(),
            2,
            "observations should merge and dedupe"
        );
    }

    #[test]
    fn merge_usage_window_histories_keeps_distinct_weekly_windows_when_beyond_jitter() {
        let mut target = vec![UsageWindowHistory {
            limit_window_seconds: WEEKLY_WINDOW_SECONDS,
            start_at: 0,
            end_at: WEEKLY_WINDOW_SECONDS,
            observations: vec![],
        }];
        let source = vec![UsageWindowHistory {
            limit_window_seconds: WEEKLY_WINDOW_SECONDS,
            start_at: 500,
            end_at: WEEKLY_WINDOW_SECONDS + 500,
            observations: vec![],
        }];

        merge_usage_window_histories(&mut target, source, 3);

        assert_eq!(
            target.len(),
            2,
            "weekly windows outside jitter tolerance should stay separate"
        );
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
                ..ProfileUsageHistory::default()
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
                ..ProfileUsageHistory::default()
            },
        );
        service.write_history_cache(&history).unwrap();

        service
            .merge_profile_history_aliases(Some("canonical"), ["alias"])
            .unwrap();

        let canonical = service.profile_history(Some("canonical")).unwrap();
        let alias = service.profile_history(Some("alias")).unwrap();
        assert_eq!(canonical.weekly_windows.len(), 0);
        assert_eq!(canonical.five_hour_windows.len(), 0);
        assert!(canonical.weekly_reset_at.is_some());
        assert!(canonical.five_hour_reset_at.is_some());
        assert!(canonical.observations.len() >= 2);
        assert!(alias.weekly_windows.is_empty());
        assert!(alias.five_hour_windows.is_empty());
    }

    #[test]
    fn profile_history_migrates_legacy_window_buckets_to_observation_rows_and_resets() {
        let cache_path = temp_file("legacy-migrate-cache.json");
        let history_path = temp_file("legacy-migrate-history.json");
        let service = UsageService::new(cache_path, history_path.clone(), 300);
        let legacy = serde_json::json!({
            "byAccountId": {
                "acct-alpha": {
                    "weekly_windows": [{
                        "limit_window_seconds": 604800,
                        "start_at": 0,
                        "end_at": 604800,
                        "observations": [
                            {"observed_at": 100, "used_percent": 10.0},
                            {"observed_at": 200, "used_percent": 20.0}
                        ]
                    }],
                    "five_hour_windows": [{
                        "limit_window_seconds": 18000,
                        "start_at": 10,
                        "end_at": 18010,
                        "observations": [
                            {"observed_at": 200, "used_percent": 30.0}
                        ]
                    }]
                }
            }
        });
        write_json(&history_path, &legacy).unwrap();

        let migrated = service.profile_history(Some("acct-alpha")).unwrap();
        assert_eq!(migrated.weekly_reset_at, Some(604_800));
        assert_eq!(migrated.five_hour_reset_at, Some(18_010));
        assert_eq!(migrated.weekly_windows.len(), 0);
        assert_eq!(migrated.five_hour_windows.len(), 0);
        assert!(migrated.observations.iter().any(|obs| {
            obs.observed_at_local == 100
                && obs.weekly_used_percent == Some(10.0)
                && obs.five_hour_used_percent.is_none()
        }));
        assert!(migrated.observations.iter().any(|obs| {
            obs.observed_at_local == 200
                && obs.weekly_used_percent == Some(20.0)
                && obs.five_hour_used_percent == Some(30.0)
        }));

        let migrated_raw = std::fs::read_to_string(&history_path).unwrap();
        let migrated_json: serde_json::Value = serde_json::from_str(&migrated_raw).unwrap();
        let acct = &migrated_json["byAccountId"]["acct-alpha"];
        assert!(acct["weekly_windows"].as_array().is_some_and(|list| list.is_empty()));
        assert!(acct["five_hour_windows"].as_array().is_some_and(|list| list.is_empty()));
        assert_eq!(acct["weekly_reset_at"], serde_json::json!(604800));
        assert_eq!(acct["five_hour_reset_at"], serde_json::json!(18010));
        assert!(acct["observations"].as_array().is_some_and(|list| !list.is_empty()));
    }

    #[test]
    fn migration_creates_cycle_summary_table_without_dropping_existing_rows() {
        let cache_path = temp_file("forecast-migrate-cache.json");
        let history_path = temp_file("forecast-migrate-history.json");
        let service = UsageService::new(cache_path, history_path, 300);
        let account = "acct-alpha";

        let cache = UsageCache::from_entries([(
            account.to_string(),
            5_000,
            sample_usage("plus"),
        )]);
        service.write_cache(&cache).unwrap();

        let mut history = UsageHistoryCache::default();
        history.by_account_id.insert(
            account.to_string(),
            ProfileUsageHistory {
                observations: vec![ProfileUsageObservation {
                    observed_at_local: 5_000,
                    weekly_used_percent: Some(41.0),
                    five_hour_used_percent: Some(12.0),
                }],
                weekly_reset_at: Some(604_800),
                five_hour_reset_at: Some(18_000),
                ..ProfileUsageHistory::default()
            },
        );
        service.write_history_cache(&history).unwrap();

        assert_eq!(sqlite_row_count(&service, "usage_cache", account), 1);
        assert_eq!(sqlite_row_count(&service, "usage_observations", account), 1);
        assert!(
            sqlite_table_exists(&service, "five_hour_cycle_summaries"),
            "forecast migration should create cycle-summary storage"
        );
        assert_eq!(
            sqlite_row_count(&service, "five_hour_cycle_summaries", account),
            0,
            "migration should preserve old rows before any summary backfill"
        );
    }

    #[test]
    fn startup_backfill_reconstructs_cycle_summaries_from_retained_observations() {
        let cache_path = temp_file("forecast-backfill-cache.json");
        let history_path = temp_file("forecast-backfill-history.json");
        let service = UsageService::new(cache_path, history_path.clone(), 300);
        let account = "acct-alpha";
        let retained = serde_json::json!({
            "byAccountId": {
                account: {
                    "observations": [
                        {"observed_at_local": 90000, "weekly_used_percent": 10.0, "five_hour_used_percent": 5.0},
                        {"observed_at_local": 93000, "weekly_used_percent": 14.0, "five_hour_used_percent": 35.0},
                        {"observed_at_local": 108100, "weekly_used_percent": 15.0, "five_hour_used_percent": 4.0},
                        {"observed_at_local": 111000, "weekly_used_percent": 17.0, "five_hour_used_percent": 28.0}
                    ],
                    "weekly_reset_at": 604800,
                    "five_hour_reset_at": 126000,
                    "weekly_window_seconds": 604800,
                    "five_hour_window_seconds": 18000,
                    "weekly_windows": [],
                    "five_hour_windows": []
                }
            }
        });
        write_json(&history_path, &retained).unwrap();

        let history = service.profile_history(Some(account)).unwrap();

        assert_eq!(history.observations.len(), 4, "startup backfill must keep raw observations");
        assert!(
            sqlite_table_exists(&service, "five_hour_cycle_summaries"),
            "startup path should migrate db before reading history"
        );
        assert!(
            sqlite_row_count(&service, "five_hour_cycle_summaries", account) >= 2,
            "retained observations spanning multiple 5h segments should backfill summaries"
        );
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
