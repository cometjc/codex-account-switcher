use std::fs;
use std::path::PathBuf;

use agent_switch::loader::load_profiles_with_report;
use agent_switch::paths::AppPaths;
use agent_switch::store::{AccountStore, StorePlatform};
use agent_switch::usage::{UsageCache, UsageResponse, UsageService, UsageRateLimit, UsageWindow};

fn temp_dir(label: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time drift")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("agent-switch-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

#[test]
fn codex_saved_profile_merges_old_history_into_current_id() {
    let root = temp_dir("codex-history-alias");
    let paths = AppPaths::from_codex_dir(root.join(".codex"));
    let store = AccountStore::new(paths.clone(), StorePlatform::Copy);

    fs::create_dir_all(paths.codex_dir()).expect("codex dir");

    // Legacy saved snapshot with old account_id.
    let stale_snapshot = serde_json::json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "account_id": "acct-old",
            "access_token": "token-old",
            "refresh_token": "refresh-old"
        }
    });
    store.save_snapshot("team", &stale_snapshot).expect("save stale");

    // Current auth snapshot with new account_id; same saved name "team".
    let current_snapshot = serde_json::json!({
        "auth_mode": "chatgpt",
        "tokens": {
            "account_id": "acct-new",
            "access_token": "token-new",
            "refresh_token": "refresh-new"
        }
    });
    fs::write(
        paths.auth_path(),
        serde_json::to_vec_pretty(&current_snapshot).unwrap(),
    )
    .expect("write auth");
    fs::write(paths.current_name_path(), "team\n").expect("write current name");

    // Seed history under the old account_id.
    let history_service = UsageService::new(
        paths.limit_cache_path().to_path_buf(),
        paths.usage_history_path().to_path_buf(),
        300,
    )
    .with_now_seconds(1_700_000_000);
    let usage = UsageResponse {
        email: None,
        plan_type: Some("team".to_string()),
        rate_limit: Some(UsageRateLimit {
            primary_window: Some(UsageWindow {
                used_percent: 10.0,
                limit_window_seconds: 18_000,
                reset_after_seconds: 18_000,
                reset_at: 1_700_000_000 + 18_000,
            }),
            secondary_window: Some(UsageWindow {
                used_percent: 20.0,
                limit_window_seconds: 604_800,
                reset_after_seconds: 604_800,
                reset_at: 1_700_000_000 + 604_800,
            }),
        }),
    };
    history_service
        .record_usage_snapshot(
            Some("acct-old"),
            &agent_switch::usage::UsageReadResult {
                usage: Some(usage),
                source: agent_switch::usage::UsageSource::Api,
                fetched_at: Some(1_700_000_000),
                stale: false,
            },
        )
        .expect("seed old history");

    // Now load profiles; loader should detect that "team" uses current auth and merge
    // history from acct-old into acct-new so charts don't reset.
    let usage = UsageService::new(
        paths.limit_cache_path().to_path_buf(),
        paths.usage_history_path().to_path_buf(),
        300,
    )
    .with_now_seconds(1_700_000_100);
    let report =
        load_profiles_with_report(&store, &usage, false, None, true, None, None, None).unwrap();

    let codex_profile = report
        .profiles
        .into_iter()
        .find(|p| matches!(p.kind, agent_switch::app_data::ProfileKind::Codex))
        .expect("codex profile");
    assert_eq!(codex_profile.account_id.as_deref(), Some("acct-new"));
    let history = usage.profile_history(Some("acct-new")).unwrap();
    assert!(
        !history.observations.is_empty(),
        "merged history for acct-new should reuse observations from acct-old"
    );
}

