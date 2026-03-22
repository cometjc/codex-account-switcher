use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use codex_auth::app::{App, ViewMode};
use codex_auth::paths::AppPaths;
use codex_auth::store::{AccountStore, StorePlatform};
use codex_auth::usage::{UsageCache, UsageReadResult, UsageResponse, UsageService};

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time drift")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("codex-auth-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn account_store_roundtrip_save_use_rename_delete_tracks_current_name() {
    let root = temp_dir("store");
    let paths = AppPaths::from_codex_dir(root.join(".codex"));
    let store = AccountStore::new(paths.clone(), StorePlatform::Symlink);

    fs::create_dir_all(paths.codex_dir()).expect("codex dir");
    fs::write(
        paths.auth_path(),
        "{\n  \"tokens\": {\"account_id\": \"acct-current\"}\n}\n",
    )
    .expect("write auth");

    let saved_name = store.save_account("alpha").expect("save account");
    assert_eq!(saved_name, "alpha");
    assert_eq!(store.list_account_names().expect("list names"), vec!["alpha"]);

    store.rename_account("alpha", "beta").expect("rename account");
    assert_eq!(store.list_account_names().expect("list names"), vec!["beta"]);

    let activated = store.use_account("beta").expect("use account");
    assert_eq!(activated, "beta");
    assert_eq!(
        fs::read_to_string(paths.current_name_path()).expect("read current"),
        "beta\n"
    );

    let current_name = store.get_current_account_name().expect("current account");
    assert_eq!(current_name.as_deref(), Some("beta"));

    store.delete_account("beta").expect("delete account");
    assert!(store.list_account_names().expect("list names").is_empty());
}

#[test]
fn usage_service_returns_stale_cache_when_fetch_fails() {
    let root = temp_dir("usage");
    let paths = AppPaths::from_codex_dir(root.join(".codex"));
    fs::create_dir_all(paths.codex_dir()).expect("codex dir");

    let cached_usage = UsageResponse {
        email: Some("cached@example.com".to_string()),
        plan_type: Some("plus".to_string()),
        rate_limit: None,
    };

    let cache = UsageCache::from_entries([(
        "acct-1".to_string(),
        100,
        cached_usage.clone(),
    )]);
    fs::write(
        paths.limit_cache_path(),
        serde_json::to_string_pretty(&cache).expect("serialize cache"),
    )
    .expect("write cache");

    let service = UsageService::new(paths.limit_cache_path().to_path_buf(), 300)
        .with_now_seconds(500)
        .with_fetcher(|_, _| Err(anyhow::anyhow!("boom")));

    let result = service
        .read_usage(Some("acct-1"), Some("token-1"), false, false)
        .expect("read usage");

    assert_eq!(
        result,
        UsageReadResult {
            usage: Some(cached_usage),
            source: codex_auth::usage::UsageSource::Cache,
            fetched_at: Some(100),
            stale: true,
        }
    );
}

#[test]
fn app_state_toggles_between_accounts_and_plot_modes() {
    let app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 1);
    assert_eq!(app.view_mode(), ViewMode::Accounts);
    assert_eq!(app.selected_profile_label(), Some("Beta"));

    let app = app.toggle_plot_mode();
    assert_eq!(app.view_mode(), ViewMode::Plot);
    assert_eq!(app.selected_profile_label(), Some("Beta"));

    let app = app.select_previous_profile();
    assert_eq!(app.selected_profile_label(), Some("Alpha"));

    let app = app.toggle_plot_mode();
    assert_eq!(app.view_mode(), ViewMode::Accounts);
    assert_eq!(app.selected_profile_label(), Some("Alpha"));
}
