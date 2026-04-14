use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use agent_switch::app::App;
use agent_switch::db::{SqliteStore, UsageCacheRow};
use agent_switch::paths::AppPaths;
use agent_switch::store::{AccountStore, StorePlatform};
use agent_switch::usage::{UsageReadResult, UsageResponse, UsageService};

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time drift")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("agent-switch-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn account_store_roundtrip_save_use_rename_delete_tracks_current_name(platform: StorePlatform) {
    let root = temp_dir("store");
    let paths = AppPaths::from_codex_dir(root.join(".codex"));
    let store = AccountStore::new(paths.clone(), platform);

    fs::create_dir_all(paths.codex_dir()).expect("codex dir");
    fs::write(
        paths.auth_path(),
        "{\n  \"tokens\": {\"account_id\": \"acct-current\"}\n}\n",
    )
    .expect("write auth");
    // Ensure the initial auth snapshot also carries a UUID tag so that xattr-based
    // current profile resolution works in tests and mirrors real app behavior.
    store.save_account("bootstrap").expect("bootstrap profile with uuid");

    let saved_name = store.save_account("alpha").expect("save account");
    assert_eq!(saved_name, "alpha");
    let names = store.list_account_names().expect("list names");
    assert!(
        names.contains(&"alpha".to_string()),
        "saved profiles should contain alpha, got {:?}",
        names
    );

    store.rename_account("alpha", "beta").expect("rename account");
    let names_after_rename = store.list_account_names().expect("list names");
    assert!(
        names_after_rename.contains(&"beta".to_string())
            && !names_after_rename.contains(&"alpha".to_string()),
        "after rename accounts should contain beta and not alpha, got {:?}",
        names_after_rename
    );

    let activated = store.use_account("beta").expect("use account");
    assert_eq!(activated, "beta");
    assert_eq!(
        fs::read_to_string(paths.current_name_path()).expect("read current"),
        "beta\n"
    );

    let current_name = store.get_current_account_name().expect("current account");
    assert_eq!(current_name.as_deref(), Some("beta"));

    store.delete_account("beta").expect("delete account");
    let final_names = store.list_account_names().expect("list names");
    assert!(
        !final_names.contains(&"beta".to_string()),
        "after deleting beta, saved profiles should no longer contain beta, got {:?}",
        final_names
    );
}

#[test]
fn account_store_roundtrip_copy_all_platforms() {
    account_store_roundtrip_save_use_rename_delete_tracks_current_name(StorePlatform::Copy);
}

#[test]
fn account_store_migrates_legacy_accounts_dir_to_config_dir() {
    let root = temp_dir("migrate-accounts");
    let paths = AppPaths::from_codex_dir(root.join(".codex"));
    let store = AccountStore::new(paths.clone(), StorePlatform::Copy);
    let legacy_accounts = paths.codex_dir().join("accounts");
    fs::create_dir_all(&legacy_accounts).expect("legacy accounts dir");
    fs::create_dir_all(paths.codex_dir()).expect("codex dir");
    fs::write(
        paths.auth_path(),
        "{\n  \"tokens\": {\"account_id\": \"acct-current\"}\n}\n",
    )
    .expect("write auth");
    fs::write(
        legacy_accounts.join("legacy.json"),
        "{\n  \"tokens\": {\"account_id\": \"acct-legacy\"}\n}\n",
    )
    .expect("write legacy account");

    let names = store.list_account_names().expect("list account names");
    assert_eq!(names, vec!["legacy"]);
    assert!(paths.accounts_dir().join("legacy.json").exists());
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

    let service_name = paths
        .limit_cache_path()
        .file_stem()
        .and_then(|name| name.to_str())
        .expect("cache filename stem")
        .to_string();
    let db_path = paths.usage_history_path().with_extension("sqlite.db");
    let db = SqliteStore::new(db_path);
    db.write_usage_cache_rows(
        &service_name,
        &[UsageCacheRow {
            account_key: "acct-1".to_string(),
            fetched_at: 100,
            payload_json: serde_json::to_string(&cached_usage).expect("serialize cache payload"),
        }],
    )
    .expect("seed sqlite cache");

    let service = UsageService::new(
        paths.limit_cache_path().to_path_buf(),
        paths.usage_history_path().to_path_buf(),
        300,
    )
        .with_now_seconds(500)
        .with_fetcher(|_, _| Err(anyhow::anyhow!("boom")));

    let result = service
        .read_usage(Some("acct-1"), Some("token-1"), false, false)
        .expect("read usage");

    assert_eq!(
        result,
        UsageReadResult {
            usage: Some(cached_usage),
            source: agent_switch::usage::UsageSource::Cache,
            fetched_at: Some(100),
            stale: true,
        }
    );
}

#[test]
fn app_state_tracks_selected_profile_and_navigates() {
    let app = App::from_profile_names(vec!["Alpha".to_string(), "Beta".to_string()], 1);
    assert_eq!(app.selected_profile_label(), Some("Beta"));

    let app = app.select_previous_profile();
    assert_eq!(app.selected_profile_label(), Some("Alpha"));
}
