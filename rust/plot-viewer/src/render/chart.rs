use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::{Color, Frame, Style};
use ratatui::style::Modifier;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap};

use super::{ChartState, RenderContext, RenderState, SERIES_COLORS};


pub fn render_chart<State: RenderState>(frame: &mut Frame, context: &RenderContext<'_, State>) {
    let chart_state = context.state.chart_state();
    let block = Block::default()
        .title("usage plot overlays")
        .borders(Borders::ALL);
    let inner = block.inner(context.area);
    frame.render_widget(block, context.area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [chart_area, band_area] =
        Layout::vertical([Constraint::Min(6), Constraint::Length(3)]).areas(inner);

    render_usage_chart(frame, chart_area, &chart_state);

    let band_summary = Paragraph::new(Text::from(vec![
        Line::from(format_five_hour_band_line(&chart_state)),
        Line::from(format_five_hour_delta_line(&chart_state)),
        Line::from(match chart_state.cursor_x {
            Some(cx) => format!("Cursor: {:.1}d ago  (←→ move, ↑↓ profile)", 7.0 - cx),
            None => {
                let window_label = match (chart_state.x_lower * 10.0).round() as i32 {
                    0 => "7d",
                    40 => "3d",
                    60 => "1d",
                    _ => "?d",
                };
                format!("Window: {} · +/-=zoom · r=reset · 1/3/7=window", window_label)
            }
        }),
    ]))
    .wrap(Wrap { trim: true });
    frame.render_widget(band_summary, band_area);
}

fn render_usage_chart(frame: &mut Frame, area: ratatui::layout::Rect, chart_state: &ChartState<'_>) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let x_bounds = [chart_state.x_lower, 7.0_f64];
    let y_bounds = [chart_state.y_lower, chart_state.y_upper];

    let visible_series: Vec<&super::ChartSeries<'_>> = if chart_state.solo {
        chart_state.series.iter().filter(|s| s.style.is_selected).collect()
    } else {
        chart_state.series.iter().collect()
    };

    // Pre-compute 7d line points for each series.
    let series_points = visible_series
        .iter()
        .map(|series| {
            series
                .points
                .iter()
                .map(|point| (point.x, point.y.clamp(y_bounds[0], y_bounds[1])))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // Pre-compute per-series 5h subframe overlay points (lower and upper bounds).
    // Each entry: (color, lower_points, upper_points).
    let subframe_per_series: Vec<(Color, Vec<(f64, f64)>, Vec<(f64, f64)>)> = visible_series
        .iter()
        .filter_map(|series| {
            let sf = &series.five_hour_subframe;
            if !sf.available {
                return None;
            }
            let (start_x, end_x) = (sf.start_x?, sf.end_x?);
            let clamp_x = |v: f64| v.clamp(x_bounds[0], x_bounds[1]);
            let clamp_y = |v: f64| v.clamp(y_bounds[0], y_bounds[1]);
            let lower = sf.lower_y.map(|y| {
                vec![(clamp_x(start_x), clamp_y(y)), (clamp_x(end_x), clamp_y(y))]
            })?;
            let upper = sf.upper_y.map(|y| {
                vec![(clamp_x(start_x), clamp_y(y)), (clamp_x(end_x), clamp_y(y))]
            })?;
            let color = SERIES_COLORS[series.style.color_slot % SERIES_COLORS.len()];
            Some((color, lower, upper))
        })
        .collect();

    // 7d data lines (Dot marker).
    let mut datasets = visible_series
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

    // Per-series 5h subframe overlays (Braille marker, same color as series).
    for (color, lower_points, upper_points) in &subframe_per_series {
        datasets.push(
            Dataset::default()
                .name("")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(*color))
                .data(lower_points),
        );
        datasets.push(
            Dataset::default()
                .name("")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(*color))
                .data(upper_points),
        );
    }

    let cursor_points: Option<Vec<(f64, f64)>> = chart_state.cursor_x.map(|cx| {
        vec![(cx, y_bounds[0]), (cx, y_bounds[1])]
    });
    if let Some(ref pts) = cursor_points {
        datasets.push(
            Dataset::default()
                .name("")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::DarkGray))
                .data(pts),
        );
    }

    let x_mid = (x_bounds[0] + 7.0) / 2.0;
    let x_label_lo = format!("{:.1}d", x_bounds[0]);
    let x_label_mid = format!("{:.1}d", x_mid);
    let y_mid = (y_bounds[0] + y_bounds[1]) / 2.0;
    let y_label_lo = format!("{:.0}%", y_bounds[0]);
    let y_label_mid = format!("{:.0}%", y_mid);
    let y_label_hi = format!("{:.0}%", y_bounds[1]);
    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .title("7d window")
                .bounds(x_bounds)
                .labels([x_label_lo.as_str(), x_label_mid.as_str(), "now"]),
        )
        .y_axis(
            Axis::default()
                .title("usage%")
                .bounds(y_bounds)
                .labels([y_label_lo.as_str(), y_label_mid.as_str(), y_label_hi.as_str()]),
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
                    five_hour_subframe: FiveHourSubframeState {
                        available: true,
                        start_x: Some(5.0),
                        end_x: Some(6.0),
                        lower_y: Some(20.0),
                        upper_y: Some(35.0),
                        reason: None,
                    },
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
                y_lower: 0.0,
                y_upper: 100.0,
                x_lower: 0.0,
                solo: false,
                cursor_x: None,
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("5h band: 20.0%..35.0%"));
        assert!(joined.contains("Band drift: 7d +4.0% | 5h +1.5%"));
        assert!(joined.contains("0.0d"));
        assert!(joined.contains("3.5d"));
        assert!(joined.contains("now"));
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
                    five_hour_subframe: FiveHourSubframeState {
                        available: false,
                        start_x: None,
                        end_x: None,
                        lower_y: None,
                        upper_y: None,
                        reason: Some("insufficient 5h overlap"),
                    },
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
                y_lower: 0.0,
                y_upper: 100.0,
                x_lower: 0.0,
                solo: false,
                cursor_x: None,
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("Band drift: 7d ? | 5h ?"));
        assert!(!joined.contains("pending Canvas plot"));
    }
}
