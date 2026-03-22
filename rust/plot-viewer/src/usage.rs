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

#[derive(Clone)]
pub struct UsageService {
    cache_path: PathBuf,
    ttl_seconds: i64,
    now_seconds: i64,
    fetcher: Arc<Fetcher>,
}

impl UsageService {
    pub fn new(cache_path: PathBuf, ttl_seconds: i64) -> Self {
        Self {
            cache_path,
            ttl_seconds,
            now_seconds: default_now_seconds(),
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
        self.now_seconds = now_seconds;
        self
    }

    pub fn read_usage(
        &self,
        account_id: Option<&str>,
        access_token: Option<&str>,
        force_refresh: bool,
        cache_only: bool,
    ) -> Result<UsageReadResult> {
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
            .map(|record| self.now_seconds - record.fetched_at)
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
                let fetched_at = self.now_seconds;
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
