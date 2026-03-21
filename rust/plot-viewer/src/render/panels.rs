use super::{RenderContext, RenderState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelSkeleton {
    pub title: String,
    pub summary_lines: Vec<String>,
    pub compare_lines: Vec<String>,
}

/// Build a structured side-panel skeleton for the plot viewer.
///
/// This lane stays intentionally layout-light: it gathers the summary and
/// compare content that later render passes can paint, but it does not depend
/// on the chart or panel widget implementation yet.
pub fn render_panels<State: RenderState>(context: &RenderContext<'_, State>) -> PanelSkeleton {
    let state = context.state;
    let viewport = context.viewport;
    let summary_lines = vec![
        format!("Focused profile: {}", state.selected_profile_label()),
        format!("Snapshot current: {}", state.snapshot_active_label()),
        format!("Focus panel: {}", state.focus_label()),
        format!("Viewport: {}x{}", viewport.width, viewport.height),
    ];

    let compare_lines = vec![
        "Target vs current".to_string(),
        format!("Target: {}", state.selected_profile_label()),
        format!("Current: {}", state.snapshot_active_label()),
        "Routing note: compare skeleton awaiting richer snapshot metadata".to_string(),
    ];

    PanelSkeleton {
        title: "Routing panels".to_string(),
        summary_lines,
        compare_lines,
    }
}
