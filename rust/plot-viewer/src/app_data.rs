use serde_json::Value;

use crate::render::ChartPoint;
use crate::usage::UsageReadResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Codex,
    Claude,
    Copilot,
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
    pub five_hour_band: OwnedFiveHourBandState,
    pub five_hour_subframe: OwnedFiveHourSubframeState,
}

impl ProfileChartData {
    pub fn empty(reason: &str) -> Self {
        Self {
            seven_day_points: Vec::new(),
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
