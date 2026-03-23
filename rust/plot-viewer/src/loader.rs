use anyhow::Result;
use serde_json::Value;

use crate::app_data::{
    OwnedFiveHourBandState, OwnedFiveHourSubframeState, ProfileChartData, ProfileEntry, ProfileKind,
};
use crate::render::ChartPoint;
use crate::store::{AccountStore, SavedProfile};
use crate::usage::{
    pick_five_hour_window, pick_weekly_window, UsageResponse, UsageService, UsageWindow,
    UsageWindowHistory,
};

pub fn load_profiles(
    store: &AccountStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    claude_store: Option<&crate::claude::ClaudeStore>,
    claude_usage_service: Option<&UsageService>,
) -> Result<Vec<ProfileEntry>> {
    let saved_profiles = store.list_saved_profiles()?;
    let current_snapshot = store.get_current_snapshot().ok();
    let current_account_id = current_snapshot.as_ref().and_then(read_account_id);
    let current_access_token = current_snapshot.as_ref().and_then(read_access_token);

    let mut profiles = saved_profiles
        .into_iter()
        .map(|profile| {
            build_saved_entry(
                profile,
                &current_account_id,
                usage_service,
                force_refresh,
                refresh_account_id,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    if let Some(snapshot) = current_snapshot {
        let current_saved = current_account_id.as_ref().is_some_and(|account_id| {
            profiles
                .iter()
                .any(|profile| profile.account_id.as_deref() == Some(account_id.as_str()))
        });
        if !current_saved {
            let force_current = refresh_account_id
                .is_some_and(|account_id| current_account_id.as_deref() == Some(account_id))
                || force_refresh;
            let usage_view = usage_service.read_usage(
                current_account_id.as_deref(),
                current_access_token.as_deref(),
                force_current,
                false,
            )?;
            usage_service
                .record_usage_snapshot(current_account_id.as_deref(), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                current_account_id.as_deref(),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            profiles.push(ProfileEntry {
                kind: ProfileKind::Codex,
                saved_name: None,
                profile_name: format!(
                    "{} [UNSAVED]",
                    build_default_name(usage_view.usage.as_ref(), &snapshot)
                ),
                account_id: current_account_id.clone(),
                is_current: true,
                snapshot,
                usage_view,
                chart_data,
            });
        }
    }

    // --- Claude entries ---
    if let (Some(cs), Some(cu)) = (claude_store, claude_usage_service) {
        let claude_entries = load_claude_profiles(cs, cu, force_refresh, refresh_account_id)?;
        profiles.extend(claude_entries);
    }

    profiles.sort_by(|left, right| left.profile_name.cmp(&right.profile_name));
    Ok(profiles)
}

fn load_claude_profiles(
    store: &crate::claude::ClaudeStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
) -> Result<Vec<ProfileEntry>> {
    let saved = store.list_saved_profiles()?;
    let current_creds = store.get_current_credentials().ok();
    let current_account_id = current_creds.as_ref().map(|c| c.account_id());

    // Helper: composite key for UsageService cache
    let composite_id = |creds: &crate::claude::ClaudeCredentials| {
        format!("{}|{}", creds.account_id(), creds.subscription_type())
    };

    let mut profiles: Vec<ProfileEntry> = saved
        .into_iter()
        .map(|saved_profile| {
            let snapshot = &saved_profile.snapshot;
            let creds: Option<crate::claude::ClaudeCredentials> =
                serde_json::from_value(snapshot.clone()).ok();
            let (acct_id, access_tok, comp_id) = match &creds {
                Some(c) => (
                    Some(c.account_id()),
                    Some(c.access_token().to_string()),
                    Some(composite_id(c)),
                ),
                None => (None, None, None),
            };
            let force_this = force_refresh
                || refresh_account_id.is_some_and(|t| acct_id.as_deref() == Some(t));
            let usage_view = usage_service.read_usage(
                comp_id.as_deref(),
                access_tok.as_deref(),
                force_this,
                false,
            )?;
            usage_service
                .record_usage_snapshot(comp_id.as_deref(), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                comp_id.as_deref(),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            Ok(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: Some(saved_profile.name.clone()),
                profile_name: saved_profile.name,
                snapshot: saved_profile.snapshot,
                usage_view,
                account_id: acct_id.clone(),
                is_current: current_account_id.as_deref() == acct_id.as_deref(),
                chart_data,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // Add unsaved current Claude profile if not already in saved list
    if let Some(creds) = &current_creds {
        let acct_id = creds.account_id();
        let access_tok = creds.access_token().to_string();
        let comp_id = composite_id(creds);
        let sub_type = creds.subscription_type().to_string();
        let already_saved =
            profiles.iter().any(|p| p.account_id.as_deref() == Some(acct_id.as_str()));
        if !already_saved {
            let force_current =
                force_refresh || refresh_account_id.is_some_and(|t| t == acct_id.as_str());
            let snapshot = store
                .get_current_snapshot()
                .unwrap_or(serde_json::json!({}));
            let usage_view = usage_service.read_usage(
                Some(&comp_id),
                Some(access_tok.as_str()),
                force_current,
                false,
            )?;
            usage_service.record_usage_snapshot(Some(&comp_id), usage_view.usage.as_ref())?;
            let chart_data = build_profile_chart_data(
                Some(&comp_id),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            profiles.push(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: None,
                profile_name: format!("{} [cl-unsaved]", sub_type),
                snapshot,
                usage_view,
                account_id: Some(acct_id),
                is_current: true,
                chart_data,
            });
        }
    }

    Ok(profiles)
}

fn build_saved_entry(
    profile: SavedProfile,
    current_account_id: &Option<String>,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
) -> Result<ProfileEntry> {
    let account_id = read_account_id(&profile.snapshot);
    let access_token = read_access_token(&profile.snapshot);
    let force_this_profile = force_refresh
        || refresh_account_id.is_some_and(|target| account_id.as_deref() == Some(target));
    let usage_view = usage_service.read_usage(
        account_id.as_deref(),
        access_token.as_deref(),
        force_this_profile,
        false,
    )?;
    usage_service.record_usage_snapshot(account_id.as_deref(), usage_view.usage.as_ref())?;
    let chart_data =
        build_profile_chart_data(account_id.as_deref(), usage_view.usage.as_ref(), usage_service)?;

    Ok(ProfileEntry {
        kind: ProfileKind::Codex,
        saved_name: Some(profile.name.clone()),
        profile_name: profile.name,
        snapshot: profile.snapshot,
        usage_view,
        account_id: account_id.clone(),
        is_current: current_account_id.as_deref() == account_id.as_deref(),
        chart_data,
    })
}

fn build_profile_chart_data(
    account_id: Option<&str>,
    usage: Option<&UsageResponse>,
    usage_service: &UsageService,
) -> Result<ProfileChartData> {
    let Some(usage) = usage else {
        return Ok(ProfileChartData::empty("no usage data"));
    };
    let Some(account_id) = account_id else {
        return Ok(ProfileChartData::empty("no account id"));
    };

    let history = usage_service.profile_history(Some(account_id))?;
    let weekly_window = pick_weekly_window(usage);
    let five_hour_window = pick_five_hour_window(usage);
    let weekly_history =
        weekly_window.and_then(|window| find_matching_window(&history.weekly_windows, window));
    let seven_day_points = weekly_history
        .map(project_history_points)
        .unwrap_or_default();
    let five_hour_band = build_five_hour_band(weekly_window, five_hour_window);
    let five_hour_subframe =
        build_five_hour_subframe(weekly_window, five_hour_window, weekly_history);

    Ok(ProfileChartData {
        seven_day_points,
        five_hour_band,
        five_hour_subframe,
    })
}

fn build_five_hour_band(
    weekly_window: Option<&UsageWindow>,
    five_hour_window: Option<&UsageWindow>,
) -> OwnedFiveHourBandState {
    let Some(five_hour_window) = five_hour_window else {
        return OwnedFiveHourBandState {
            available: false,
            lower_y: None,
            upper_y: None,
            delta_seven_day_percent: None,
            delta_five_hour_percent: None,
            reason: Some("no 5h window".to_string()),
        };
    };
    let used = five_hour_window.used_percent.clamp(0.0, 100.0);
    OwnedFiveHourBandState {
        available: true,
        lower_y: Some((used - 10.0).max(0.0)),
        upper_y: Some((used + 10.0).min(100.0)),
        delta_seven_day_percent: weekly_window.map(|weekly| used - weekly.used_percent),
        delta_five_hour_percent: Some(0.0),
        reason: None,
    }
}

fn build_five_hour_subframe(
    weekly_window: Option<&UsageWindow>,
    five_hour_window: Option<&UsageWindow>,
    weekly_history: Option<&UsageWindowHistory>,
) -> OwnedFiveHourSubframeState {
    let Some(weekly_window) = weekly_window else {
        return OwnedFiveHourSubframeState {
            available: false,
            start_x: None,
            end_x: None,
            lower_y: None,
            upper_y: None,
            reason: Some("no 7d window".to_string()),
        };
    };
    let Some(five_hour_window) = five_hour_window else {
        return OwnedFiveHourSubframeState {
            available: false,
            start_x: None,
            end_x: None,
            lower_y: None,
            upper_y: None,
            reason: Some("no 5h window".to_string()),
        };
    };
    let weekly_start = weekly_window.reset_at - weekly_window.limit_window_seconds;
    let weekly_duration = weekly_window.limit_window_seconds as f64;
    let five_hour_start = five_hour_window.reset_at - five_hour_window.limit_window_seconds;
    let start_x = (((five_hour_start - weekly_start) as f64) / weekly_duration * 7.0)
        .clamp(0.0, 7.0);
    let end_x = (((five_hour_window.reset_at - weekly_start) as f64) / weekly_duration * 7.0)
        .clamp(0.0, 7.0);

    let current_7d = weekly_window.used_percent.clamp(0.0, 100.0);
    let five_hour_used = five_hour_window.used_percent.clamp(0.0, 100.0);

    // lower_y: first 7d observation at or after the 5h window start timestamp
    let lower_y = weekly_history
        .and_then(|hist| {
            hist.observations
                .iter()
                .find(|obs| obs.observed_at >= five_hour_start)
                .map(|obs| obs.used_percent.clamp(0.0, 100.0))
        })
        .unwrap_or_else(|| (current_7d - five_hour_used).max(0.0));

    // upper_y: project 5h usage to 100% using the observed 7d growth rate
    // Formula: lower_y + (7d_delta / 5h_used%) * 100
    let seven_day_delta = (current_7d - lower_y).max(0.0);
    let upper_y = if five_hour_used > 0.0 {
        (lower_y + (seven_day_delta / five_hour_used) * 100.0).clamp(lower_y, 100.0)
    } else {
        (lower_y + 100.0).clamp(lower_y, 100.0)
    };

    OwnedFiveHourSubframeState {
        available: true,
        start_x: Some(start_x),
        end_x: Some(end_x.max(start_x)),
        lower_y: Some(lower_y),
        upper_y: Some(upper_y),
        reason: None,
    }
}

fn find_matching_window<'a>(
    windows: &'a [UsageWindowHistory],
    window: &UsageWindow,
) -> Option<&'a UsageWindowHistory> {
    let start_at = window.reset_at - window.limit_window_seconds;
    windows.iter().find(|candidate| {
        candidate.limit_window_seconds == window.limit_window_seconds
            && candidate.start_at == start_at
            && candidate.end_at == window.reset_at
    })
}

fn project_history_points(window: &UsageWindowHistory) -> Vec<ChartPoint> {
    let total = (window.end_at - window.start_at) as f64;
    if total <= 0.0 {
        return Vec::new();
    }

    let mut points = window
        .observations
        .iter()
        .map(|observation| ChartPoint {
            x: (((observation.observed_at - window.start_at) as f64 / total) * 7.0)
                .clamp(0.0, 7.0),
            y: observation.used_percent.clamp(0.0, 100.0),
        })
        .collect::<Vec<_>>();
    points.sort_by(|left, right| left.x.total_cmp(&right.x));
    points.dedup_by(|left, right| {
        (left.x - right.x).abs() < f64::EPSILON && (left.y - right.y).abs() < f64::EPSILON
    });
    points
}

fn read_account_id(snapshot: &Value) -> Option<String> {
    snapshot
        .get("tokens")
        .and_then(|value| value.get("account_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn read_access_token(snapshot: &Value) -> Option<String> {
    snapshot
        .get("tokens")
        .and_then(|value| value.get("access_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn build_default_name(usage: Option<&UsageResponse>, snapshot: &Value) -> String {
    let email_part = sanitize_name_part(usage.and_then(|value| value.email.as_deref()));
    let plan_part = sanitize_name_part(usage.and_then(|value| value.plan_type.as_deref()));
    let account_part = sanitize_name_part(read_account_id(snapshot).as_deref());

    match (email_part, plan_part, account_part) {
        (Some(email), Some(plan), _) => format!("{email}-{plan}"),
        (Some(email), None, _) => email,
        (None, _, Some(account)) => {
            format!("profile-{}", &account.chars().take(8).collect::<String>())
        }
        _ => "profile".to_string(),
    }
}

fn sanitize_name_part(input: Option<&str>) -> Option<String> {
    input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase().replace('@', "-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_window_history_projects_real_observation_points() {
        let history = UsageWindowHistory {
            limit_window_seconds: 604_800,
            start_at: 100,
            end_at: 604_900,
            observations: vec![
                crate::usage::UsageObservation {
                    observed_at: 100,
                    used_percent: 12.0,
                },
                crate::usage::UsageObservation {
                    observed_at: 302_500,
                    used_percent: 44.0,
                },
                crate::usage::UsageObservation {
                    observed_at: 604_900,
                    used_percent: 70.0,
                },
            ],
        };

        let points = project_history_points(&history);
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], ChartPoint { x: 0.0, y: 12.0 });
        assert!(points[1].x > 3.4 && points[1].x < 3.6);
        assert_eq!(points[2], ChartPoint { x: 7.0, y: 70.0 });
    }

    #[test]
    fn five_hour_subframe_is_bounded_inside_weekly_chart_space() {
        // weekly: 0..604800, currently at 60%
        // five_hour: 522000..540000 (start = reset_at - limit = 540000 - 18000), currently at 30%
        let weekly = UsageWindow {
            used_percent: 60.0,
            limit_window_seconds: 604_800,
            reset_after_seconds: 86_400,
            reset_at: 604_800,
        };
        let five_hour = UsageWindow {
            used_percent: 30.0,
            limit_window_seconds: 18_000,
            reset_after_seconds: 1_800,
            reset_at: 540_000,
        };

        // With history: first observation at or after five_hour_start (522000) is at 522000, used=45%
        // lower_y = 45.0 (data lookup)
        // 7d_delta = 60 - 45 = 15, 5h_used = 30
        // upper_y = 45 + (15/30)*100 = 45 + 50 = 95
        let history = crate::usage::UsageWindowHistory {
            limit_window_seconds: 604_800,
            start_at: 0,
            end_at: 604_800,
            observations: vec![
                crate::usage::UsageObservation {
                    observed_at: 300_000,
                    used_percent: 30.0,
                },
                crate::usage::UsageObservation {
                    observed_at: 522_000,
                    used_percent: 45.0,
                },
                crate::usage::UsageObservation {
                    observed_at: 604_800,
                    used_percent: 60.0,
                },
            ],
        };
        let subframe = build_five_hour_subframe(Some(&weekly), Some(&five_hour), Some(&history));
        assert!(subframe.available);
        assert!(subframe.start_x.unwrap() < subframe.end_x.unwrap());
        assert!(subframe.end_x.unwrap() <= 7.0);
        assert_eq!(subframe.lower_y, Some(45.0));
        assert_eq!(subframe.upper_y, Some(95.0));

        // Without history: fallback to weekly_used - 5h_used = 30, upper = 30 + (30/30)*100 = 130 -> 100
        let subframe_no_hist = build_five_hour_subframe(Some(&weekly), Some(&five_hour), None);
        assert_eq!(subframe_no_hist.lower_y, Some(30.0));
        assert_eq!(subframe_no_hist.upper_y, Some(100.0));
    }

    #[test]
    fn claude_profile_chart_uses_recorded_history() {
        use crate::usage::{UsageRateLimit, UsageResponse, UsageWindow};
        use std::path::PathBuf;

        let cache_path = PathBuf::from("dummy_cache.json");
        let history_path =
            std::env::temp_dir().join(format!("test_claude_history_{}.json", std::process::id()));
        let usage_service = UsageService::new(cache_path, history_path.clone(), 300);

        let now = 1_700_000_000;
        let usage = UsageResponse {
            email: None,
            plan_type: Some("pro".to_string()),
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 50.0,
                    limit_window_seconds: 18_000,
                    reset_at: now + 3_600,
                    reset_after_seconds: 3_600,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 20.0,
                    limit_window_seconds: 604_800,
                    reset_at: now + 300_000,
                    reset_after_seconds: 300_000,
                }),
            }),
        };

        let account_id = "claude-test|pro";
        let service = usage_service.with_now_seconds(now);
        service.record_usage_snapshot(Some(account_id), Some(&usage)).unwrap();

        let chart_data = build_profile_chart_data(Some(account_id), Some(&usage), &service).unwrap();

        assert!(
            !chart_data.seven_day_points.is_empty(),
            "Claude chart data should have points"
        );
        assert!(
            chart_data.five_hour_band.available,
            "Claude 5h band should be available"
        );

        let _ = std::fs::remove_file(history_path);
    }
}
