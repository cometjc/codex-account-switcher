use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::{Color, Frame, Style};
use ratatui::style::Modifier;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap};

use super::{ChartState, FocusTarget, RenderContext, RenderState};

const X_AXIS_BOUNDS: [f64; 2] = [0.0, 7.0];
const Y_AXIS_BOUNDS: [f64; 2] = [0.0, 100.0];

const SERIES_COLORS: [Color; 8] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::LightBlue,
    Color::LightRed,
    Color::LightGreen,
    Color::White,
];

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
        .title("usage plot overlays")
        .borders(Borders::ALL);
    let inner = block.inner(context.area);
    frame.render_widget(block, context.area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(6),
        Constraint::Min(6),
        Constraint::Length(3),
    ])
    .split(inner);

    let overview = Paragraph::new(Text::from(vec![
        Line::from(format!("Selected: {}", selected_label)),
        Line::from(format!("Current: {}", current_label)),
        Line::from(format!("Focus: {}", focus_label)),
        Line::from(format!(
            "Profiles: {} · 7d samples: {}",
            chart_state.series.len(),
            chart_state.total_points
        )),
        Line::from(format_legend_line(&chart_state)),
        Line::from(format_subframe_line(&chart_state)),
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

    let series_points = chart_state
        .series
        .iter()
        .map(|series| {
            series
                .points
                .iter()
                .map(|point| (point.x, point.y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut subframe_overlays: Vec<Vec<(f64, f64)>> = Vec::new();
    if chart_state.five_hour_subframe.available {
        if let (Some(start_x), Some(end_x), Some(lower_y)) = (
            chart_state.five_hour_subframe.start_x,
            chart_state.five_hour_subframe.end_x,
            chart_state.five_hour_subframe.lower_y,
        ) {
            subframe_overlays.push(vec![
                (start_x.clamp(X_AXIS_BOUNDS[0], X_AXIS_BOUNDS[1]), lower_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
                (end_x.clamp(X_AXIS_BOUNDS[0], X_AXIS_BOUNDS[1]), lower_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
            ]);
        }
        if let (Some(start_x), Some(end_x), Some(upper_y)) = (
            chart_state.five_hour_subframe.start_x,
            chart_state.five_hour_subframe.end_x,
            chart_state.five_hour_subframe.upper_y,
        ) {
            subframe_overlays.push(vec![
                (start_x.clamp(X_AXIS_BOUNDS[0], X_AXIS_BOUNDS[1]), upper_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
                (end_x.clamp(X_AXIS_BOUNDS[0], X_AXIS_BOUNDS[1]), upper_y.clamp(Y_AXIS_BOUNDS[0], Y_AXIS_BOUNDS[1])),
            ]);
        }
    }

    let mut datasets = chart_state
        .series
        .iter()
        .zip(series_points.iter())
        .map(|(series, points)| {
            let color = SERIES_COLORS[series.style.color_slot % SERIES_COLORS.len()];
            let mut style = Style::default().fg(color);
            if series.style.is_selected {
                style = style.add_modifier(Modifier::BOLD);
            } else if !series.style.is_current {
                style = style.add_modifier(Modifier::DIM);
            }
            Dataset::default()
                .name(series.profile.label)
                .marker(Marker::Dot)
                .graph_type(GraphType::Line)
                .style(style)
                .data(points)
        })
        .collect::<Vec<_>>();

    if let Some(lower_points) = subframe_overlays.first() {
        datasets.push(
            Dataset::default()
                .name("5h lower")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::White))
                .data(lower_points),
        );
    }
    if let Some(upper_points) = subframe_overlays.get(1) {
        datasets.push(
            Dataset::default()
                .name("5h upper")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::White))
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
            .five_hour_subframe
            .reason
            .unwrap_or("no 5h subframe data");
        format!("5h frame: unavailable ({})", reason)
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

fn format_legend_line(chart_state: &ChartState<'_>) -> String {
    if chart_state.series.is_empty() {
        return "Legend: no profile data".to_string();
    }

    let labels = chart_state
        .series
        .iter()
        .take(5)
        .map(|series| {
            let mut label = series.profile.label.to_string();
            if series.style.is_selected {
                label.push('*');
            }
            if series.style.is_current {
                label.push('•');
            }
            label
        })
        .collect::<Vec<_>>()
        .join(" | ");
    if chart_state.series.len() > 5 {
        format!("Legend: {} | +{} more", labels, chart_state.series.len() - 5)
    } else {
        format!("Legend: {}", labels)
    }
}

fn format_subframe_line(chart_state: &ChartState<'_>) -> String {
    if chart_state.five_hour_subframe.available {
        match (
            chart_state.five_hour_subframe.start_x,
            chart_state.five_hour_subframe.end_x,
            chart_state.five_hour_subframe.lower_y,
            chart_state.five_hour_subframe.upper_y,
        ) {
            (Some(start_x), Some(end_x), Some(lower_y), Some(upper_y)) => format!(
                "5h frame: {:.1}d..{:.1}d @ {:.1}%..{:.1}%",
                start_x, end_x, lower_y, upper_y
            ),
            _ => "5h frame: bounds incomplete".to_string(),
        }
    } else {
        format!(
            "5h frame: unavailable ({})",
            chart_state
                .five_hour_subframe
                .reason
                .unwrap_or("no 5h subframe data")
        )
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::render::{
        ChartPoint, ChartSeries, ChartSeriesStyle, FiveHourBandState, FiveHourSubframeState,
        RenderProfile, SelectionState,
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
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "alpha",
                        label: "Alpha",
                        is_current: false,
                    },
                    style: ChartSeriesStyle {
                        color_slot: 0,
                        is_selected: true,
                        is_current: false,
                    },
                    points: vec![
                        ChartPoint { x: 0.0, y: 8.0 },
                        ChartPoint { x: 1.0, y: 18.0 },
                        ChartPoint { x: 3.5, y: 44.0 },
                        ChartPoint { x: 5.0, y: 58.0 },
                        ChartPoint { x: 7.0, y: 76.0 },
                    ],
                }],
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
                five_hour_subframe: FiveHourSubframeState {
                    available: true,
                    start_x: Some(5.0),
                    end_x: Some(6.0),
                    lower_y: Some(20.0),
                    upper_y: Some(35.0),
                    reason: None,
                },
                total_points: 5,
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("Selected: Alpha"));
        assert!(joined.contains("Current: Beta"));
        assert!(joined.contains("Profiles: 1"));
        assert!(joined.contains("7d samples: 5"));
        assert!(joined.contains("Legend: Alpha*"));
        assert!(joined.contains("5h frame: 5.0d..6.0d @ 20.0%..35.0%"));
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
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "alpha",
                        label: "Alpha",
                        is_current: true,
                    },
                    style: ChartSeriesStyle {
                        color_slot: 0,
                        is_selected: true,
                        is_current: true,
                    },
                    points: vec![
                        ChartPoint { x: 0.0, y: 12.0 },
                        ChartPoint { x: 4.0, y: 40.0 },
                        ChartPoint { x: 7.0, y: 61.0 },
                    ],
                }],
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
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("insufficient 5h overlap"),
                },
                total_points: 3,
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("Focus: Summary"));
        assert!(joined.contains("5h frame: unavailable (insufficient 5h overlap)"));
        assert!(joined.contains("Band drift: 7d ? | 5h ?"));
        assert!(!joined.contains("pending Canvas plot"));
    }
}
