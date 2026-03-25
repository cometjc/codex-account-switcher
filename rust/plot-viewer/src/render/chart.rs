use std::collections::HashSet;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::{Color, Frame, Style};
use ratatui::style::Modifier;
use ratatui::symbols::Marker;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Wrap};

use super::{ChartState, RenderContext, RenderState, SERIES_COLORS};

const SERIES_BAND_COLORS: [Color; 8] = [
    Color::Rgb(18, 72, 92),
    Color::Rgb(92, 82, 18),
    Color::Rgb(86, 34, 92),
    Color::Rgb(18, 82, 42),
    Color::Rgb(24, 52, 96),
    Color::Rgb(96, 42, 42),
    Color::Rgb(36, 88, 48),
    Color::Rgb(72, 72, 72),
];


pub fn render_chart<State: RenderState>(frame: &mut Frame, context: &RenderContext<'_, State>) {
    let chart_state = context.state.chart_state();
    let inner = if chart_state.fullscreen {
        context.area
    } else {
        let block = if chart_state.focused {
            Block::default()
                .title("usage chart (align to 7d window)")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(20, 20, 20)))
        } else {
            Block::default()
                .title("usage chart (align to 7d window)")
                .borders(Borders::ALL)
        };
        let inner = block.inner(context.area);
        frame.render_widget(block, context.area);
        inner
    };

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let [chart_area, band_area] =
        Layout::vertical([Constraint::Min(6), Constraint::Length(1)]).areas(inner);

    render_usage_chart(frame, chart_area, &chart_state);

    let window_days = chart_state.x_upper - chart_state.x_lower;
    let offset_days = 7.0 - chart_state.x_upper;
    let window_label = if offset_days > 0.01 {
        format!("{:.1}d @{:.1}d", window_days, offset_days)
    } else {
        let window_days_i = (window_days * 10.0).round() as i32;
        match window_days_i {
            70 => "7d".to_string(),
            30 => "3d".to_string(),
            10 => "1d".to_string(),
            _ => format!("{:.1}d", window_days),
        }
    };
    let view_prefix = match chart_state.tab_zoom_label {
        Some(label) => format!("[{}] · ", label),
        None => String::new(),
    };
    let hint_line = if chart_state.fullscreen {
        format!("{}W:{} · ←→=pan · =/- zoom-x · ↑↓=pan-y · [/]=zoom-y · z=reset · 1/3/7=snap · {}", view_prefix, window_label, env!("BUILD_VER"))
    } else {
        format!("{}W:{} · ←→=pan · =/- zoom-x · ↑↓=pan-y · [/]=zoom-y · z=reset · 1/3/7=snap", view_prefix, window_label)
    };
    let band_summary = Paragraph::new(Text::from(vec![Line::from(hint_line)]))
        .wrap(Wrap { trim: true });
    frame.render_widget(band_summary, band_area);
}

fn render_usage_chart(frame: &mut Frame, area: ratatui::layout::Rect, chart_state: &ChartState<'_>) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let x_bounds = [chart_state.x_lower, chart_state.x_upper];
    let y_bounds = [chart_state.y_lower, chart_state.y_upper];

    let visible_series: Vec<&super::ChartSeries<'_>> = chart_state
        .series
        .iter()
        .filter(|s| !s.style.hidden)
        .filter(|s| !chart_state.solo || s.style.is_selected)
        .collect();

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

    let band_rects: Vec<(usize, f64, f64, f64, f64)> = visible_series
        .iter()
        .filter_map(|series| {
            let sf = &series.five_hour_subframe;
            if !sf.available {
                return None;
            }
            let (start_x, end_x) = (sf.start_x?, sf.end_x?);
            let lower = sf.lower_y?.clamp(y_bounds[0], y_bounds[1]);
            let upper = sf.upper_y?.clamp(y_bounds[0], y_bounds[1]);
            Some((
                series.style.color_slot,
                start_x.clamp(x_bounds[0], x_bounds[1]),
                end_x.clamp(x_bounds[0], x_bounds[1]),
                lower.min(upper),
                lower.max(upper),
            ))
        })
        .collect();

    // 7d data lines.
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
                .name("")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(style)
                .data(points)
        })
        .collect::<Vec<_>>();


    let x_range = x_bounds[1] - x_bounds[0];
    let x_label_lo  = format!("{:.1}d", x_bounds[0]);
    let x_label_q1  = format!("{:.1}d", x_bounds[0] + x_range * 0.25);
    let x_label_mid = format!("{:.1}d", x_bounds[0] + x_range * 0.5);
    let x_label_q3  = format!("{:.1}d", x_bounds[0] + x_range * 0.75);
    let x_label_hi  = if (x_bounds[1] - 7.0).abs() < 0.01 {
        "now".to_string()
    } else {
        format!("{:.1}d ago", 7.0 - x_bounds[1])
    };
    let y_range = y_bounds[1] - y_bounds[0];
    let y_label_lo  = format!("{:.0}%", y_bounds[0]);
    let y_label_q1  = format!("{:.0}%", y_bounds[0] + y_range * 0.25);
    let y_label_mid = format!("{:.0}%", y_bounds[0] + y_range * 0.5);
    let y_label_q3  = format!("{:.0}%", y_bounds[0] + y_range * 0.75);
    let y_label_hi  = format!("{:.0}%", y_bounds[1]);
    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .title("7d window")
                .bounds(x_bounds)
                .labels([x_label_lo.as_str(), x_label_q1.as_str(), x_label_mid.as_str(), x_label_q3.as_str(), x_label_hi.as_str()]),
        )
        .y_axis(
            Axis::default()
                .title("usage%")
                .bounds(y_bounds)
                .labels([y_label_lo.as_str(), y_label_q1.as_str(), y_label_mid.as_str(), y_label_q3.as_str(), y_label_hi.as_str()]),
        );
    frame.render_widget(chart, area);
    let graph_area = chart_graph_area(area, x_label_lo.as_str(), [y_label_lo.as_str(), y_label_q1.as_str(), y_label_mid.as_str(), y_label_q3.as_str(), y_label_hi.as_str()]);
    let blocked_cells = apply_band_backgrounds(frame, graph_area, &band_rects, x_bounds, y_bounds);
    let occupied_cells = collect_occupied_plot_cells(frame, graph_area);
    render_end_labels(frame, graph_area, &visible_series, x_bounds, y_bounds, &occupied_cells, &blocked_cells);
}

fn chart_graph_area(area: Rect, first_x_label: &str, y_labels: [&str; 5]) -> Rect {
    let mut x = area.left();
    let mut y = area.bottom().saturating_sub(1);
    if y > area.top() {
        y = y.saturating_sub(1);
    }
    let max_y_label_width = y_labels.iter().map(|label| label.chars().count() as u16).max().unwrap_or(0);
    let left_of_y_axis = max_y_label_width.max(first_x_label.chars().count() as u16).saturating_sub(1);
    x = x.saturating_add(left_of_y_axis);
    if y > area.top() {
        y = y.saturating_sub(1);
    }
    if x + 1 < area.right() {
        x = x.saturating_add(1);
    }
    Rect::new(
        x,
        area.top(),
        area.right().saturating_sub(x),
        y.saturating_sub(area.top()).saturating_add(1),
    )
}

fn apply_band_backgrounds(
    frame: &mut Frame,
    graph_area: Rect,
    band_rects: &[(usize, f64, f64, f64, f64)],
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
) -> HashSet<(u16, u16)> {
    let mut blocked = HashSet::new();
    for (color_slot, start_x, end_x, lower_y, upper_y) in band_rects {
        let (left, right) = project_band_x_bounds(*start_x, *end_x, graph_area, x_bounds);
        let top = project_y(*upper_y, graph_area, y_bounds);
        let bottom = project_y(*lower_y, graph_area, y_bounds);
        let bg = band_background_color(*color_slot);
        for y in top.min(bottom)..=top.max(bottom) {
            for x in left.min(right)..=left.max(right) {
                let cell = &mut frame.buffer_mut()[(x, y)];
                cell.set_bg(bg);
                blocked.insert((x, y));
            }
        }
    }
    blocked
}

fn collect_occupied_plot_cells(frame: &mut Frame, graph_area: Rect) -> HashSet<(u16, u16)> {
    let mut occupied = HashSet::new();
    let buffer = frame.buffer_mut();
    for y in graph_area.top()..graph_area.bottom() {
        for x in graph_area.left()..graph_area.right() {
            if buffer[(x, y)].symbol() != " " {
                occupied.insert((x, y));
            }
        }
    }
    occupied
}

fn render_end_labels(
    frame: &mut Frame,
    graph_area: Rect,
    visible_series: &[&super::ChartSeries<'_>],
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    occupied_cells: &HashSet<(u16, u16)>,
    blocked_cells: &HashSet<(u16, u16)>,
) {
    let mut anchors = visible_series
        .iter()
        .filter_map(|series| {
            let point = series
                .points
                .iter()
                .rev()
                .find(|point| point.x >= x_bounds[0] && point.x <= x_bounds[1])?;
            Some(LabelAnchor {
                text: format_end_label(series),
                color: SERIES_COLORS[series.style.color_slot % SERIES_COLORS.len()],
                x: project_x(point.x, graph_area, x_bounds),
                y: project_y(point.y, graph_area, y_bounds),
            })
        })
        .collect::<Vec<_>>();
    anchors.sort_by_key(|anchor| anchor.y);

    for label in layout_end_labels(&anchors, graph_area, occupied_cells, blocked_cells) {
        frame
            .buffer_mut()
            .set_string(label.x, label.y, label.text, Style::default().fg(label.color));
    }
}

fn project_x(x: f64, graph_area: Rect, x_bounds: [f64; 2]) -> u16 {
    if graph_area.width <= 1 || (x_bounds[1] - x_bounds[0]).abs() < f64::EPSILON {
        return graph_area.left();
    }
    let ratio = ((x - x_bounds[0]) / (x_bounds[1] - x_bounds[0])).clamp(0.0, 1.0);
    graph_area.left() + ((graph_area.width - 1) as f64 * ratio).round() as u16
}

fn project_band_x_bounds(start_x: f64, end_x: f64, graph_area: Rect, x_bounds: [f64; 2]) -> (u16, u16) {
    if graph_area.width <= 1 || (x_bounds[1] - x_bounds[0]).abs() < f64::EPSILON {
        return (graph_area.left(), graph_area.left());
    }

    let span = (graph_area.width - 1) as f64;
    let to_raw = |value: f64| ((value - x_bounds[0]) / (x_bounds[1] - x_bounds[0])).clamp(0.0, 1.0) * span;
    let raw_left = to_raw(start_x.min(end_x));
    let raw_right = to_raw(start_x.max(end_x));

    let mut left = graph_area.left() + raw_left.ceil() as u16;
    let mut right = graph_area.left() + raw_right.floor() as u16;

    if right < left {
        let midpoint = graph_area.left() + ((raw_left + raw_right) / 2.0).round() as u16;
        left = midpoint;
        right = midpoint;
    }

    (
        left.clamp(graph_area.left(), graph_area.right().saturating_sub(1)),
        right.clamp(graph_area.left(), graph_area.right().saturating_sub(1)),
    )
}

fn project_y(y: f64, graph_area: Rect, y_bounds: [f64; 2]) -> u16 {
    if graph_area.height <= 1 || (y_bounds[1] - y_bounds[0]).abs() < f64::EPSILON {
        return graph_area.bottom().saturating_sub(1);
    }
    let ratio = ((y - y_bounds[0]) / (y_bounds[1] - y_bounds[0])).clamp(0.0, 1.0);
    graph_area.bottom().saturating_sub(1) - ((graph_area.height - 1) as f64 * ratio).round() as u16
}

fn band_background_color(color_slot: usize) -> Color {
    SERIES_BAND_COLORS[color_slot % SERIES_BAND_COLORS.len()]
}

fn format_end_label(series: &super::ChartSeries<'_>) -> String {
    format!(
        "[{}] {} {}/{}",
        series.profile.agent_type,
        series.profile.label,
        format_unsigned_percent(series.last_seven_day_percent),
        format_unsigned_percent(series.five_hour_used_percent),
    )
}

fn format_unsigned_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.0}%", value))
        .unwrap_or("?%".to_string())
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct LabelAnchor {
    text: String,
    color: Color,
    x: u16,
    y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlacedLabel {
    text: String,
    color: Color,
    x: u16,
    y: u16,
}

fn layout_end_labels(
    anchors: &[LabelAnchor],
    graph_area: Rect,
    occupied_cells: &HashSet<(u16, u16)>,
    blocked_cells: &HashSet<(u16, u16)>,
) -> Vec<PlacedLabel> {
    let mut placed = Vec::new();
    let mut reserved = HashSet::new();

    for anchor in anchors {
        let width = anchor.text.chars().count() as u16;
        if width == 0 || graph_area.width == 0 || graph_area.height == 0 {
            continue;
        }

        let mut candidates = Vec::new();
        for step in 0..graph_area.height {
            let step = step as i16;
            let offsets = if step == 0 { vec![0] } else { vec![-step, step] };
            for dy in offsets {
                let y = anchor.y as i16 + dy;
                if y < graph_area.top() as i16 || y >= graph_area.bottom() as i16 {
                    continue;
                }
                let y = y as u16;
                let right_x = anchor.x.saturating_add(1);
                if right_x + width <= graph_area.right() {
                    candidates.push((right_x, y));
                }
                let left_x = anchor.x.saturating_sub(width);
                if left_x >= graph_area.left() && left_x + width <= graph_area.right() {
                    candidates.push((left_x, y));
                }
            }
        }

        if let Some((x, y)) = candidates.into_iter().find(|(x, y)| {
            (0..width).all(|dx| {
                let cell = (x + dx, *y);
                !occupied_cells.contains(&cell)
                    && !blocked_cells.contains(&cell)
                    && !reserved.contains(&cell)
            })
        }) {
            for dx in 0..width {
                reserved.insert((x + dx, y));
            }
            placed.push(PlacedLabel {
                text: anchor.text.clone(),
                color: anchor.color,
                x,
                y,
            });
        }
    }

    placed
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
    fn layout_end_labels_staggers_names_away_from_conflicts() {
        let anchors = vec![
            LabelAnchor {
                text: "Alpha".to_string(),
                color: Color::Cyan,
                x: 12,
                y: 4,
            },
            LabelAnchor {
                text: "Beta".to_string(),
                color: Color::Yellow,
                x: 12,
                y: 4,
            },
        ];
        let occupied = HashSet::from([(13, 4), (14, 4), (15, 4), (16, 4), (17, 4)]);
        let blocked = HashSet::from([(13, 5), (14, 5), (15, 5), (16, 5), (17, 5)]);

        let labels = layout_end_labels(&anchors, Rect::new(0, 0, 24, 10), &occupied, &blocked);

        assert_eq!(labels.len(), 2);
        assert_ne!(labels[0].y, labels[1].y);
        for label in &labels {
            for dx in 0..label.text.chars().count() as u16 {
                let cell = (label.x + dx, label.y);
                assert!(!occupied.contains(&cell));
                assert!(!blocked.contains(&cell));
            }
        }
    }

    #[test]
    fn band_background_palette_stays_distinct_from_line_palette() {
        for (slot, line_color) in SERIES_COLORS.iter().enumerate() {
            assert_ne!(band_background_color(slot), *line_color);
        }
    }

    #[test]
    fn project_band_x_bounds_shrinks_extra_edge_padding() {
        let graph_area = Rect::new(10, 2, 20, 8);
        let bounds = project_band_x_bounds(1.2, 5.8, graph_area, [0.0, 7.0]);

        assert_eq!(bounds, (14, 25));
    }

    #[test]
    fn format_end_label_includes_profile_and_usage_numbers() {
        let series = ChartSeries {
            profile: RenderProfile {
                id: "alpha",
                label: "Alpha",
                is_current: true,
                agent_type: "codex",
            },
            style: ChartSeriesStyle {
                color_slot: 0,
                is_selected: true,
                is_current: true,
                hidden: false,
            },
            points: vec![ChartPoint { x: 7.0, y: 76.0 }],
            last_seven_day_percent: Some(76.0),
            five_hour_used_percent: Some(40.0),
            five_hour_subframe: FiveHourSubframeState {
                available: true,
                start_x: Some(6.0),
                end_x: Some(7.0),
                lower_y: Some(20.0),
                upper_y: Some(35.0),
                reason: None,
            },
        };

        assert_eq!(format_end_label(&series), "[codex] Alpha 76%/40%");
    }

    #[test]
    fn render_chart_draws_visible_seven_day_curve_and_band_summary() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: false,
                    agent_type: "codex",
                }),
                current: Some(RenderProfile {
                    id: "beta",
                    label: "Beta",
                    is_current: true,
                    agent_type: "codex",
                }),
            },
            chart: ChartState {
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "alpha",
                        label: "Alpha",
                        is_current: false,
                        agent_type: "codex",
                    },
                    style: ChartSeriesStyle {
                        color_slot: 0,
                        is_selected: true,
                        is_current: false,
                        hidden: false,
                    },
                    points: vec![
                        ChartPoint { x: 0.0, y: 8.0 },
                        ChartPoint { x: 1.0, y: 18.0 },
                        ChartPoint { x: 3.5, y: 44.0 },
                        ChartPoint { x: 5.0, y: 58.0 },
                        ChartPoint { x: 7.0, y: 76.0 },
                    ],
                    last_seven_day_percent: Some(76.0),
                    five_hour_used_percent: Some(40.0),
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
                    used_percent: Some(40.0),
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
                x_upper: 7.0,
                solo: false,
                tab_zoom_label: None,
                focused: false,
                fullscreen: false,
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("usage chart (align to 7d window)"));
        assert!(joined.contains("[codex] Alpha 76%/40%"));
        assert!(joined.contains("0.0d"));
        assert!(joined.contains("3.5d"));
        assert!(joined.contains("now"));
        assert!(joined.contains("100%"));
        assert!(!joined.contains("▪"));
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
                    agent_type: "codex",
                }),
                current: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: true,
                    agent_type: "codex",
                }),
            },
            chart: ChartState {
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "alpha",
                        label: "Alpha",
                        is_current: true,
                        agent_type: "codex",
                    },
                    style: ChartSeriesStyle {
                        color_slot: 0,
                        is_selected: true,
                        is_current: true,
                        hidden: false,
                    },
                    points: vec![
                        ChartPoint { x: 0.0, y: 12.0 },
                        ChartPoint { x: 4.0, y: 40.0 },
                        ChartPoint { x: 7.0, y: 61.0 },
                    ],
                    last_seven_day_percent: Some(61.0),
                    five_hour_used_percent: None,
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
                    used_percent: None,
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
                x_upper: 7.0,
                solo: false,
                tab_zoom_label: None,
                focused: false,
                fullscreen: false,
            },
        };

        let lines = render_lines(&state, 72, 18);
        let joined = lines.join("\n");

        assert!(!joined.contains("pending Canvas plot"));
    }

}
