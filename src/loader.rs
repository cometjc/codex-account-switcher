use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::app_data::{
    OwnedFiveHourBandState, OwnedFiveHourSubframeState, ProfileChartData, ProfileEntry, ProfileKind,
};
use crate::render::ChartPoint;
use crate::store::{AccountStore, SavedProfile};
use crate::usage::{
    pick_five_hour_window, pick_weekly_window, ProfileUsageHistory, UsageObservation, UsageReadResult,
    UsageResponse, UsageService, UsageWindow, UsageWindowHistory,
};

pub struct ProfileLoadReport {
    pub profiles: Vec<ProfileEntry>,
    pub refresh_errors: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn load_profiles(
    store: &AccountStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    cache_only: bool,
    claude_store: Option<&crate::claude::ClaudeStore>,
    claude_usage_service: Option<&UsageService>,
    copilot_usage_service: Option<&UsageService>,
) -> Result<Vec<ProfileEntry>> {
    Ok(load_profiles_with_report(
        store,
        usage_service,
        force_refresh,
        refresh_account_id,
        cache_only,
        claude_store,
        claude_usage_service,
        copilot_usage_service,
    )?
    .profiles)
}

#[allow(clippy::too_many_arguments)]
pub fn load_profiles_with_report(
    store: &AccountStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    cache_only: bool,
    claude_store: Option<&crate::claude::ClaudeStore>,
    claude_usage_service: Option<&UsageService>,
    copilot_usage_service: Option<&UsageService>,
) -> Result<ProfileLoadReport> {
    let saved_profiles = store.list_saved_profiles()?;
    let current_snapshot = store.get_current_snapshot().ok();
    let current_account_id = current_snapshot.as_ref().and_then(read_account_id);
    let current_access_token = current_snapshot.as_ref().and_then(read_access_token);
    // Name-based fallback: survives broken symlinks and renames (current file is updated by rename_account)
    let current_codex_name = store.get_current_account_name().ok().flatten();
    let mut refresh_errors = Vec::new();

    let mut profiles = saved_profiles
        .into_iter()
        .map(|profile| {
            build_saved_entry(
                store,
                profile,
                &current_account_id,
                &current_codex_name,
                usage_service,
                force_refresh,
                refresh_account_id,
                cache_only,
                &mut refresh_errors,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    if let Some(snapshot) = current_snapshot {
        let current_saved = profiles.iter().any(|profile| {
            current_account_id
                .as_deref()
                .is_some_and(|account_id| profile.account_id.as_deref() == Some(account_id))
                || current_codex_name
                    .as_deref()
                    .is_some_and(|name| profile.saved_name.as_deref() == Some(name))
        });
        if !current_saved {
            let force_current = refresh_account_id
                .is_some_and(|account_id| current_account_id.as_deref() == Some(account_id))
                || force_refresh;
            let (usage_view, error) = read_usage_for_profile(
                "Codex",
                "current",
                usage_service,
                Some(store.paths().auth_path()),
                current_account_id.as_deref(),
                current_access_token.as_deref(),
                force_current,
                cache_only,
            )?;
            push_refresh_error(&mut refresh_errors, error);
            usage_service
                .record_usage_snapshot(current_account_id.as_deref(), &usage_view)?;
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
        let (claude_entries, claude_errors) =
            load_claude_profiles(cs, cu, force_refresh, refresh_account_id, cache_only)?;
        profiles.extend(claude_entries);
        refresh_errors.extend(claude_errors);
    }

    // --- Copilot entries ---
    if let Some(cu) = copilot_usage_service {
        if let Some(creds) = crate::copilot::CopilotCredentials::detect() {
            let account_id = creds.account_id();
            let force = force_refresh
                || refresh_account_id.is_some_and(|id| id == account_id.as_str());
            let (usage_view, error) = read_usage_for_profile(
                "Copilot",
                &creds.login,
                cu,
                None,
                Some(account_id.as_str()),
                Some(creds.oauth_token.as_str()),
                force,
                cache_only,
            )?;
            push_refresh_error(&mut refresh_errors, error);
            cu.record_usage_snapshot(Some(account_id.as_str()), &usage_view)?;
            let chart_data = build_profile_chart_data(Some(account_id.as_str()), usage_view.usage.as_ref(), cu)?;
            profiles.push(ProfileEntry {
                kind: ProfileKind::Copilot,
                saved_name: Some(creds.login.clone()),
                profile_name: creds.login.clone(),
                account_id: Some(account_id),
                is_current: true,  // single detected account is always the active one
                snapshot: serde_json::json!({}),
                usage_view,
                chart_data,
            });
        }
    }

    profiles.sort_by(|left, right| {
        left.kind.as_str().cmp(right.kind.as_str())
            .then_with(|| left.profile_name.cmp(&right.profile_name))
    });
    Ok(ProfileLoadReport {
        profiles,
        refresh_errors,
    })
}

fn load_claude_profiles(
    store: &crate::claude::ClaudeStore,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    cache_only: bool,
) -> Result<(Vec<ProfileEntry>, Vec<String>)> {
    let saved = store.list_saved_profiles()?;
    let current_creds = store.get_current_credentials().ok();
    let current_account_id = current_creds.as_ref().map(|c| c.account_id());
    let current_name = store.get_current_account_name().ok().flatten();
    let mut refresh_errors = Vec::new();

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
            let use_current_credentials = current_name.as_deref() == Some(saved_profile.name.as_str())
                || (current_account_id.is_some() && current_account_id.as_deref() == acct_id.as_deref());
            let effective_acct_id = if use_current_credentials {
                current_creds.as_ref().map(|creds| creds.account_id())
            } else {
                acct_id.clone()
            };
            let effective_access_tok = if use_current_credentials {
                current_creds.as_ref().map(|creds| creds.access_token().to_string())
            } else {
                access_tok.clone()
            };
            let effective_comp_id = if use_current_credentials {
                current_creds.as_ref().map(composite_id)
            } else {
                comp_id.clone()
            };
            if use_current_credentials {
                usage_service.merge_profile_history_aliases(
                    effective_comp_id.as_deref(),
                    comp_id.as_deref().into_iter(),
                )?;
            }
            let force_this = force_refresh
                || refresh_account_id.is_some_and(|t| effective_acct_id.as_deref() == Some(t));
            let (usage_view, error) = read_usage_for_profile(
                "Claude",
                &saved_profile.name,
                usage_service,
                None,
                effective_comp_id.as_deref(),
                effective_access_tok.as_deref(),
                force_this,
                cache_only,
            )?;
            push_refresh_error(&mut refresh_errors, error);
            if use_current_credentials {
                let _ = store.update_account(&saved_profile.name);
            }
            usage_service
                .record_usage_snapshot(effective_comp_id.as_deref(), &usage_view)?;
            let chart_data = build_profile_chart_data(
                effective_comp_id.as_deref(),
                usage_view.usage.as_ref(),
                usage_service,
            )?;
            let effective_snapshot = if use_current_credentials {
                store
                    .get_current_snapshot()
                    .unwrap_or_else(|_| saved_profile.snapshot.clone())
            } else {
                saved_profile.snapshot.clone()
            };
            let is_current = use_current_credentials;
            Ok(ProfileEntry {
                kind: ProfileKind::Claude,
                saved_name: Some(saved_profile.name.clone()),
                profile_name: saved_profile.name,
                snapshot: effective_snapshot,
                usage_view,
                account_id: effective_acct_id.clone(),
                is_current,
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

        let matched_by_acct_or_name = profiles.iter().any(|p| {
            p.account_id.as_deref() == Some(acct_id.as_str())
                || current_name.as_deref().is_some_and(|n| p.saved_name.as_deref() == Some(n))
        });

        // Fallback: if exactly one saved profile shares the same subscriptionType,
        // treat it as the same account (e.g. after a full re-auth that changes refresh token).
        let sub_type_match_name: Option<String> = if !matched_by_acct_or_name {
            let matches: Vec<_> = profiles
                .iter()
                .filter(|p| {
                    serde_json::from_value::<crate::claude::ClaudeCredentials>(p.snapshot.clone())
                        .ok()
                        .map(|c| c.subscription_type() == sub_type.as_str())
                        .unwrap_or(false)
                })
                .collect();
            if matches.len() == 1 { matches[0].saved_name.clone() } else { None }
        } else {
            None
        };

        if let Some(ref matched_name) = sub_type_match_name {
            // Auto-merge: update saved file with current credentials and record current name
            let _ = store.update_account(matched_name);
            let _ = store.set_current_name(matched_name);
            for p in profiles.iter_mut() {
                if p.saved_name.as_deref() == Some(matched_name.as_str()) {
                    let old_comp_id = serde_json::from_value::<crate::claude::ClaudeCredentials>(p.snapshot.clone())
                        .ok()
                        .map(|c| composite_id(&c));
                    let _ = usage_service.merge_profile_history_aliases(
                        Some(&comp_id),
                        old_comp_id.as_deref().into_iter(),
                    );
                    p.is_current = true;
                    p.account_id = Some(acct_id.clone());
                }
            }
        }

        let already_saved = matched_by_acct_or_name || sub_type_match_name.is_some();
        if !already_saved {
            let force_current =
                force_refresh || refresh_account_id.is_some_and(|t| t == acct_id.as_str());
            let snapshot = store
                .get_current_snapshot()
                .unwrap_or(serde_json::json!({}));
            let (usage_view, error) = read_usage_for_profile(
                "Claude",
                "current",
                usage_service,
                None,
                Some(&comp_id),
                Some(access_tok.as_str()),
                force_current,
                cache_only,
            )?;
            push_refresh_error(&mut refresh_errors, error);
            usage_service.record_usage_snapshot(Some(&comp_id), &usage_view)?;
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

    Ok((profiles, refresh_errors))
}

#[allow(clippy::too_many_arguments)]
fn build_saved_entry(
    store: &AccountStore,
    profile: SavedProfile,
    current_account_id: &Option<String>,
    current_name: &Option<String>,
    usage_service: &UsageService,
    force_refresh: bool,
    refresh_account_id: Option<&str>,
    cache_only: bool,
    refresh_errors: &mut Vec<String>,
) -> Result<ProfileEntry> {
    let saved_account_id = read_account_id(&profile.snapshot);
    let access_token = read_access_token(&profile.snapshot);
    let matches_id = current_account_id.is_some()
        && current_account_id.as_deref() == saved_account_id.as_deref();
    let matches_name = current_name.as_deref()
        .is_some_and(|n| n == profile.name.as_str());
    let use_current_auth = matches_id || matches_name;
    let effective_account_id = if use_current_auth {
        current_account_id.clone().or_else(|| saved_account_id.clone())
    } else {
        saved_account_id.clone()
    };
    let force_this_profile = force_refresh
        || refresh_account_id.is_some_and(|target| effective_account_id.as_deref() == Some(target));
    let (usage_view, error) = read_usage_for_profile(
        "Codex",
        &profile.name,
        usage_service,
        Some(if use_current_auth {
            store.paths().auth_path()
        } else {
            profile.file_path.as_path()
        }),
        effective_account_id.as_deref(),
        access_token.as_deref(),
        force_this_profile,
        cache_only,
    )?;
    push_refresh_error(refresh_errors, error);
    if use_current_auth {
        let _ = store.update_account(&profile.name);
    }
    let snapshot = if use_current_auth {
        store
            .get_current_snapshot()
            .unwrap_or_else(|_| profile.snapshot.clone())
    } else {
        profile.snapshot.clone()
    };
    usage_service.record_usage_snapshot(effective_account_id.as_deref(), &usage_view)?;
    let chart_data =
        build_profile_chart_data(effective_account_id.as_deref(), usage_view.usage.as_ref(), usage_service)?;

    // Match by account_id (from auth.json content) OR by name (from ~/.codex/current file).
    // Name-based match survives broken symlinks and renames; account_id match handles the
    // case where the current file is absent but auth.json is readable.
    Ok(ProfileEntry {
        kind: ProfileKind::Codex,
        saved_name: Some(profile.name.clone()),
        profile_name: profile.name,
        snapshot,
        usage_view,
        account_id: effective_account_id,
        is_current: matches_id || matches_name,
        chart_data,
    })
}

#[allow(clippy::too_many_arguments)]
fn read_usage_for_profile(
    service_label: &str,
    profile_label: &str,
    usage_service: &UsageService,
    codex_auth_path: Option<&Path>,
    account_id: Option<&str>,
    access_token: Option<&str>,
    force_refresh: bool,
    cache_only: bool,
) -> Result<(UsageReadResult, Option<String>)> {
    if service_label == "Codex" {
        let auth_path = codex_auth_path.context("missing Codex auth path")?;
        if !force_refresh {
            return Ok((usage_service.read_codex_usage(auth_path, false, cache_only)?, None));
        }

        return match usage_service.read_codex_usage(auth_path, true, false) {
            Ok(view) => Ok((view, None)),
            Err(error) => Ok((
                usage_service.read_codex_usage(auth_path, false, false)?,
                Some(format!("{service_label} {profile_label}: {error:#}")),
            )),
        };
    }

    if !force_refresh {
        return Ok((
            usage_service.read_usage(account_id, access_token, false, cache_only)?,
            None,
        ));
    }

    match usage_service.read_usage(account_id, access_token, true, false) {
        Ok(view) => Ok((view, None)),
        Err(error) => Ok((
            usage_service.read_usage(account_id, access_token, false, false)?,
            Some(format!("{service_label} {profile_label}: {error:#}")),
        )),
    }
}

fn push_refresh_error(errors: &mut Vec<String>, error: Option<String>) {
    if let Some(error) = error {
        errors.push(error);
    }
}

fn build_profile_chart_data(
    account_id: Option<&str>,
    usage: Option<&UsageResponse>,
    usage_service: &UsageService,
) -> Result<ProfileChartData> {
    let Some(account_id) = account_id else {
        return Ok(ProfileChartData::empty("no account id"));
    };

    let history = usage_service.profile_history(Some(account_id))?;
    let weekly_window_live = usage.and_then(pick_weekly_window);
    let five_hour_window_live = usage.and_then(pick_five_hour_window);
    let weekly_window_fallback = history.weekly_reset_at.map(|reset_at| UsageWindow {
        used_percent: latest_weekly_used_percent(&history),
        limit_window_seconds: history.weekly_window_seconds,
        reset_after_seconds: 0,
        reset_at,
    });
    let five_hour_window_fallback = history.five_hour_reset_at.map(|reset_at| UsageWindow {
        used_percent: latest_five_hour_used_percent(&history),
        limit_window_seconds: history.five_hour_window_seconds,
        reset_after_seconds: 0,
        reset_at,
    });
    let weekly_window = weekly_window_live.or(weekly_window_fallback.as_ref());
    let five_hour_window = five_hour_window_live.or(five_hour_window_fallback.as_ref());

    let seven_day_points = weekly_window
        .map(|window| {
            project_weekly_points_from_profile_observations(window, &history.observations)
        })
        .unwrap_or_default();
    let weekly_history = weekly_window
        .map(|window| build_weekly_history_from_profile_observations(window, &history.observations));
    let five_hour_histories = build_five_hour_histories_from_profile_observations(&history);
    let weekly_histories = build_weekly_histories_from_profile_observations(&history);
    let five_hour_band = build_five_hour_band(weekly_window, five_hour_window);
    let five_hour_subframe = build_five_hour_subframe(
        weekly_window,
        five_hour_window,
        weekly_history.as_ref(),
        &five_hour_histories,
        &weekly_histories,
    );

    Ok(ProfileChartData {
        seven_day_points,
        five_hour_band,
        five_hour_subframe,
    })
}

fn latest_weekly_used_percent(history: &ProfileUsageHistory) -> f64 {
    history
        .observations
        .iter()
        .rev()
        .find_map(|obs| obs.weekly_used_percent)
        .unwrap_or(0.0)
}

fn latest_five_hour_used_percent(history: &ProfileUsageHistory) -> f64 {
    history
        .observations
        .iter()
        .rev()
        .find_map(|obs| obs.five_hour_used_percent)
        .unwrap_or(0.0)
}

fn project_weekly_points_from_profile_observations(
    window: &UsageWindow,
    observations: &[crate::usage::ProfileUsageObservation],
) -> Vec<ChartPoint> {
    let now_seconds = current_unix_seconds();
    let end_at = window.reset_at.min(now_seconds);
    let start_at = end_at - window.limit_window_seconds;
    let total = (end_at - start_at) as f64;
    if total <= 0.0 {
        return Vec::new();
    }
    let mut points = observations
        .iter()
        .filter_map(|obs| {
            let y = obs.weekly_used_percent?;
            if obs.observed_at_local <= start_at || obs.observed_at_local > end_at {
                return None;
            }
            Some(ChartPoint {
                x: (((obs.observed_at_local - start_at) as f64 / total) * 7.0).clamp(0.0, 7.0),
                y: y.clamp(0.0, 100.0),
            })
        })
        .collect::<Vec<_>>();
    points.sort_by(|left, right| left.x.total_cmp(&right.x));
    points.dedup_by(|left, right| {
        (left.x - right.x).abs() < f64::EPSILON && (left.y - right.y).abs() < f64::EPSILON
    });
    points
}

fn build_weekly_history_from_profile_observations(
    window: &UsageWindow,
    observations: &[crate::usage::ProfileUsageObservation],
) -> UsageWindowHistory {
    let now_seconds = current_unix_seconds();
    let end_at = window.reset_at.min(now_seconds);
    let start_at = end_at - window.limit_window_seconds;
    let mut obs = observations
        .iter()
        .filter_map(|obs| {
            let y = obs.weekly_used_percent?;
            if obs.observed_at_local <= start_at || obs.observed_at_local > end_at {
                return None;
            }
            Some(UsageObservation {
                observed_at: obs.observed_at_local,
                used_percent: y.clamp(0.0, 100.0),
            })
        })
        .collect::<Vec<_>>();
    obs.sort_by_key(|o| o.observed_at);
    UsageWindowHistory {
        limit_window_seconds: window.limit_window_seconds,
        start_at,
        end_at,
        observations: obs,
    }
}

fn current_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn build_weekly_histories_from_profile_observations(
    history: &ProfileUsageHistory,
) -> Vec<UsageWindowHistory> {
    let Some(reset_at) = history.weekly_reset_at else {
        return Vec::new();
    };
    let window = UsageWindow {
        used_percent: 0.0,
        limit_window_seconds: history.weekly_window_seconds,
        reset_after_seconds: 0,
        reset_at,
    };
    vec![build_weekly_history_from_profile_observations(
        &window,
        &history.observations,
    )]
}

fn build_five_hour_histories_from_profile_observations(
    history: &ProfileUsageHistory,
) -> Vec<UsageWindowHistory> {
    let Some(reset_at) = history.five_hour_reset_at else {
        return Vec::new();
    };
    let start_at = reset_at - history.five_hour_window_seconds;
    let mut obs = history
        .observations
        .iter()
        .filter_map(|obs| {
            let y = obs.five_hour_used_percent?;
            if obs.observed_at_local < start_at || obs.observed_at_local > reset_at {
                return None;
            }
            Some(UsageObservation {
                observed_at: obs.observed_at_local,
                used_percent: y.clamp(0.0, 100.0),
            })
        })
        .collect::<Vec<_>>();
    obs.sort_by_key(|o| o.observed_at);
    vec![UsageWindowHistory {
        limit_window_seconds: history.five_hour_window_seconds,
        start_at,
        end_at: reset_at,
        observations: obs,
    }]
}

fn build_five_hour_band(
    weekly_window: Option<&UsageWindow>,
    five_hour_window: Option<&UsageWindow>,
) -> OwnedFiveHourBandState {
    let Some(five_hour_window) = five_hour_window else {
        return OwnedFiveHourBandState {
            available: false,
            used_percent: None,
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
        used_percent: Some(used),
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
    all_five_hour_windows: &[UsageWindowHistory],
    all_weekly_windows: &[UsageWindowHistory],
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
    let mut start_x = (((five_hour_start - weekly_start) as f64) / weekly_duration * 7.0)
        .clamp(0.0, 7.0);
    let mut end_x = (((five_hour_window.reset_at - weekly_start) as f64) / weekly_duration * 7.0)
        .clamp(0.0, 7.0);
    if start_x > end_x {
        std::mem::swap(&mut start_x, &mut end_x);
    }
    // Chart x uses the same weekly axis as `project_history_points`:
    //   point_x = (observed_at - weekly_start) / weekly_duration * 7
    //   end_x   = (five_hour.reset_at - weekly_start) / weekly_duration * 7
    // For consistent inputs, last observation should satisfy observed_at <= five_hour.reset_at
    // (next 5h reset is in the future), so the last point is not to the right of end_x.
    // If observed_at > reset_at (e.g. stale cached 5h vs newer weekly observations), that is a
    // data/refresh issue — do not patch here.

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

    // upper_y: project the max observed 7d/5h rate across all windows (current + historical)
    // to 100% 5h usage. Using the maximum ensures the band covers the worst-case observed ratio.
    // When five_hour_used is 0, fall back to the historical max rate (thin line, min 1%).
    let seven_day_delta = (current_7d - lower_y).max(0.0);
    const MIN_BAND_HEIGHT: f64 = 1.0;
    let upper_y = if five_hour_used > 0.0 {
        let current_rate = seven_day_delta / five_hour_used;
        let rate = compute_max_historical_rate(all_five_hour_windows, all_weekly_windows)
            .map(|hist_max| hist_max.max(current_rate))
            .unwrap_or(current_rate);
        (lower_y + rate * 100.0).clamp(lower_y, 100.0)
    } else {
        let fallback_band = compute_max_historical_rate(all_five_hour_windows, all_weekly_windows)
            .map(|r| (r * 10.0).max(MIN_BAND_HEIGHT))
            .unwrap_or(MIN_BAND_HEIGHT);
        (lower_y + fallback_band).clamp(lower_y, 100.0)
    };
    // Invariant: upper_y >= current_7d. The current 7d value was already reached within
    // this 5h window, so the projected ceiling must not fall below the observed fact.
    let upper_y = upper_y.max(current_7d);

    OwnedFiveHourSubframeState {
        available: true,
        start_x: Some(start_x),
        end_x: Some(end_x.max(start_x)),
        lower_y: Some(lower_y),
        upper_y: Some(upper_y),
        reason: None,
    }
}

/// Returns the maximum `7d_delta / 5h_delta` rate across past 5h windows with usage > 0.
/// Returns None if no valid windows found.
fn compute_max_historical_rate(
    five_hour_windows: &[UsageWindowHistory],
    weekly_windows: &[UsageWindowHistory],
) -> Option<f64> {
    five_hour_windows
        .iter()
        .filter_map(|fh_win| {
            let five_hour_delta = fh_win.observations.last()?.used_percent;
            if five_hour_delta <= 0.0 {
                return None;
            }
            let seven_day_delta = seven_day_delta_for_window(fh_win, weekly_windows);
            if seven_day_delta < 0.0 {
                return None;
            }
            Some(seven_day_delta / five_hour_delta)
        })
        .reduce(f64::max)
}

/// Computes 7d usage growth during a 5h window's time span.
/// Handles the case where the 7d window resets to 0 mid-span.
/// Returns -1.0 if the required weekly observations are not available.
fn seven_day_delta_for_window(
    fh_win: &UsageWindowHistory,
    weekly_windows: &[UsageWindowHistory],
) -> f64 {
    let t_start = fh_win.start_at;
    let t_end = fh_win.end_at;

    // Prefer the window with the most observations when multiple windows overlap the same
    // timestamp — duplicate windows with ±1s jitter are common and the denser one is more accurate.
    let win_at_start = weekly_windows
        .iter()
        .filter(|w| w.start_at <= t_start && t_start < w.end_at)
        .max_by_key(|w| w.observations.len());
    let win_at_end = weekly_windows
        .iter()
        .filter(|w| w.start_at <= t_end && t_end <= w.end_at)
        .max_by_key(|w| w.observations.len());

    match (win_at_start, win_at_end) {
        (Some(w1), Some(w2)) if std::ptr::eq(w1, w2) => {
            // Same 7d cycle: simple interpolated delta
            let y_start = interp_weekly_at(w1, t_start);
            let y_end = interp_weekly_at(w1, t_end);
            (y_end - y_start).max(0.0)
        }
        (Some(w1), Some(w2)) => {
            // 7d window reset during this 5h window: sum growth before and after reset
            let y_start = interp_weekly_at(w1, t_start);
            let y_before_reset = w1.observations.last().map_or(0.0, |o| o.used_percent);
            let delta_before = (y_before_reset - y_start).max(0.0);
            let y_after_reset = interp_weekly_at(w2, t_end);
            delta_before + y_after_reset
        }
        _ => -1.0,
    }
}

/// Linearly interpolates `used_percent` at timestamp `t` within a window's observations.
fn interp_weekly_at(win: &UsageWindowHistory, t: i64) -> f64 {
    let obs = &win.observations;
    if obs.is_empty() {
        return 0.0;
    }
    let pos = obs.partition_point(|o| o.observed_at <= t);
    if pos == 0 {
        return obs[0].used_percent;
    }
    if pos >= obs.len() {
        return obs.last().unwrap().used_percent;
    }
    let before = &obs[pos - 1];
    let after = &obs[pos];
    let span = (after.observed_at - before.observed_at) as f64;
    if span <= 0.0 {
        return before.used_percent;
    }
    let ratio = (t - before.observed_at) as f64 / span;
    before.used_percent + (after.used_percent - before.used_percent) * ratio
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn project_weekly_points_for_window_range(
    window: &UsageWindow,
    weekly_windows: &[UsageWindowHistory],
) -> Vec<ChartPoint> {
    let start_at = window.reset_at - window.limit_window_seconds;
    let end_at = window.reset_at;
    let total = (end_at - start_at) as f64;
    if total <= 0.0 {
        return Vec::new();
    }

    let mut points = weekly_windows
        .iter()
        .filter(|w| w.limit_window_seconds == window.limit_window_seconds)
        .flat_map(|w| w.observations.iter())
        .filter(|obs| obs.observed_at >= start_at && obs.observed_at <= end_at)
        .map(|obs| ChartPoint {
            x: (((obs.observed_at - start_at) as f64 / total) * 7.0).clamp(0.0, 7.0),
            y: obs.used_percent.clamp(0.0, 100.0),
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
    use crate::claude::{ClaudePaths, ClaudeStore};
    use crate::paths::AppPaths;
    use crate::store::{AccountStore, StorePlatform};
    use crate::usage::{ProfileUsageHistory, UsageCache, UsageHistoryCache, UsageObservation, UsageRateLimit, UsageSource, UsageWindowHistory};
    use std::fs;
    use std::path::PathBuf;

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
        let expected_api_end = 540_000f64 / 604_800.0 * 7.0;
        let subframe = build_five_hour_subframe(Some(&weekly), Some(&five_hour), Some(&history), &[], &[]);
        assert!(subframe.available);
        assert!(
            (subframe.end_x.unwrap() - expected_api_end).abs() < 1e-9,
            "end_x follows five_hour.reset_at on the weekly axis"
        );
        let last_point_x = project_history_points(&history).last().unwrap().x;
        assert!(
            last_point_x > subframe.end_x.unwrap(),
            "fixture: last obs at week end (x=7) but 5h reset_at earlier — inconsistent (stale 5h vs obs)"
        );
        assert!(subframe.start_x.unwrap() < subframe.end_x.unwrap());
        assert_eq!(subframe.lower_y, Some(45.0));
        assert_eq!(subframe.upper_y, Some(95.0));

        // Consistent API: 5h window ends at or after the last observation time on the same week.
        let five_hour_aligned = UsageWindow {
            used_percent: 30.0,
            limit_window_seconds: 18_000,
            reset_after_seconds: 1_800,
            reset_at: 604_800,
        };
        let subframe_ok = build_five_hour_subframe(
            Some(&weekly),
            Some(&five_hour_aligned),
            Some(&history),
            &[],
            &[],
        );
        assert_eq!(subframe_ok.end_x, Some(7.0));
        assert_eq!(project_history_points(&history).last().unwrap().x, 7.0);

        // Without history: fallback to weekly_used - 5h_used = 30, upper = 30 + (30/30)*100 = 130 -> 100
        let subframe_no_hist = build_five_hour_subframe(Some(&weekly), Some(&five_hour), None, &[], &[]);
        assert_eq!(subframe_no_hist.lower_y, Some(30.0));
        assert_eq!(subframe_no_hist.upper_y, Some(100.0));
    }

    #[test]
    fn claude_profile_chart_uses_recorded_history() {
        use crate::usage::{UsageRateLimit, UsageResponse, UsageWindow};

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
        let read = crate::usage::UsageReadResult {
            usage: Some(usage.clone()),
            source: crate::usage::UsageSource::Api,
            fetched_at: Some(now),
            stale: false,
        };
        service.record_usage_snapshot(Some(account_id), &read).unwrap();

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

    #[test]
    fn chart_uses_history_when_live_usage_temporarily_missing() {
        use crate::usage::{UsageRateLimit, UsageResponse, UsageWindow};

        let cache_path = PathBuf::from("dummy_cache_history_fallback.json");
        let history_path =
            std::env::temp_dir().join(format!("test_history_fallback_{}.json", std::process::id()));
        let usage_service = UsageService::new(cache_path, history_path.clone(), 300);
        let now = 1_700_000_000;
        let usage = UsageResponse {
            email: None,
            plan_type: Some("pro".to_string()),
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 40.0,
                    limit_window_seconds: 18_000,
                    reset_at: now + 3_600,
                    reset_after_seconds: 3_600,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 18.0,
                    limit_window_seconds: 604_800,
                    reset_at: now + 300_000,
                    reset_after_seconds: 300_000,
                }),
            }),
        };
        let account_id = "claude-fallback|pro";
        usage_service
            .clone()
            .with_now_seconds(now)
            .record_usage_snapshot(
                Some(account_id),
                &crate::usage::UsageReadResult {
                    usage: Some(usage),
                    source: crate::usage::UsageSource::Api,
                    fetched_at: Some(now),
                    stale: false,
                },
            )
            .unwrap();

        let chart_data = build_profile_chart_data(Some(account_id), None, &usage_service).unwrap();
        assert!(
            !chart_data.seven_day_points.is_empty(),
            "history fallback should keep chart visible when live usage is missing"
        );

        let _ = std::fs::remove_file(history_path);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_usage(plan: &str) -> UsageResponse {
        UsageResponse {
            email: Some("demo@example.com".to_string()),
            plan_type: Some(plan.to_string()),
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 42.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 3_600,
                    reset_at: 1_700_000_000,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 18.0,
                    limit_window_seconds: 18_000,
                    reset_after_seconds: 900,
                    reset_at: 1_699_990_000,
                }),
            }),
        }
    }

    fn write_usage_cache(path: &std::path::Path, account_id: &str, usage: UsageResponse) {
        let cache = UsageCache::from_entries([(account_id.to_string(), 1_700_000_000, usage)]);
        fs::write(path, format!("{}\n", serde_json::to_string_pretty(&cache).unwrap())).unwrap();
    }

    #[test]
    fn codex_saved_profile_uses_current_snapshot_and_syncs_saved_file() {
        let base = unique_temp_dir("loader-codex-current-sync");
        let codex_paths = AppPaths::from_codex_dir(base.join("codex"));
        let store = AccountStore::new(codex_paths.clone(), StorePlatform::Copy);
        fs::create_dir_all(codex_paths.codex_dir()).unwrap();

        let stale_saved = serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "account_id": "acct-stale",
                "access_token": "token-stale",
                "refresh_token": "refresh-stale"
            },
            "last_refresh": "2026-03-01T00:00:00Z"
        });
        let current = serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "account_id": "acct-current",
                "access_token": "token-current",
                "refresh_token": "refresh-current"
            },
            "last_refresh": "2026-03-26T00:00:00Z"
        });
        store.save_snapshot("team", &stale_saved).unwrap();
        fs::write(
            codex_paths.auth_path(),
            serde_json::to_vec_pretty(&current).unwrap(),
        )
        .unwrap();
        fs::write(codex_paths.current_name_path(), "team\n").unwrap();
        write_usage_cache(
            codex_paths.limit_cache_path(),
            "acct-current",
            sample_usage("team"),
        );

        let usage = UsageService::new(
            codex_paths.limit_cache_path().to_path_buf(),
            codex_paths.usage_history_path().to_path_buf(),
            300,
        );
        let report = load_profiles_with_report(&store, &usage, false, None, true, None, None, None).unwrap();

        let codex_profile = report
            .profiles
            .iter()
            .find(|profile| profile.kind == ProfileKind::Codex)
            .expect("codex profile");
        assert_eq!(codex_profile.saved_name.as_deref(), Some("team"));
        assert_eq!(codex_profile.account_id.as_deref(), Some("acct-current"));
        assert!(codex_profile.is_current);
        assert_eq!(
            codex_profile
                .snapshot
                .get("tokens")
                .and_then(|tokens| tokens.get("account_id"))
                .and_then(|value| value.as_str()),
            Some("acct-current")
        );

        let saved_raw = fs::read_to_string(codex_paths.accounts_dir().join("team.json")).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&saved_raw).unwrap();
        assert_eq!(saved, current);
    }

    #[test]
    fn cache_only_load_uses_stale_cache_without_fetching() {
        let base = unique_temp_dir("loader-cache-only");
        let codex_paths = AppPaths::from_codex_dir(base.join("codex"));
        let store = AccountStore::new(codex_paths.clone(), StorePlatform::Copy);
        fs::create_dir_all(codex_paths.codex_dir()).unwrap();

        let snapshot = serde_json::json!({
            "auth_mode": "chatgpt",
            "tokens": {
                "account_id": "acct-cache-only",
                "access_token": "token-cache-only",
                "refresh_token": "refresh-cache-only"
            },
            "last_refresh": "2026-03-01T00:00:00Z"
        });
        store.save_snapshot("cache-only", &snapshot).unwrap();
        fs::write(
            codex_paths.auth_path(),
            serde_json::to_vec_pretty(&snapshot).unwrap(),
        )
        .unwrap();
        fs::write(codex_paths.current_name_path(), "cache-only\n").unwrap();

        let stale_usage = sample_usage("team");
        let stale_cache = UsageCache::from_entries([(
            "acct-cache-only".to_string(),
            100,
            stale_usage.clone(),
        )]);
        fs::write(
            codex_paths.limit_cache_path(),
            format!("{}\n", serde_json::to_string_pretty(&stale_cache).unwrap()),
        )
        .unwrap();

        let usage = UsageService::new(
            codex_paths.limit_cache_path().to_path_buf(),
            codex_paths.usage_history_path().to_path_buf(),
            300,
        )
        .with_now_seconds(1_000)
        .with_fetcher(|_, _| panic!("cache-only load should not fetch from API"));

        let report = load_profiles_with_report(&store, &usage, false, None, true, None, None, None).unwrap();
        let codex_profile = report
            .profiles
            .iter()
            .find(|profile| profile.kind == ProfileKind::Codex)
            .expect("codex profile");

        assert_eq!(codex_profile.usage_view.source, UsageSource::Cache);
        assert!(codex_profile.usage_view.stale);
        assert_eq!(codex_profile.usage_view.usage.as_ref(), Some(&stale_usage));
    }

    #[test]
    fn claude_saved_profile_uses_current_snapshot_when_name_matches() {
        let base = unique_temp_dir("loader-claude-current-sync");
        let codex_paths = AppPaths::from_codex_dir(base.join("codex"));
        let store = AccountStore::new(codex_paths, StorePlatform::Copy);
        let claude_paths = ClaudePaths::from_claude_dir(base.join("claude"));
        fs::create_dir_all(claude_paths.claude_dir()).unwrap();

        let stale_saved = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "token-stale",
                "refreshToken": "sk-ant-ort01-stale-token",
                "expiresAt": 1700000000,
                "subscriptionType": "pro"
            }
        });
        let current = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "token-current",
                "refreshToken": "sk-ant-ort01-current-token",
                "expiresAt": 1700000000,
                "subscriptionType": "pro"
            }
        });
        let claude_store = ClaudeStore::new(claude_paths.clone());
        claude_store.save_snapshot("claude-one", &stale_saved).unwrap();
        fs::write(
            claude_paths.credentials_path(),
            serde_json::to_vec_pretty(&current).unwrap(),
        )
        .unwrap();
        fs::write(claude_paths.current_name_path(), "claude-one\n").unwrap();

        let claude_usage = UsageService::new(
            claude_paths.limit_cache_path().to_path_buf(),
            claude_paths.usage_history_path().to_path_buf(),
            300,
        )
        .with_fetcher(|_, access_token| {
            assert_eq!(access_token, "token-current");
            Ok(sample_usage("pro"))
        });

        let report = load_profiles_with_report(
            &store,
            &UsageService::new(base.join("noop-cache.json"), base.join("noop-history.json"), 300),
            false,
            None,
            true,
            Some(&claude_store),
            Some(&claude_usage),
            None,
        )
        .unwrap();

        let claude_profile = report
            .profiles
            .iter()
            .find(|profile| profile.kind == ProfileKind::Claude)
            .expect("claude profile");
        let current_creds: crate::claude::ClaudeCredentials =
            serde_json::from_value(current.clone()).unwrap();
        assert_eq!(
            claude_profile.account_id.as_deref(),
            Some(current_creds.account_id().as_str())
        );
        assert!(claude_profile.is_current);
        assert_eq!(
            claude_profile
                .snapshot
                .get("claudeAiOauth")
                .and_then(|oauth| oauth.get("accessToken"))
                .and_then(|value| value.as_str()),
            Some("token-current")
        );

        let saved_raw = fs::read_to_string(claude_paths.accounts_dir().join("claude-one.json")).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&saved_raw).unwrap();
        assert_eq!(saved, current);
    }

    #[test]
    fn claude_saved_profile_merges_old_history_into_current_key() {
        let base = unique_temp_dir("loader-claude-history-merge");
        let codex_paths = AppPaths::from_codex_dir(base.join("codex"));
        let store = AccountStore::new(codex_paths, StorePlatform::Copy);
        let claude_paths = ClaudePaths::from_claude_dir(base.join("claude"));
        fs::create_dir_all(claude_paths.claude_dir()).unwrap();

        let stale_saved = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "token-stale",
                "refreshToken": "sk-ant-ort01-stale-token",
                "expiresAt": 1700000000,
                "subscriptionType": "team"
            }
        });
        let current = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "token-current",
                "refreshToken": "sk-ant-ort01-current-token",
                "expiresAt": 1700000000,
                "subscriptionType": "team"
            }
        });
        let claude_store = ClaudeStore::new(claude_paths.clone());
        claude_store.save_snapshot("claude-team", &stale_saved).unwrap();
        fs::write(
            claude_paths.credentials_path(),
            serde_json::to_vec_pretty(&current).unwrap(),
        )
        .unwrap();
        fs::write(claude_paths.current_name_path(), "claude-team\n").unwrap();

        let stale_creds: crate::claude::ClaudeCredentials =
            serde_json::from_value(stale_saved.clone()).unwrap();
        let current_creds: crate::claude::ClaudeCredentials =
            serde_json::from_value(current.clone()).unwrap();
        let stale_comp_id = format!("{}|{}", stale_creds.account_id(), stale_creds.subscription_type());
        let current_comp_id =
            format!("{}|{}", current_creds.account_id(), current_creds.subscription_type());

        let claude_usage = UsageService::new(
            claude_paths.limit_cache_path().to_path_buf(),
            claude_paths.usage_history_path().to_path_buf(),
            300,
        )
        .with_fetcher(|_, _| Ok(sample_usage("team")));
        let mut history = UsageHistoryCache::default();
        history.by_account_id.insert(
            stale_comp_id.clone(),
            ProfileUsageHistory {
                weekly_windows: vec![UsageWindowHistory {
                    limit_window_seconds: 604_800,
                    start_at: 0,
                    end_at: 604_800,
                    observations: vec![UsageObservation {
                        observed_at: 123,
                        used_percent: 45.0,
                    }],
                }],
                five_hour_windows: vec![UsageWindowHistory {
                    limit_window_seconds: 18_000,
                    start_at: 10,
                    end_at: 20,
                    observations: vec![UsageObservation {
                        observed_at: 15,
                        used_percent: 55.0,
                    }],
                }],
                ..ProfileUsageHistory::default()
            },
        );
        fs::write(
            claude_paths.usage_history_path(),
            format!("{}\n", serde_json::to_string_pretty(&history).unwrap()),
        )
        .unwrap();

        let _ = load_profiles_with_report(
            &store,
            &UsageService::new(base.join("noop-cache.json"), base.join("noop-history.json"), 300),
            false,
            None,
            true,
            Some(&claude_store),
            Some(&claude_usage),
            None,
        )
        .unwrap();

        let merged_history: UsageHistoryCache = serde_json::from_str(
            &fs::read_to_string(claude_paths.usage_history_path()).unwrap(),
        )
        .unwrap();
        let current_entry = merged_history
            .by_account_id
            .get(&current_comp_id)
            .expect("current history entry");
        assert!(
            current_entry
                .observations
                .iter()
                .any(|observation| {
                    observation.observed_at_local == 123
                        && observation.weekly_used_percent == Some(45.0)
                })
        );
        assert!(
            current_entry
                .observations
                .iter()
                .any(|observation| {
                    observation.observed_at_local == 15
                        && observation.five_hour_used_percent == Some(55.0)
                })
        );
        assert!(!merged_history.by_account_id.contains_key(&stale_comp_id));
    }

    #[test]
    fn force_refresh_collects_claude_errors_without_dropping_codex_profiles() {
        let base = unique_temp_dir("loader-partial-refresh");
        let codex_paths = AppPaths::from_codex_dir(base.join("codex"));
        let store = AccountStore::new(codex_paths.clone(), StorePlatform::Copy);
        let codex_snapshot = serde_json::json!({
            "tokens": { "account_id": "acct-codex", "access_token": "token-codex" }
        });
        store.save_snapshot("codex-one", &codex_snapshot).unwrap();
        fs::create_dir_all(codex_paths.codex_dir()).unwrap();
        fs::write(
            codex_paths.auth_path(),
            serde_json::to_vec_pretty(&codex_snapshot).unwrap(),
        )
        .unwrap();

        let claude_paths = ClaudePaths::from_claude_dir(base.join("claude"));
        fs::create_dir_all(claude_paths.claude_dir()).unwrap();
        let claude_snapshot = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "token-claude",
                "refreshToken": "sk-ant-ort01-refresh-token-claude",
                "expiresAt": 1700000000,
                "subscriptionType": "pro"
            }
        });
        fs::write(
            claude_paths.credentials_path(),
            serde_json::to_vec_pretty(&claude_snapshot).unwrap(),
        )
        .unwrap();
        let claude_store = ClaudeStore::new(claude_paths.clone());
        claude_store.save_snapshot("claude-one", &claude_snapshot).unwrap();

        let codex_usage = UsageService::new(
            codex_paths.limit_cache_path().to_path_buf(),
            codex_paths.usage_history_path().to_path_buf(),
            300,
        )
        .with_fetcher(|_, _| Ok(sample_usage("team")));
        let claude_usage = UsageService::new(
            claude_paths.limit_cache_path().to_path_buf(),
            claude_paths.usage_history_path().to_path_buf(),
            300,
        )
        .with_fetcher(|_, _| Err(anyhow::anyhow!("Claude usage request failed: 429")));

        let report = load_profiles_with_report(
            &store,
            &codex_usage,
            true,
            None,
            false,
            Some(&claude_store),
            Some(&claude_usage),
            None,
        )
        .unwrap();

        assert!(report
            .profiles
            .iter()
            .any(|profile| profile.kind == ProfileKind::Codex));
        assert!(report
            .profiles
            .iter()
            .any(|profile| profile.kind == ProfileKind::Claude));
        assert!(report
            .refresh_errors
            .iter()
            .any(|error| error.contains("Claude")));
    }

    #[test]
    fn weekly_points_project_from_window_range_includes_observations_across_nearby_windows() {
        // Target weekly window: [start_at, end_at] mapped to 0..7.
        let weekly = UsageWindow {
            used_percent: 10.0,
            limit_window_seconds: 604_800,
            reset_after_seconds: 0,
            reset_at: 10_000,
        };
        let start_at = weekly.reset_at - weekly.limit_window_seconds;
        let end_at = weekly.reset_at;
        assert_eq!(end_at - start_at, 604_800);

        // Simulate API jitter: history windows have slightly different start/end, but observations
        // still land inside the target [start_at, end_at] range.
        let history_windows = vec![
            UsageWindowHistory {
                limit_window_seconds: weekly.limit_window_seconds,
                start_at: start_at + 60,
                end_at: end_at - 60,
                observations: vec![
                    UsageObservation {
                        observed_at: start_at + 100,
                        used_percent: 12.0,
                    },
                    UsageObservation {
                        observed_at: end_at - 100,
                        used_percent: 18.0,
                    },
                ],
            },
            UsageWindowHistory {
                limit_window_seconds: weekly.limit_window_seconds,
                start_at: start_at - 120,
                end_at: end_at + 120,
                observations: vec![UsageObservation {
                    observed_at: (start_at + end_at) / 2,
                    used_percent: 24.0,
                }],
            },
        ];

        let points = project_weekly_points_for_window_range(&weekly, &history_windows);
        // All 3 observations should be projected into the 0..7 axis.
        assert_eq!(points.len(), 3);
        assert!(points.iter().any(|p| (p.y - 12.0).abs() < f64::EPSILON));
        assert!(points.iter().any(|p| (p.y - 24.0).abs() < f64::EPSILON));
        assert!(points.iter().any(|p| (p.y - 18.0).abs() < f64::EPSILON));
        assert!(points.iter().all(|p| p.x >= 0.0 && p.x <= 7.0));
    }

    #[test]
    fn chart_projects_only_last_7d_points_from_profile_observations() {
        let end_at = current_unix_seconds();
        let weekly = UsageWindow {
            used_percent: 50.0,
            limit_window_seconds: 604_800,
            reset_after_seconds: 0,
            // Reset can be in the future; projection should still use rolling-now end.
            reset_at: end_at + 300_000,
        };
        let observations = (0..2000_i64)
            .map(|i| crate::usage::ProfileUsageObservation {
                observed_at_local: end_at - ((1999 - i) * 600),
                weekly_used_percent: Some((i % 100) as f64),
                five_hour_used_percent: Some((i % 50) as f64),
            })
            .collect::<Vec<_>>();

        let points = project_weekly_points_from_profile_observations(&weekly, &observations);
        assert_eq!(points.len(), 1008);
        assert!(points.iter().all(|point| point.x >= 0.0 && point.x <= 7.0));
    }

    #[test]
    fn chart_remains_visible_after_concurrent_app_cron_writes() {
        let cache_path = PathBuf::from("dummy_cache_concurrent.json");
        let history_path = std::env::temp_dir().join(format!(
            "test_concurrent_visibility_{}.json",
            std::process::id()
        ));
        let account_id = "acct-concurrent-chart";
        let base_now = current_unix_seconds() - 7_200;

        let app_service = UsageService::new(cache_path.clone(), history_path.clone(), 300);
        let cron_service = UsageService::new(cache_path, history_path.clone(), 300);

        for i in 0..36_i64 {
            let app_now = base_now + i * 120;
            let cron_now = app_now + 30;
            let app_usage = UsageResponse {
                email: None,
                plan_type: Some("plus".to_string()),
                rate_limit: Some(UsageRateLimit {
                    primary_window: Some(UsageWindow {
                        used_percent: 12.0,
                        limit_window_seconds: 18_000,
                        reset_after_seconds: 18_000,
                        reset_at: app_now + 18_000,
                    }),
                    secondary_window: Some(UsageWindow {
                        used_percent: 22.0,
                        limit_window_seconds: 604_800,
                        reset_after_seconds: 604_800,
                        reset_at: app_now + 604_800,
                    }),
                }),
            };
            let cron_usage = UsageResponse {
                email: None,
                plan_type: Some("plus".to_string()),
                rate_limit: Some(UsageRateLimit {
                    primary_window: Some(UsageWindow {
                        used_percent: 13.0,
                        limit_window_seconds: 18_000,
                        reset_after_seconds: 18_000,
                        reset_at: cron_now + 18_000,
                    }),
                    secondary_window: Some(UsageWindow {
                        used_percent: 23.0,
                        limit_window_seconds: 604_800,
                        reset_after_seconds: 604_800,
                        reset_at: cron_now + 604_800,
                    }),
                }),
            };

            app_service
                .clone()
                .with_now_seconds(app_now)
                .record_usage_snapshot(
                    Some(account_id),
                    &UsageReadResult {
                        usage: Some(app_usage),
                        source: crate::usage::UsageSource::Api,
                        fetched_at: Some(app_now),
                        stale: false,
                    },
                )
                .unwrap();
            cron_service
                .clone()
                .with_now_seconds(cron_now)
                .record_usage_snapshot(
                    Some(account_id),
                    &UsageReadResult {
                        usage: Some(cron_usage),
                        source: crate::usage::UsageSource::Api,
                        fetched_at: Some(cron_now),
                        stale: false,
                    },
                )
                .unwrap();
        }

        let latest_usage = UsageResponse {
            email: None,
            plan_type: Some("plus".to_string()),
            rate_limit: Some(UsageRateLimit {
                primary_window: Some(UsageWindow {
                    used_percent: 15.0,
                    limit_window_seconds: 18_000,
                    reset_after_seconds: 18_000,
                    reset_at: current_unix_seconds() + 18_000,
                }),
                secondary_window: Some(UsageWindow {
                    used_percent: 25.0,
                    limit_window_seconds: 604_800,
                    reset_after_seconds: 604_800,
                    reset_at: current_unix_seconds() + 604_800,
                }),
            }),
        };
        let chart_data = build_profile_chart_data(Some(account_id), Some(&latest_usage), &app_service).unwrap();
        assert!(
            chart_data.seven_day_points.len() >= 20,
            "chart should keep visible points after concurrent app/cron appends"
        );
        assert!(chart_data.seven_day_points.iter().all(|p| p.x >= 0.0 && p.x <= 7.0));

        let _ = std::fs::remove_file(history_path);
    }
}
