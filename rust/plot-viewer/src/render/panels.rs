use super::{FocusTarget, RenderContext, RenderState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelSkeleton {
    pub title: String,
    pub summary_lines: Vec<String>,
    pub compare_lines: Vec<String>,
}

/// Build a structured side-panel skeleton for the plot viewer.
///
/// Lane 4 now consumes the runtime-owned selection/chart state directly so the
/// Summary / Compare copy stays aligned with the visible viewer behavior.
pub fn render_panels<State: RenderState>(context: &RenderContext<'_, State>) -> PanelSkeleton {
    let selection = context.state.selection_state();
    let chart = context.state.chart_state();
    let viewport = context.viewport;
    let selected = selection.selected;
    let current = selection.current;
    let selected_label = selected
        .map(|profile| profile.label)
        .unwrap_or("no profiles loaded");
    let current_label = current
        .map(|profile| profile.label)
        .unwrap_or("none");
    let focus_label = match selection.focus {
        FocusTarget::Chart => "Chart",
        FocusTarget::Summary => "Summary",
    };
    let band_line = if chart.five_hour_band.available {
        match (chart.five_hour_band.lower_y, chart.five_hour_band.upper_y) {
            (Some(lower_y), Some(upper_y)) => {
                format!("5h band: {:.1}%..{:.1}%", lower_y, upper_y)
            }
            _ => "5h band: available but bounds incomplete".to_string(),
        }
    } else {
        format!(
            "5h band: unavailable ({})",
            chart.five_hour_band.reason.unwrap_or("no 5h band data")
        )
    };

    let summary_lines = vec![
        format!("Focused profile: {}", selected_label),
        format!("Snapshot current: {}", current_label),
        format!("Focus panel: {}", focus_label),
        format!("7d samples: {}", chart.seven_day_points.len()),
        band_line.clone(),
    ];

    let compare_lines = if let (Some(selected), Some(current)) = (selected, current) {
        if selected.id == current.id {
            vec![
                "Target vs current".to_string(),
                format!("Adopted target: {} (already current)", selected.label),
                "Routing delta: none".to_string(),
                format!("Compare focus: {}", focus_label),
                band_line,
            ]
        } else {
            vec![
                "Target vs current".to_string(),
                format!("Adopted target: {}", selected.label),
                format!("Current route: {}", current.label),
                "Routing delta: switched target".to_string(),
                band_line,
            ]
        }
    } else {
        vec![
            "Target vs current".to_string(),
            format!("Target: {}", selected_label),
            format!("Current: {}", current_label),
            "Routing delta: insufficient selection state".to_string(),
            band_line,
        ]
    };

    PanelSkeleton {
        title: format!("Routing panels {}x{}", viewport.width, viewport.height),
        summary_lines,
        compare_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{
        ChartPoint, ChartState, FiveHourBandState, RenderProfile, SelectionState,
    };

    #[derive(Debug, Clone)]
    struct MockState {
        selection: SelectionState<'static>,
        chart: ChartState<'static>,
    }

    impl RenderState for MockState {
        fn selection_state(&self) -> SelectionState<'_> {
            self.selection
        }

        fn chart_state(&self) -> ChartState<'_> {
            self.chart.clone()
        }
    }

    #[test]
    fn render_panels_builds_visible_summary_and_compare_blocks() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: false,
                }),
                current: Some(RenderProfile {
                    id: "beta",
                    label: "Beta",
                    is_current: true,
                }),
                focus: FocusTarget::Summary,
            },
            chart: ChartState {
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 10.0 },
                    ChartPoint { x: 2.0, y: 22.0 },
                    ChartPoint { x: 7.0, y: 48.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: true,
                    lower_y: Some(18.0),
                    upper_y: Some(31.0),
                    delta_seven_day_percent: Some(3.0),
                    delta_five_hour_percent: Some(1.0),
                    reason: None,
                },
            },
        };

        let skeleton = render_panels(&RenderContext::new(
            &state,
            ratatui::layout::Rect::new(0, 0, 48, 12),
        ));

        assert_eq!(skeleton.title, "Routing panels 48x12");
        assert_eq!(skeleton.summary_lines.len(), 5);
        assert_eq!(skeleton.compare_lines.len(), 5);
        assert_eq!(skeleton.summary_lines[0], "Focused profile: Alpha");
        assert_eq!(skeleton.summary_lines[1], "Snapshot current: Beta");
        assert_eq!(skeleton.summary_lines[2], "Focus panel: Summary");
        assert_eq!(skeleton.summary_lines[3], "7d samples: 3");
        assert_eq!(skeleton.summary_lines[4], "5h band: 18.0%..31.0%");
        assert_eq!(skeleton.compare_lines[1], "Adopted target: Alpha");
        assert_eq!(skeleton.compare_lines[2], "Current route: Beta");
        assert_eq!(skeleton.compare_lines[3], "Routing delta: switched target");
    }

    #[test]
    fn render_panels_locks_visible_summary_compare_copy_and_shape() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: true,
                }),
                current: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: true,
                }),
                focus: FocusTarget::Chart,
            },
            chart: ChartState {
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 12.0 },
                    ChartPoint { x: 5.0, y: 62.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: false,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("insufficient 5h overlap"),
                },
            },
        };

        let skeleton = render_panels(&RenderContext::new(
            &state,
            ratatui::layout::Rect::new(0, 0, 40, 10),
        ));

        assert_eq!(
            skeleton.summary_lines,
            vec![
                "Focused profile: Alpha".to_string(),
                "Snapshot current: Alpha".to_string(),
                "Focus panel: Chart".to_string(),
                "7d samples: 2".to_string(),
                "5h band: unavailable (insufficient 5h overlap)".to_string(),
            ]
        );
        assert_eq!(
            skeleton.compare_lines,
            vec![
                "Target vs current".to_string(),
                "Adopted target: Alpha (already current)".to_string(),
                "Routing delta: none".to_string(),
                "Compare focus: Chart".to_string(),
                "5h band: unavailable (insufficient 5h overlap)".to_string(),
            ]
        );
    }
}
