use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlotSnapshot {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u8,
    #[serde(rename = "generatedAt")]
    pub generated_at: i64,
    #[serde(rename = "currentProfileId")]
    pub current_profile_id: Option<String>,
    pub profiles: Vec<PlotProfile>,
    #[serde(skip)]
    pub active_profile_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlotProfile {
    pub id: String,
    pub name: String,
    #[serde(rename = "isCurrent")]
    pub is_current: bool,
    pub usage: Option<Value>,
    #[serde(rename = "sevenDayWindow")]
    pub seven_day_window: PlotWindowBounds,
    #[serde(rename = "sevenDayPoints")]
    pub seven_day_points: Vec<PlotWindowPoint>,
    #[serde(rename = "fiveHourWindow")]
    pub five_hour_window: PlotWindowBounds,
    #[serde(rename = "fiveHourBand")]
    pub five_hour_band: PlotFiveHourBand,
    #[serde(rename = "summaryLabels", default)]
    pub summary_labels: PlotSummaryLabels,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlotWindowBounds {
    #[serde(rename = "startAt")]
    pub start_at: Option<i64>,
    #[serde(rename = "endAt")]
    pub end_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlotWindowPoint {
    #[serde(rename = "offsetSeconds")]
    pub offset_seconds: i64,
    #[serde(rename = "usedPercent")]
    pub used_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlotFiveHourBand {
    #[serde(default)]
    pub available: bool,
    #[serde(rename = "lowerY")]
    pub lower_y: Option<f64>,
    #[serde(rename = "upperY")]
    pub upper_y: Option<f64>,
    #[serde(rename = "bandHeight")]
    pub band_height: Option<f64>,
    #[serde(rename = "delta7dPercent")]
    pub delta_seven_day_percent: Option<f64>,
    #[serde(rename = "delta5hPercent")]
    pub delta_five_hour_percent: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlotSummaryLabels {
    #[serde(rename = "timeToReset")]
    pub time_to_reset: String,
    #[serde(rename = "usageLeft")]
    pub usage_left: String,
    #[serde(rename = "drift")]
    pub drift: String,
    #[serde(rename = "pacingStatus")]
    pub pacing_status: String,
}

impl Default for PlotSummaryLabels {
    fn default() -> Self {
        Self {
            time_to_reset: "Time to reset".to_string(),
            usage_left: "Usage Left".to_string(),
            drift: "Drift".to_string(),
            pacing_status: "Pacing Status".to_string(),
        }
    }
}

impl PlotSnapshot {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read snapshot file at {}", path.display()))?;
        let mut snapshot: Self = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse snapshot JSON at {}", path.display()))?;
        snapshot.refresh_derived_state();
        Ok(snapshot)
    }

    pub fn active_profile(&self) -> Option<&PlotProfile> {
        self.current_profile().or_else(|| self.profiles.get(self.active_profile_index))
    }

    pub fn current_profile(&self) -> Option<&PlotProfile> {
        self.current_profile_id
            .as_ref()
            .and_then(|current_profile_id| self.profiles.iter().find(|profile| &profile.id == current_profile_id))
    }

    pub fn current_profile_index(&self) -> Option<usize> {
        self.current_profile_id.as_ref().and_then(|current_profile_id| {
            self.profiles
                .iter()
                .position(|profile| &profile.id == current_profile_id)
        })
    }

    fn refresh_derived_state(&mut self) {
        self.active_profile_index = self.current_profile_index().unwrap_or(0);
    }
}
