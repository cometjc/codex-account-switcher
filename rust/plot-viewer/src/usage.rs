use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

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
            Err(_) => Ok(cached
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
                })),
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
            trim_history_windows(target_windows);
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

fn trim_history_windows(windows: &mut Vec<UsageWindowHistory>) {
    windows.sort_by_key(|window| (window.start_at, window.end_at));
    while windows.len() > 6 {
        windows.remove(0);
    }
    for window in windows {
        window.observations.sort_by_key(|observation| observation.observed_at);
        if window.observations.len() > 256 {
            let remove_count = window.observations.len() - 256;
            window.observations.drain(0..remove_count);
        }
    }
}

fn fetch_usage_from_api(account_id: &str, access_token: &str) -> Result<UsageResponse> {
    let client = Client::builder().build().context("build reqwest client")?;
    let response = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .bearer_auth(access_token)
        .header("ChatGPT-Account-Id", account_id)
        .header("User-Agent", "codex-auth")
        .send()
        .context("send usage request")?
        .error_for_status()
        .context("usage request failed")?;
    response.json().context("parse usage response JSON")
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

    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "codex-auth-usage-tests-{}-{}",
            std::process::id(),
            name
        ));
        let _ = fs::remove_file(&base);
        base
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
}
