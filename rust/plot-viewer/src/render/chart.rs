use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::{Color, Frame};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap};

use super::{ChartState, FocusTarget, RenderContext, RenderState};

const X_AXIS_BOUNDS: [f64; 2] = [0.0, 7.0];
const Y_AXIS_BOUNDS: [f64; 2] = [0.0, 100.0];

/// Visible chart renderer for the plot viewer.
///
/// Lane 3 upgrades the shell from placeholder copy to a real chart backed by
/// the selected profile's seven-day points, with a readable five-hour band
/// summary and overlays when the data is available.
pub fn render_chart<State: RenderState>(frame: &mut Frame, context: &RenderContext<'_, State>) {
    let selection = context.state.selection_state();
    let chart_state = context.state.chart_state();
    let selected_label = selection
        .selected
        .map(|profile| profile.label)
        .unwrap_or("no profiles loaded");
    let current_label = selection
        .current
        .map(|profile| profile.label)
        .unwrap_or("none");
    let focus_label = match selection.focus {
        FocusTarget::Chart => "Chart",
        FocusTarget::Summary => "Summary",
    };
    let block = Block::default()
        .title("plot chart")
        .borders(Borders::ALL);
    let inner = block.inner(context.area);
    frame.render_widget(block, context.area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(6),
        Constraint::Length(3),
    ])
    .split(inner);

    let overview = Paragraph::new(Text::from(vec![
        Line::from(format!("Selected: {}", selected_label)),
        Line::from(format!("Current: {}", current_label)),
        Line::from(format!("Focus: {}", focus_label)),
        Line::from(format!("7d usage: {} points", chart_state.seven_day_points.len())),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(overview, chunks[0]);

    render_usage_chart(frame, chunks[1], &chart_state);

    let band_summary = Paragraph::new(Text::from(vec![
        Line::from(format_five_hour_band_line(&chart_state)),
        Line::from(format_five_hour_delta_line(&chart_state)),
        Line::from("Axis: 7d window, Y = usage%"),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(band_summary, chunks[2]);
}

fn render_usage_chart(frame: &mut Frame, area: ratatui::layout::Rect, chart_state: &ChartState<'_>) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let seven_day_points: Vec<(f64, f64)> = chart_state
        .seven_day_points
        .iter()
        .map(|point| (point.x, point.y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])))
        .collect();
    let mut overlays: Vec<Vec<(f64, f64)>> = Vec::new();
    if chart_state.five_hour_band.available {
        if let Some(lower_y) = chart_state.five_hour_band.lower_y {
            overlays.push(vec![
                (X_AXIS_BOUNDS[0], lower_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
                (X_AXIS_BOUNDS[1], lower_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
            ]);
        }
        if let Some(upper_y) = chart_state.five_hour_band.upper_y {
            overlays.push(vec![
                (X_AXIS_BOUNDS[0], upper_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
                (X_AXIS_BOUNDS[1], upper_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
            ]);
        }
    }

    let mut datasets = vec![Dataset::default()
        .name("7d usage")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Color::Cyan)
        .data(&seven_day_points)];

    if let Some(lower_points) = overlays.first() {
        datasets.push(
            Dataset::default()
                .name("5h lower")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Color::Yellow)
                .data(lower_points),
        );
    }
    if let Some(upper_points) = overlays.get(1) {
        datasets.push(
            Dataset::default()
                .name("5h upper")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Color::Magenta)
                .data(upper_points),
        );
    }

    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .title("7d window")
                .bounds(X_AXIS_BOUNDS)
                .labels(["0d", "3.5d", "7d"]),
        )
        .y_axis(
            Axis::default()
                .title("usage%")
                .bounds(Y_AXIS_BOUNDS)
                .labels(["0%", "50%", "100%"]),
        );
    frame.render_widget(chart, area);
}

fn format_five_hour_band_line(chart_state: &ChartState<'_>) -> String {
    if chart_state.five_hour_band.available {
        match (
            chart_state.five_hour_band.lower_y,
            chart_state.five_hour_band.upper_y,
        ) {
            (Some(lower_y), Some(upper_y)) => {
                format!("5h band: {:.1}%..{:.1}%", lower_y, upper_y)
            }
            _ => "5h band: available but bounds incomplete".to_string(),
        }
    } else {
        let reason = chart_state
            .five_hour_band
            .reason
            .unwrap_or("no 5h band data");
        format!("5h band: unavailable ({})", reason)
    }
}

fn format_five_hour_delta_line(chart_state: &ChartState<'_>) -> String {
    let delta_7d = chart_state
        .five_hour_band
        .delta_seven_day_percent
        .map(|value| format!("{:+.1}%", value))
        .unwrap_or("?".to_string());
    let delta_5h = chart_state
        .five_hour_band
        .delta_five_hour_percent
        .map(|value| format!("{:+.1}%", value))
        .unwrap_or("?".to_string());
    format!("Band drift: 7d {} | 5h {}", delta_7d, delta_5h)
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::render::{
        ChartPoint, FiveHourBandState, RenderProfile, SelectionState,
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

    fn render_lines(state: &MockState, width: u16, height: u16) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let frame = terminal
            .draw(|frame| render_chart(frame, &RenderContext::new(state, frame.area())))
            .unwrap();

        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| frame.buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    fn visible_chart_glyph_count(lines: &[String]) -> usize {
        lines.iter()
            .flat_map(|line| line.chars())
            .filter(|symbol| matches!(symbol, '⠁'..='⣿'))
            .count()
    }

    #[test]
    fn render_chart_draws_visible_seven_day_curve_and_band_summary() {
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
                focus: FocusTarget::Chart,
            },
            chart: ChartState {
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 8.0 },
                    ChartPoint { x: 1.0, y: 18.0 },
                    ChartPoint { x: 3.5, y: 44.0 },
                    ChartPoint { x: 5.0, y: 58.0 },
                    ChartPoint { x: 7.0, y: 76.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: true,
                    lower_y: Some(20.0),
                    upper_y: Some(35.0),
                    delta_seven_day_percent: Some(4.0),
                    delta_five_hour_percent: Some(1.5),
                    reason: None,
                },
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("Selected: Alpha"));
        assert!(joined.contains("Current: Beta"));
        assert!(joined.contains("7d usage: 5 points"));
        assert!(joined.contains("5h band: 20.0%..35.0%"));
        assert!(joined.contains("Band drift: 7d +4.0% | 5h +1.5%"));
        assert!(joined.contains("0d"));
        assert!(joined.contains("3.5d"));
        assert!(joined.contains("7d"));
        assert!(joined.contains("100%"));
        assert!(!joined.contains("pending Canvas plot"));
        assert!(!joined.contains("reserved for projected overlap lines"));
        assert!(
            visible_chart_glyph_count(&lines) > 0,
            "expected chart area to contain braille glyphs, got:\n{}",
            joined
        );
    }

    #[test]
    fn render_chart_surfaces_unavailable_band_reason_without_placeholder_copy() {
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
                focus: FocusTarget::Summary,
            },
            chart: ChartState {
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 12.0 },
                    ChartPoint { x: 4.0, y: 40.0 },
                    ChartPoint { x: 7.0, y: 61.0 },
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

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("Focus: Summary"));
        assert!(joined.contains("5h band: unavailable (insufficient 5h overlap)"));
        assert!(joined.contains("Band drift: 7d ? | 5h ?"));
        assert!(!joined.contains("pending Canvas plot"));
    }
}
