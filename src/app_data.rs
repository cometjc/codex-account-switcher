use serde_json::Value;

use crate::render::ChartPoint;
use crate::usage::UsageReadResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Codex,
    Claude,
    Copilot,
}

impl ProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Copilot => "copilot",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileEntry {
    pub kind: ProfileKind,
    pub saved_name: Option<String>,
    pub profile_name: String,
    pub snapshot: Value,
    pub usage_view: UsageReadResult,
    pub account_id: Option<String>,
    pub is_current: bool,
    pub chart_data: ProfileChartData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileChartData {
    pub seven_day_points: Vec<ChartPoint>,
    pub quota_window_label: String,
    pub forecast: OwnedUsageForecast,
    // Optional prepared countdowns for chart reset-line feature. None when unavailable or <= 0.
    pub weekly_reset_countdown_seconds: Option<i64>,
    pub five_hour_reset_countdown_seconds: Option<i64>,
    pub five_hour_band: OwnedFiveHourBandState,
    pub five_hour_subframe: OwnedFiveHourSubframeState,
    pub is_zero_state: bool,
}

impl ProfileChartData {
    pub fn empty(reason: &str) -> Self {
        Self {
            seven_day_points: Vec::new(),
            quota_window_label: "?d".to_string(),
            forecast: OwnedUsageForecast::empty(reason),
            weekly_reset_countdown_seconds: None,
            five_hour_reset_countdown_seconds: None,
            five_hour_band: OwnedFiveHourBandState {
                available: false,
                used_percent: None,
                lower_y: None,
                upper_y: None,
                delta_seven_day_percent: None,
                delta_five_hour_percent: None,
                reason: Some(reason.to_string()),
            },
            five_hour_subframe: OwnedFiveHourSubframeState {
                available: false,
                start_x: None,
                end_x: None,
                lower_y: None,
                upper_y: None,
                reason: Some(reason.to_string()),
            },
            is_zero_state: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastEventKind {
    Hit,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastConfidence {
    Low,
    High,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedUsageForecast {
    pub event: Option<ForecastEventKind>,
    pub eta_seconds: Option<i64>,
    pub compact_label: Option<String>,
    pub confidence: ForecastConfidence,
    pub reason: Option<String>,
}

impl OwnedUsageForecast {
    pub fn empty(reason: &str) -> Self {
        Self {
            event: None,
            eta_seconds: None,
            compact_label: None,
            confidence: ForecastConfidence::Low,
            reason: Some(reason.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedFiveHourBandState {
    pub available: bool,
    pub used_percent: Option<f64>,
    pub lower_y: Option<f64>,
    pub upper_y: Option<f64>,
    pub delta_seven_day_percent: Option<f64>,
    pub delta_five_hour_percent: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnedFiveHourSubframeState {
    pub available: bool,
    pub start_x: Option<f64>,
    pub end_x: Option<f64>,
    pub lower_y: Option<f64>,
    pub upper_y: Option<f64>,
    pub reason: Option<String>,
}
