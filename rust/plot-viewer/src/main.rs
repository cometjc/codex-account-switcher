use anyhow::Result;
use codex_auth::app::App;
use codex_auth::cron;
use codex_auth::paths::AppPaths;
use codex_auth::store::{AccountStore, StorePlatform};
use codex_auth::usage::UsageService;
use codex_auth::claude::{ClaudePaths, ClaudeStore, ClaudeCredentials};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--refresh-all") {
        return run_refresh_all();
    }

    let paths = AppPaths::detect();
    let binary_path = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("codex-auth"))
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
        ).with_fetcher(codex_auth::claude::fetch_claude_usage);
        (Some(cl_store), Some(cl_usage))
    } else {
        (None, None)
    };

    let mut app = App::load(store, usage, cron_status, claude_store, claude_usage_service)?;
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

    // Force-refresh usage for every saved profile
    let saved = store.list_saved_profiles()?;
    for profile in &saved {
        let account_id = profile.snapshot
            .get("tokens")
            .and_then(|t| t.get("account_id"))
            .and_then(|v| v.as_str());
        let access_token = profile.snapshot
            .get("tokens")
            .and_then(|t| t.get("access_token"))
            .and_then(|v| v.as_str());
        let _ = usage.read_usage(account_id, access_token, true, false);
        if let Ok(result) = usage.read_usage(account_id, access_token, false, false) {
            let _ = usage.record_usage_snapshot(account_id, result.usage.as_ref());
        }
    }

    // Also refresh the current (unsaved) profile
    if let Ok(current) = store.get_current_snapshot() {
        let account_id = current.get("tokens")
            .and_then(|t| t.get("account_id"))
            .and_then(|v| v.as_str());
        let access_token = current.get("tokens")
            .and_then(|t| t.get("access_token"))
            .and_then(|v| v.as_str());
        if let Ok(result) = usage.read_usage(account_id, access_token, true, false) {
            let _ = usage.record_usage_snapshot(account_id, result.usage.as_ref());
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
        ).with_fetcher(codex_auth::claude::fetch_claude_usage);

        if let Ok(saved) = cl_store.list_saved_profiles() {
            for profile in saved {
                if let Ok(creds) = serde_json::from_value::<ClaudeCredentials>(profile.snapshot) {
                    let composite_id = format!("{}|{}", creds.account_id(), creds.subscription_type());
                    let _ = cl_usage.read_usage(Some(composite_id.as_str()), Some(creds.access_token()), true, false);
                    if let Ok(result) = cl_usage.read_usage(Some(composite_id.as_str()), Some(creds.access_token()), false, false) {
                        let _ = cl_usage.record_usage_snapshot(Some(composite_id.as_str()), result.usage.as_ref());
                    }
                }
            }
        }

        if let Ok(creds) = cl_store.get_current_credentials() {
            let composite_id = format!("{}|{}", creds.account_id(), creds.subscription_type());
            let _ = cl_usage.read_usage(Some(composite_id.as_str()), Some(creds.access_token()), true, false);
            if let Ok(result) = cl_usage.read_usage(Some(composite_id.as_str()), Some(creds.access_token()), false, false) {
                let _ = cl_usage.record_usage_snapshot(Some(composite_id.as_str()), result.usage.as_ref());
            }
        }
    }

    cron::write_last_run_success(paths.cron_status_path())?;
    Ok(())
}
