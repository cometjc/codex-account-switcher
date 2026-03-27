use anyhow::{Result, bail};
use agent_switch::app::App;
use agent_switch::cron;
use agent_switch::paths::AppPaths;
use agent_switch::store::{AccountStore, StorePlatform};
use agent_switch::usage::UsageService;
use agent_switch::claude::{ClaudePaths, ClaudeStore, ClaudeCredentials};
use agent_switch::copilot::{CopilotCredentials, CopilotPaths};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--refresh-all") {
        return run_refresh_all();
    }

    let paths = AppPaths::detect();
    let binary_path = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("agent-switch"))
        .to_string_lossy()
        .into_owned();
    let _ = cron::ensure_installed(&binary_path);
    let cron_status = cron::read_status(paths.cron_status_path());

    let store = AccountStore::new(paths.clone(), StorePlatform::detect());
    let usage = UsageService::new(
        paths.limit_cache_path().to_path_buf(),
        paths.usage_history_path().to_path_buf(),
        300,
    );

    // Claude setup
    let claude_paths = ClaudePaths::detect();
    let (claude_store, claude_usage_service) = if claude_paths.credentials_path().exists() {
        let cl_store = ClaudeStore::new(claude_paths.clone());
        let cl_usage = UsageService::new(
            claude_paths.limit_cache_path().to_path_buf(),
            claude_paths.usage_history_path().to_path_buf(),
            300,
        ).with_fetcher(agent_switch::claude::fetch_claude_usage);
        (Some(cl_store), Some(cl_usage))
    } else {
        (None, None)
    };

    // Copilot setup
    let copilot_usage_service = CopilotCredentials::detect().map(|_| {
        let paths = CopilotPaths::detect();
        UsageService::new(
            paths.limit_cache_path().to_path_buf(),
            paths.usage_history_path().to_path_buf(),
            300,
        ).with_fetcher(agent_switch::copilot::fetch_copilot_usage)
    });

    let mut app = App::load(store, usage, cron_status, claude_store, claude_usage_service, copilot_usage_service)?;
    app.run()
}

/// Cron mode: refresh usage for all saved profiles and write the success timestamp.
fn run_refresh_all() -> Result<()> {
    let paths = AppPaths::detect();
    let store = AccountStore::new(paths.clone(), StorePlatform::detect());
    let usage = UsageService::new(
        paths.limit_cache_path().to_path_buf(),
        paths.usage_history_path().to_path_buf(),
        300,
    );
    let mut codex_errors = Vec::new();
    let mut claude_errors = Vec::new();
    let mut copilot_errors = Vec::new();

    // Force-refresh usage for every saved profile
    let saved = store.list_saved_profiles()?;
    let current_snapshot = store.get_current_snapshot().ok();
    let current_account_id = current_snapshot.as_ref().and_then(|snapshot| {
        snapshot
            .get("tokens")
            .and_then(|t| t.get("account_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    });
    let current_name = store.get_current_account_name().ok().flatten();
    let mut current_saved = false;
    for profile in &saved {
        let saved_account_id = profile.snapshot
            .get("tokens")
            .and_then(|t| t.get("account_id"))
            .and_then(|v| v.as_str());
        let use_current_auth = current_account_id.as_deref() == saved_account_id
            || current_name.as_deref() == Some(profile.name.as_str());
        let effective_account_id = if use_current_auth {
            current_account_id.as_deref().or(saved_account_id)
        } else {
            saved_account_id
        };
        let auth_path = if use_current_auth {
            current_saved = true;
            paths.auth_path()
        } else {
            profile.file_path.as_path()
        };
        if let Err(error) = refresh_codex_usage_snapshot(&usage, auth_path, effective_account_id) {
            codex_errors.push(format!("saved {}: {error:#}", profile.name));
        } else if use_current_auth {
            let _ = store.update_account(&profile.name);
        }
    }

    // Also refresh the current (unsaved) profile
    if !current_saved {
        if let Some(current) = current_snapshot {
            let account_id = current.get("tokens")
                .and_then(|t| t.get("account_id"))
                .and_then(|v| v.as_str());
            if let Err(error) = refresh_codex_usage_snapshot(&usage, paths.auth_path(), account_id) {
                codex_errors.push(format!("current: {error:#}"));
            }
        }
    }

    // Refresh Claude profiles
    let claude_paths = ClaudePaths::detect();
    if claude_paths.credentials_path().exists() {
        let cl_store = ClaudeStore::new(claude_paths.clone());
        let cl_usage = UsageService::new(
            claude_paths.limit_cache_path().to_path_buf(),
            claude_paths.usage_history_path().to_path_buf(),
            300,
        ).with_fetcher(agent_switch::claude::fetch_claude_usage);
        let current_creds = cl_store.get_current_credentials().ok();
        let current_account_id = current_creds.as_ref().map(|creds| creds.account_id());
        let current_name = cl_store.get_current_account_name().ok().flatten();
        let mut current_saved = false;

        if let Ok(saved) = cl_store.list_saved_profiles() {
            for profile in saved {
                if let Ok(creds) = serde_json::from_value::<ClaudeCredentials>(profile.snapshot) {
                    let use_current_credentials = current_name.as_deref() == Some(profile.name.as_str())
                        || (current_account_id.is_some()
                            && current_account_id.as_deref() == Some(creds.account_id().as_str()));
                    let effective_creds = if use_current_credentials {
                        current_saved = true;
                        current_creds.as_ref().unwrap_or(&creds)
                    } else {
                        &creds
                    };
                    let composite_id = format!(
                        "{}|{}",
                        effective_creds.account_id(),
                        effective_creds.subscription_type()
                    );
                    if let Err(error) = refresh_usage_snapshot(
                        &cl_usage,
                        Some(composite_id.as_str()),
                        Some(effective_creds.access_token()),
                    ) {
                        claude_errors.push(format!("saved {}: {error:#}", profile.name));
                    } else if use_current_credentials {
                        let _ = cl_store.update_account(&profile.name);
                    }
                }
            }
        }

        if !current_saved {
            if let Some(creds) = current_creds {
                let composite_id = format!("{}|{}", creds.account_id(), creds.subscription_type());
                if let Err(error) = refresh_usage_snapshot(
                    &cl_usage,
                    Some(composite_id.as_str()),
                    Some(creds.access_token()),
                ) {
                    claude_errors.push(format!("current: {error:#}"));
                }
            }
        }
    }

    // Refresh Copilot usage
    if let Some(creds) = CopilotCredentials::detect() {
        let copilot_paths = CopilotPaths::detect();
        let copilot_usage = UsageService::new(
            copilot_paths.limit_cache_path().to_path_buf(),
            copilot_paths.usage_history_path().to_path_buf(),
            300,
        ).with_fetcher(agent_switch::copilot::fetch_copilot_usage);
        let account_id = creds.account_id();
        if let Err(error) = refresh_usage_snapshot(&copilot_usage, Some(account_id.as_str()), Some(creds.oauth_token.as_str())) {
            copilot_errors.push(format!("{error:#}"));
        }
    }

    let report = cron::CronRunReport {
        attempted_at: current_unix_seconds(),
        codex_error: summarize_client_errors("Codex", &codex_errors),
        claude_error: summarize_client_errors("Claude", &claude_errors),
        copilot_error: summarize_client_errors("Copilot", &copilot_errors),
    };
    cron::write_run_report(paths.cron_status_path(), &report)?;
    if report.has_errors() {
        bail!(
            "{}",
            [report.codex_error.as_deref(), report.claude_error.as_deref(), report.copilot_error.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    Ok(())
}

fn refresh_usage_snapshot(
    usage: &UsageService,
    account_id: Option<&str>,
    access_token: Option<&str>,
) -> Result<()> {
    let result = usage.read_usage(account_id, access_token, true, false)?;
    usage.record_usage_snapshot(account_id, result.usage.as_ref())
}

fn refresh_codex_usage_snapshot(
    usage: &UsageService,
    auth_path: &std::path::Path,
    account_id: Option<&str>,
) -> Result<()> {
    let result = usage.read_codex_usage(auth_path, true, false)?;
    usage.record_usage_snapshot(account_id, result.usage.as_ref())
}

fn summarize_client_errors(service: &str, errors: &[String]) -> Option<String> {
    match errors {
        [] => None,
        [single] => Some(format!("{service}: {single}")),
        [first, rest @ ..] => Some(format!(
            "{service}: {first} (and {} more error{})",
            rest.len(),
            if rest.len() == 1 { "" } else { "s" }
        )),
    }
}

fn current_unix_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_usage_snapshot_surfaces_force_refresh_errors() {
        let cache_path = std::env::temp_dir().join(format!(
            "agent-switch-main-refresh-cache-{}",
            std::process::id()
        ));
        let history_path = std::env::temp_dir().join(format!(
            "agent-switch-main-refresh-history-{}",
            std::process::id()
        ));
        let usage = UsageService::new(cache_path, history_path, 300)
            .with_fetcher(|_, _| Err(anyhow::anyhow!("fetch failed")));

        let result = refresh_usage_snapshot(&usage, Some("acct-alpha"), Some("token"));

        assert!(result.is_err(), "refresh helper should not swallow fetch failures");
    }
}
