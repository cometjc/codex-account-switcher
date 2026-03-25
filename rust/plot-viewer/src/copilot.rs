//! GitHub Copilot credential detection and usage fetching.
//!
//! Reads the active GitHub OAuth token from ~/.config/gh/hosts.yml
//! and queries the Copilot internal user API for monthly chat quota usage.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::usage::{UsageRateLimit, UsageResponse, UsageWindow};

// ── Paths ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopilotPaths {
    gh_hosts_path: PathBuf,
    copilot_config_path: PathBuf,
    limit_cache_path: PathBuf,
    usage_history_path: PathBuf,
}

impl CopilotPaths {
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let gh_config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("gh");
        let cache_dir = home.join(".config").join("agent-switch").join("copilot");
        Self {
            gh_hosts_path: gh_config_dir.join("hosts.yml"),
            copilot_config_path: home.join(".copilot").join("config.json"),
            limit_cache_path: cache_dir.join("limit-cache.json"),
            usage_history_path: cache_dir.join("usage-history.json"),
        }
    }

    pub fn gh_hosts_path(&self) -> &Path { &self.gh_hosts_path }
    pub fn copilot_config_path(&self) -> &Path { &self.copilot_config_path }
    pub fn limit_cache_path(&self) -> &Path { &self.limit_cache_path }
    pub fn usage_history_path(&self) -> &Path { &self.usage_history_path }
}

// ── Credentials ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopilotCredentials {
    pub login: String,
    pub oauth_token: String,
}

impl CopilotCredentials {
    /// Stable unique identifier for this Copilot account.
    pub fn account_id(&self) -> String {
        format!("copilot-{}", self.login)
    }

    /// Try to detect credentials, preferring the copilot CLI's own token store
    /// (`~/.copilot/config.json`) over the gh CLI hosts file.
    pub fn detect() -> Option<Self> {
        let paths = CopilotPaths::detect();
        detect_from_copilot_config(paths.copilot_config_path())
            .or_else(|_| detect_from_path(paths.gh_hosts_path()))
            .ok()
    }
}

pub fn detect_from_path(path: &Path) -> Result<CopilotCredentials> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    parse_gh_hosts(&raw)
        .with_context(|| format!("parse {}", path.display()))
}

pub fn detect_from_copilot_config(path: &Path) -> Result<CopilotCredentials> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    parse_copilot_config(&raw)
        .with_context(|| format!("parse {}", path.display()))
}

/// Parse ~/.config/gh/hosts.yml to extract the active GitHub user and OAuth token.
///
/// Expected format (4-space indented under `github.com:`):
/// ```yaml
/// github.com:
///     user: jcppkkk
///     oauth_token: gho_...
/// ```
fn parse_gh_hosts(raw: &str) -> Result<CopilotCredentials> {
    let mut in_github_section = false;
    let mut login: Option<String> = None;
    let mut oauth_token: Option<String> = None;

    for line in raw.lines() {
        if line.starts_with("github.com:") {
            in_github_section = true;
            continue;
        }
        // Non-whitespace non-empty line = new top-level key
        if line.starts_with(|c: char| !c.is_whitespace()) && !line.trim().is_empty() {
            if in_github_section {
                break;
            }
            continue;
        }
        if !in_github_section {
            continue;
        }

        let indent = line.len() - line.trim_start().len();
        // Only pick direct children of github.com (4-space indent)
        if indent != 4 {
            continue;
        }

        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("user:") {
            login = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("oauth_token:") {
            let token = val.trim().to_string();
            if !token.is_empty() {
                oauth_token = Some(token);
            }
        }
    }

    match (login, oauth_token) {
        (Some(login), Some(oauth_token)) => Ok(CopilotCredentials { login, oauth_token }),
        (None, _) => bail!("no GitHub user found in gh hosts.yml"),
        (_, None) => bail!("no oauth_token found in gh hosts.yml"),
    }
}

/// Parse ~/.copilot/config.json to extract the last-logged-in user and their token.
///
/// Expected format:
/// ```json
/// {
///   "last_logged_in_user": {"host": "https://github.com", "login": "alice"},
///   "copilot_tokens": {"https://github.com:alice": "gho_..."}
/// }
/// ```
fn parse_copilot_config(raw: &str) -> Result<CopilotCredentials> {
    #[derive(serde::Deserialize)]
    struct UserEntry {
        host: String,
        login: String,
    }
    #[derive(serde::Deserialize)]
    struct CopilotConfig {
        last_logged_in_user: Option<UserEntry>,
        copilot_tokens: Option<std::collections::HashMap<String, String>>,
    }

    let cfg: CopilotConfig = serde_json::from_str(raw)
        .context("parse copilot config.json")?;

    let user = cfg.last_logged_in_user
        .ok_or_else(|| anyhow::anyhow!("no last_logged_in_user in copilot config.json"))?;

    let key = format!("{}:{}", user.host.trim_end_matches('/'), user.login);
    let token = cfg.copilot_tokens
        .as_ref()
        .and_then(|m| m.get(&key))
        .ok_or_else(|| anyhow::anyhow!("no copilot_token for {key}"))?
        .clone();

    Ok(CopilotCredentials { login: user.login, oauth_token: token })
}

// ── API response types ────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CopilotUserResponse {
    login: Option<String>,
    copilot_plan: Option<String>,
    // Individual/free plan fields
    monthly_quotas: Option<CopilotQuotas>,
    limited_user_quotas: Option<CopilotQuotas>,
    limited_user_reset_date: Option<String>,
    // Business/Pro plan fields
    quota_snapshots: Option<CopilotQuotaSnapshots>,
    quota_reset_date_utc: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CopilotQuotas {
    chat: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CopilotQuotaEntry {
    pub(crate) entitlement: u64,
    pub(crate) remaining: u64,
    pub(crate) percent_remaining: f64,
    pub(crate) unlimited: bool,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct CopilotQuotaSnapshots {
    pub(crate) premium_interactions: Option<CopilotQuotaEntry>,
}

// ── Usage fetching ────────────────────────────────────────────────────────────

pub fn fetch_copilot_usage(_account_id: &str, access_token: &str) -> Result<UsageResponse> {
    let client = reqwest::blocking::Client::builder()
        .build()
        .context("build reqwest client")?;
    let response: CopilotUserResponse = client
        .get("https://api.github.com/copilot_internal/user")
        .bearer_auth(access_token)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "agent-switch")
        .send()
        .context("send Copilot usage request")?
        .error_for_status()
        .context("Copilot usage request failed")?
        .json()
        .context("parse Copilot usage JSON")?;

    map_copilot_usage_response(&response)
        .ok_or_else(|| anyhow::anyhow!("unexpected Copilot usage response shape"))
}

pub(crate) fn map_copilot_usage_response(r: &CopilotUserResponse) -> Option<UsageResponse> {
    let plan_type = r.copilot_plan.clone().unwrap_or_else(|| "copilot".to_string());

    // ── Business/Pro path: quota_snapshots.premium_interactions ──────────────
    if let Some(snapshots) = &r.quota_snapshots {
        if let Some(pi) = &snapshots.premium_interactions {
            if !pi.unlimited {
                let used_percent = (100.0 - pi.percent_remaining).clamp(0.0, 100.0);
                let reset_date = r.quota_reset_date_utc.as_deref().unwrap_or("");
                let reset_at = crate::claude::parse_rfc3339_unix(reset_date).unwrap_or(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let reset_after_seconds = (reset_at - now).max(0);
                return Some(UsageResponse {
                    email: r.login.clone(),
                    plan_type: Some(plan_type),
                    rate_limit: Some(UsageRateLimit {
                        primary_window: None,
                        secondary_window: Some(UsageWindow {
                            used_percent,
                            limit_window_seconds: 604_800,
                            reset_at,
                            reset_after_seconds,
                        }),
                    }),
                });
            }
        }
    }

    // ── Individual/free path: limited_user_quotas.chat ───────────────────────
    let total = r.monthly_quotas.as_ref()?.chat? as f64;
    let remaining = r.limited_user_quotas.as_ref()?.chat? as f64;
    let used = (total - remaining).max(0.0);
    let used_percent = if total > 0.0 {
        (used / total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let reset_date = r.limited_user_reset_date.as_deref().unwrap_or("");
    let reset_at = parse_date_to_unix(reset_date).unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let reset_after_seconds = (reset_at - now).max(0);

    Some(UsageResponse {
        email: r.login.clone(),
        plan_type: Some(plan_type),
        rate_limit: Some(UsageRateLimit {
            primary_window: None,
            // Monthly quota goes into secondary_window with limit_window_seconds=604_800
            // so that pick_weekly_window() surfaces it for display.
            secondary_window: Some(UsageWindow {
                used_percent,
                limit_window_seconds: 604_800,
                reset_at,
                reset_after_seconds,
            }),
        }),
    })
}

/// Parse "YYYY-MM-DD" to unix timestamp at midnight UTC.
fn parse_date_to_unix(date: &str) -> Option<i64> {
    crate::claude::parse_rfc3339_unix(&format!("{date}T00:00:00+00:00"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gh_hosts_extracts_user_and_token() {
        let raw = "github.com:\n    git_protocol: ssh\n    user: jcppkkk\n    oauth_token: gho_abc123\n";
        let creds = parse_gh_hosts(raw).unwrap();
        assert_eq!(creds.login, "jcppkkk");
        assert_eq!(creds.oauth_token, "gho_abc123");
    }

    #[test]
    fn parse_gh_hosts_ignores_nested_user_tokens() {
        // The nested tokens under `users:` section should be ignored (they have deeper indent)
        let raw = "github.com:\n    users:\n        jcppkkk:\n            oauth_token: gho_nested\n    user: jcppkkk\n    oauth_token: gho_toplevel\n";
        let creds = parse_gh_hosts(raw).unwrap();
        assert_eq!(creds.oauth_token, "gho_toplevel");
    }

    #[test]
    fn map_copilot_usage_response_computes_percent() {
        let r = CopilotUserResponse {
            login: Some("alice".to_string()),
            copilot_plan: Some("individual".to_string()),
            monthly_quotas: Some(CopilotQuotas { chat: Some(500) }),
            limited_user_quotas: Some(CopilotQuotas { chat: Some(290) }),
            limited_user_reset_date: Some("2026-03-28".to_string()),
            quota_snapshots: None,
            quota_reset_date_utc: None,
        };
        let usage = map_copilot_usage_response(&r).unwrap();
        assert_eq!(usage.plan_type.as_deref(), Some("individual"));
        let window = usage.rate_limit.unwrap().secondary_window.unwrap();
        // (500-290)/500 * 100 = 42%
        assert!((window.used_percent - 42.0).abs() < 0.01, "expected ~42%, got {}", window.used_percent);
        assert!(window.reset_at > 0);
    }

    #[test]
    fn parse_copilot_config_extracts_login_and_token() {
        let raw = r#"{
            "logged_in_users": [{"host": "https://github.com", "login": "teamt5-it"}],
            "last_logged_in_user": {"host": "https://github.com", "login": "teamt5-it"},
            "copilot_tokens": {
                "https://github.com:teamt5-it": "gho_businesstoken"
            }
        }"#;
        let creds = parse_copilot_config(raw).unwrap();
        assert_eq!(creds.login, "teamt5-it");
        assert_eq!(creds.oauth_token, "gho_businesstoken");
    }

    #[test]
    fn parse_copilot_config_returns_err_when_no_last_user() {
        let raw = r#"{"copilot_tokens": {"https://github.com:x": "tok"}}"#;
        assert!(parse_copilot_config(raw).is_err());
    }

    #[test]
    fn map_copilot_usage_response_handles_business_quota_snapshots() {
        let r = CopilotUserResponse {
            login: Some("teamt5-it".to_string()),
            copilot_plan: Some("business".to_string()),
            monthly_quotas: None,
            limited_user_quotas: None,
            limited_user_reset_date: None,
            quota_snapshots: Some(CopilotQuotaSnapshots {
                premium_interactions: Some(CopilotQuotaEntry {
                    entitlement: 300,
                    remaining: 40,
                    percent_remaining: 13.4,
                    unlimited: false,
                }),
            }),
            quota_reset_date_utc: Some("2026-04-01T00:00:00.000Z".to_string()),
        };
        let usage = map_copilot_usage_response(&r).unwrap();
        assert_eq!(usage.plan_type.as_deref(), Some("business"));
        let window = usage.rate_limit.unwrap().secondary_window.unwrap();
        // used = 100 - 13.4 = 86.6
        assert!((window.used_percent - 86.6).abs() < 0.1,
            "expected ~86.6%, got {}", window.used_percent);
        assert!(window.reset_at > 0);
    }

    #[test]
    fn map_copilot_usage_response_falls_back_to_individual_schema() {
        let r = CopilotUserResponse {
            login: Some("alice".to_string()),
            copilot_plan: Some("individual".to_string()),
            monthly_quotas: Some(CopilotQuotas { chat: Some(500) }),
            limited_user_quotas: Some(CopilotQuotas { chat: Some(290) }),
            limited_user_reset_date: Some("2026-03-28".to_string()),
            quota_snapshots: None,
            quota_reset_date_utc: None,
        };
        let usage = map_copilot_usage_response(&r).unwrap();
        let window = usage.rate_limit.unwrap().secondary_window.unwrap();
        assert!((window.used_percent - 42.0).abs() < 0.01,
            "expected ~42%, got {}", window.used_percent);
    }
}
