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

const LABEL_BG_COLOR: Color = Color::Rgb(20, 20, 20);

pub fn render_chart<State: RenderState>(frame: &mut Frame, context: &RenderContext<'_, State>) {
    let chart_state = context.state.chart_state();
    let inner = if chart_state.fullscreen {
        context.area
    } else {
        let block = if chart_state.focused {
            Block::default()
                .title("Usage chart")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(20, 20, 20)))
        } else {
            Block::default()
                .title("Usage chart")
                .borders(Borders::ALL)
        };
        let inner = block.inner(context.area);
        frame.render_widget(block, context.area);
        inner
    };

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Reserve the hint row first so it never collapses to 0 when vertical space is tight.
    let [chart_area, band_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);

    render_usage_chart(frame, chart_area, &chart_state);

    let window_days = chart_state.x_upper - chart_state.x_lower;
    let offset_days = 7.0 - chart_state.x_upper;
    let window_percent = ((window_days / 7.0) * 100.0).round() as i32;
    let offset_percent = ((offset_days / 7.0) * 100.0).round() as i32;
    let window_label = if offset_percent > 0 {
        format!("{window_percent}% @{offset_percent}%")
    } else {
        format!("{window_percent}%")
    };
    let view_prefix = match chart_state.tab_zoom_label {
        Some(label) => format!("[{}] · ", label),
        None => String::new(),
    };
    let hint_line = format!(
        "{}Window view:{} · ←→=pan · =/- zoom-x · ↑↓=pan-y · [/]=zoom-y · z=reset · 1/3/7=snap",
        view_prefix, window_label
    );
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

    // Pre-compute normalized quota-window line points for each series.
    // Zero-state series render as a single point at the actual chart origin
    // instead of their full observation history (which would draw a flat line at y=0).
    let series_points = visible_series
        .iter()
        .map(|series| {
            if series.is_zero_state {
                return vec![(0.0, 0.0_f64.clamp(y_bounds[0], y_bounds[1]))];
            }
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
    let datasets = visible_series
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
    let format_x_label = |value: f64| {
        let percent = ((value / 7.0) * 100.0).round() as i32;
        match percent {
            0 => "start".to_string(),
            100 => "reset".to_string(),
            value => format!("{value}%"),
        }
    };
    let x_label_lo  = format_x_label(x_bounds[0]);
    let x_label_q1  = format_x_label(x_bounds[0] + x_range * 0.25);
    let x_label_mid = format_x_label(x_bounds[0] + x_range * 0.5);
    let x_label_q3  = format_x_label(x_bounds[0] + x_range * 0.75);
    let x_label_hi  = format_x_label(x_bounds[1]);
    let y_range = y_bounds[1] - y_bounds[0];
    let y_label_lo  = format!("{:.0}%", y_bounds[0]);
    let y_label_q1  = format!("{:.0}%", y_bounds[0] + y_range * 0.25);
    let y_label_mid = format!("{:.0}%", y_bounds[0] + y_range * 0.5);
    let y_label_q3  = format!("{:.0}%", y_bounds[0] + y_range * 0.75);
    let y_label_hi  = format!("{:.0}%", y_bounds[1]);
    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .title("window-relative")
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
    let zero_state_series = visible_series
        .iter()
        .copied()
        .filter(|series| series.is_zero_state)
        .collect::<Vec<_>>();
    let normal_series = visible_series
        .iter()
        .copied()
        .filter(|series| !series.is_zero_state)
        .collect::<Vec<_>>();

    if !zero_state_series.is_empty() {
        render_zero_state_labels(
            frame,
            graph_area,
            x_bounds,
            y_bounds,
            &zero_state_series,
            &occupied_cells,
            &blocked_cells,
        );
    }

    let occupied_cells = collect_occupied_plot_cells(frame, graph_area);
    render_end_labels(frame, graph_area, &normal_series, x_bounds, y_bounds, &occupied_cells, &blocked_cells);
    render_zero_state_origin_marker(frame, graph_area, x_bounds, y_bounds, &zero_state_series);
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
    let y_span = y_bounds[1] - y_bounds[0];
    let denom = graph_area.height.saturating_sub(1).max(1) as f64;
    let min_band_dy = if y_span.abs() < f64::EPSILON {
        1.0
    } else {
        y_span / denom
    };

    let mut blocked = HashSet::new();
    for &(color_slot, start_x, end_x, y_min, y_max) in band_rects {
        let (mut y_lo, mut y_hi) = (y_min.min(y_max), y_min.max(y_max));
        if y_hi - y_lo < min_band_dy {
            let mid = (y_lo + y_hi) / 2.0;
            y_lo = mid - min_band_dy / 2.0;
            y_hi = mid + min_band_dy / 2.0;
            y_lo = y_lo.clamp(y_bounds[0], y_bounds[1]);
            y_hi = y_hi.clamp(y_bounds[0], y_bounds[1]);
            if y_hi - y_lo < min_band_dy {
                y_hi = (y_lo + min_band_dy).min(y_bounds[1]);
                if y_hi - y_lo < min_band_dy {
                    y_lo = (y_hi - min_band_dy).max(y_bounds[0]);
                }
            }
        }

        let (left, right) = project_band_x_bounds(start_x, end_x, graph_area, x_bounds);
        let top = project_y(y_hi, graph_area, y_bounds);
        let bottom = project_y(y_lo, graph_area, y_bounds);
        let bg = band_background_color(color_slot);
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
                fallback_texts: compact_end_label_variants(series),
                color: SERIES_COLORS[series.style.color_slot % SERIES_COLORS.len()],
                x: project_x(point.x, graph_area, x_bounds),
                y: project_y(point.y, graph_area, y_bounds),
            })
        })
        .collect::<Vec<_>>();
    anchors.sort_by_key(|anchor| anchor.y);

    for label in layout_end_labels(&anchors, graph_area, occupied_cells, blocked_cells) {
        draw_label_connector(frame, &label, graph_area);
        let label_width = label.text.chars().count() as u16;
        for dx in 0..label_width {
            frame
                .buffer_mut()[(label.x + dx, label.y)]
                .set_symbol(" ")
                .set_bg(LABEL_BG_COLOR);
        }
        frame.buffer_mut().set_string(
            label.x,
            label.y,
            label.text,
            Style::default().fg(label.color).bg(LABEL_BG_COLOR),
        );
    }
}

fn render_zero_state_labels(
    frame: &mut Frame,
    graph_area: Rect,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    zero_state_series: &[&super::ChartSeries<'_>],
    _occupied_cells: &HashSet<(u16, u16)>,
    _blocked_cells: &HashSet<(u16, u16)>,
) {
    if zero_state_series.is_empty() || graph_area.width == 0 || graph_area.height == 0 {
        return;
    }

    let origin_x = project_x(0.0, graph_area, x_bounds);
    let origin_y = project_y(0.0, graph_area, y_bounds);
    if origin_y <= graph_area.top() {
        return;
    }

    let branch_rows = zero_state_branch_rows(origin_y, graph_area, zero_state_series.len());
    for ((series, row), branch_kind) in zero_state_series
        .iter()
        .zip(branch_rows.iter())
        .zip(zero_state_branch_kinds(zero_state_series.len()).iter())
    {
        let label = zero_state_branch_text(series, zero_state_series.len());
        let branch_style = Style::default()
            .fg(SERIES_COLORS[series.style.color_slot % SERIES_COLORS.len()])
            .bg(LABEL_BG_COLOR)
            .add_modifier(Modifier::BOLD);
        let label_x = origin_x.saturating_add(3);
        let available = graph_area.right().saturating_sub(label_x) as usize;
        if available == 0 {
            continue;
        }

        frame.buffer_mut()[(origin_x, *row)]
            .set_symbol(branch_kind.symbol())
            .set_style(branch_style);

        if origin_x.saturating_add(1) < graph_area.right() {
            frame.buffer_mut()[(origin_x.saturating_add(1), *row)]
                .set_symbol("─")
                .set_style(branch_style);
        }

        if origin_x.saturating_add(2) < graph_area.right() {
            frame.buffer_mut()[(origin_x.saturating_add(2), *row)]
                .set_symbol(" ")
                .set_bg(LABEL_BG_COLOR);
        }

        let clipped = label.chars().take(available).collect::<String>();
        frame.buffer_mut().set_string(label_x, *row, clipped, branch_style);
    }
}

fn zero_state_branch_rows(origin_y: u16, graph_area: Rect, count: usize) -> Vec<u16> {
    if count == 0 || origin_y == graph_area.top() {
        return Vec::new();
    }

    let available = origin_y.saturating_sub(graph_area.top()) as usize;
    let visible_count = count.min(available);
    let start_y = origin_y.saturating_sub(visible_count as u16);
    (0..visible_count)
        .map(|offset| start_y.saturating_add(offset as u16))
        .collect()
}

fn zero_state_branch_text(series: &super::ChartSeries<'_>, branch_count: usize) -> String {
    if branch_count == 1 {
        format!(
            "[{}] {} reset / no usage yet",
            series.profile.agent_type, series.profile.label
        )
    } else {
        format!("[{}] {}", series.profile.agent_type, series.profile.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZeroStateBranchKind {
    First,
    Continuing,
}

impl ZeroStateBranchKind {
    fn symbol(self) -> &'static str {
        match self {
            Self::First => "┌",
            Self::Continuing => "├",
        }
    }
}

fn zero_state_branch_kinds(count: usize) -> Vec<ZeroStateBranchKind> {
    (0..count)
        .map(|index| {
            if index == 0 {
                ZeroStateBranchKind::First
            } else {
                ZeroStateBranchKind::Continuing
            }
        })
        .collect()
}

fn render_zero_state_origin_marker(
    frame: &mut Frame,
    graph_area: Rect,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    zero_state_series: &[&super::ChartSeries<'_>],
) {
    if zero_state_series.is_empty()
        || graph_area.width == 0
        || graph_area.height == 0
        || !(x_bounds[0] <= 0.0 && 0.0 <= x_bounds[1])
        || !(y_bounds[0] <= 0.0 && 0.0 <= y_bounds[1])
    {
        return;
    }

    let marker_series = zero_state_series
        .iter()
        .copied()
        .find(|series| series.style.is_selected)
        .unwrap_or(zero_state_series[0]);
    let marker_x = project_x(0.0, graph_area, x_bounds);
    let marker_y = project_y(0.0, graph_area, y_bounds);
    frame.buffer_mut()[(marker_x, marker_y)].set_symbol("•").set_style(
        Style::default().fg(SERIES_COLORS[marker_series.style.color_slot % SERIES_COLORS.len()]),
    );
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
    let weekly = format_unsigned_percent(series.last_seven_day_percent);
    let base = if series.profile.agent_type == "copilot" {
        format!(
            "[{} {}] {} {}",
            series.profile.agent_type, series.profile.window_label, series.profile.label, weekly
        )
    } else {
        format!(
            "[{} {}] {} {}/{}",
            series.profile.agent_type,
            series.profile.window_label,
            series.profile.label,
            weekly,
            format_unsigned_percent(series.five_hour_used_percent),
        )
    };
    match series.forecast_label {
        Some(forecast) => format!("{base} {forecast}"),
        None => base,
    }
}

fn format_unsigned_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{:.0}%", value))
        .unwrap_or("?%".to_string())
}

fn compact_end_label_variants(series: &super::ChartSeries<'_>) -> Vec<String> {
    let compact = format!(
        "{} {}",
        series.profile.label,
        format_unsigned_percent(series.last_seven_day_percent),
    );
    let minimal = series.profile.label.to_string();
    let mut variants = Vec::new();
    for text in [compact, minimal] {
        if !variants.iter().any(|existing| existing == &text) {
            variants.push(text);
        }
    }
    variants
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct LabelAnchor {
    text: String,
    fallback_texts: Vec<String>,
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
    anchor_x: u16,
    anchor_y: u16,
    attach_x: u16,
}

const PREFERRED_LABEL_OFFSET: u16 = 3;
const FALLBACK_LABEL_OFFSET: u16 = 1;

fn placement_candidates_for_variant(
    anchor: &LabelAnchor,
    text: &str,
    graph_area: Rect,
) -> Vec<(u16, u16)> {
    let width = text.chars().count() as u16;
    if width == 0 || graph_area.width == 0 || graph_area.height == 0 {
        return vec![];
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
            for offset in [PREFERRED_LABEL_OFFSET, FALLBACK_LABEL_OFFSET] {
                let right_x = anchor.x.saturating_add(offset);
                if right_x + width <= graph_area.right() {
                    candidates.push((right_x, y));
                }
                let left_x = anchor
                    .x
                    .saturating_sub(width.saturating_add(offset.saturating_sub(1)))
                    .max(graph_area.left());
                if left_x >= graph_area.left() && left_x + width <= graph_area.right() {
                    candidates.push((left_x, y));
                }
            }
        }
    }
    candidates
}

fn best_safe_placement_for_variant(
    anchor: &LabelAnchor,
    text: &str,
    graph_area: Rect,
    occupied_cells: &HashSet<(u16, u16)>,
    label_exclusion_cells: &HashSet<(u16, u16)>,
    blocked_cells: &HashSet<(u16, u16)>,
    reserved: &HashSet<(u16, u16)>,
) -> Option<(u16, u16, u16)> {
    let width = text.chars().count() as u16;
    if width == 0 {
        return None;
    }

    let candidates = placement_candidates_for_variant(anchor, text, graph_area);
    let mut best: Option<(u16, u16, u16, u16, u16)> = None;

    for (x, y) in candidates {
        let attach_x = connector_attach_x(x, width, anchor.x);
        let label_cells_ok = (0..width).all(|dx| {
            let cell = (x + dx, y);
            !occupied_cells.contains(&cell)
                && !label_exclusion_cells.contains(&cell)
                && !reserved.contains(&cell)
        });
        if !label_cells_ok {
            continue;
        }
        if !connector_cells(attach_x, y, anchor.x, anchor.y, graph_area)
            .into_iter()
            .all(|cell| !blocked_cells.contains(&cell) && !reserved.contains(&cell))
        {
            continue;
        }

        let dy = y.abs_diff(anchor.y);
        let dx = attach_x.abs_diff(anchor.x);
        if best
            .as_ref()
            .is_none_or(|(_, _, _, best_dy, best_dx)| (dy, dx) < (*best_dy, *best_dx))
        {
            best = Some((x, y, attach_x, dy, dx));
        }
    }

    best.map(|(x, y, attach_x, _, _)| (x, y, attach_x))
}

fn layout_end_labels(
    anchors: &[LabelAnchor],
    graph_area: Rect,
    occupied_cells: &HashSet<(u16, u16)>,
    blocked_cells: &HashSet<(u16, u16)>,
) -> Vec<PlacedLabel> {
    let mut placed = Vec::new();
    let mut placed_anchor_indices = HashSet::new();
    let mut reserved = HashSet::new();
    let label_exclusion_cells = expand_label_exclusion_cells(blocked_cells, graph_area);

    for (anchor_idx, anchor) in anchors.iter().enumerate() {
        let variants = std::iter::once(&anchor.text).chain(anchor.fallback_texts.iter());
        let mut best: Option<(u16, u16, u16, &String)> = None;

        for variant_text in variants {
            if let Some((x, y, attach_x)) = best_safe_placement_for_variant(
                anchor,
                variant_text,
                graph_area,
                occupied_cells,
                &label_exclusion_cells,
                blocked_cells,
                &reserved,
            ) {
                best = Some((x, y, attach_x, variant_text));
                break;
            }
        }

        if let Some((x, y, attach_x, text)) = best {
            let width = text.chars().count() as u16;
            let connector = connector_cells(attach_x, y, anchor.x, anchor.y, graph_area);
            for cell in connector {
                reserved.insert(cell);
            }
            for dx in 0..width {
                reserved.insert((x + dx, y));
            }
            placed.push(PlacedLabel {
                text: text.clone(),
                color: anchor.color,
                x,
                y,
                anchor_x: anchor.x,
                anchor_y: anchor.y,
                attach_x,
            });
            placed_anchor_indices.insert(anchor_idx);
        }
    }

    for (anchor_idx, anchor) in anchors.iter().enumerate() {
        if placed_anchor_indices.contains(&anchor_idx) {
            continue;
        }

        let force_variants = std::iter::once(&anchor.text).chain(anchor.fallback_texts.iter());
        let mut best: Option<(u16, u16, u16, &String, usize, u16, u16)> = None;

        for (variant_idx, text) in force_variants.enumerate() {
            let width = text.chars().count() as u16;
            if width == 0 || width > graph_area.width {
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
                    for offset in [FALLBACK_LABEL_OFFSET, PREFERRED_LABEL_OFFSET] {
                        let right_x = anchor
                            .x
                            .saturating_add(offset)
                            .min(graph_area.right().saturating_sub(width));
                        if right_x + width <= graph_area.right() {
                            candidates.push((right_x, y));
                        }
                        let left_x = anchor
                            .x
                            .saturating_sub(width.saturating_add(offset.saturating_sub(1)))
                            .max(graph_area.left());
                        if left_x + width <= graph_area.right() {
                            candidates.push((left_x, y));
                        }
                    }
                }
            }

            for (x, y) in candidates {
                if !(0..width).all(|dx| {
                    let cell = (x + dx, y);
                    !reserved.contains(&cell) && !label_exclusion_cells.contains(&cell)
                }) {
                    continue;
                }
                let attach_x = connector_attach_x(x, width, anchor.x);
                let dy = y.abs_diff(anchor.y);
                let dx = attach_x.abs_diff(anchor.x);
                let candidate = (x, y, attach_x, text, variant_idx, dy, dx);
                if best
                    .as_ref()
                    .is_none_or(|(_, _, _, _, best_variant, best_dy, best_dx)| {
                        (dy, dx, variant_idx) < (*best_dy, *best_dx, *best_variant)
                    })
                {
                    best = Some(candidate);
                }
            }
        }

        if let Some((x, y, attach_x, text, _, _, _)) = best {
            let width = text.chars().count() as u16;
            for dx in 0..width {
                reserved.insert((x + dx, y));
            }
            placed.push(PlacedLabel {
                text: text.clone(),
                color: anchor.color,
                x,
                y,
                anchor_x: anchor.x,
                anchor_y: anchor.y,
                attach_x,
            });
            placed_anchor_indices.insert(anchor_idx);
        }
    }

    placed
}

fn expand_label_exclusion_cells(
    blocked_cells: &HashSet<(u16, u16)>,
    graph_area: Rect,
) -> HashSet<(u16, u16)> {
    let mut expanded = HashSet::new();
    for &(x, y) in blocked_cells {
        for dx in -1i16..=1 {
            for dy in -1i16..=1 {
                let expanded_x = x as i16 + dx;
                let expanded_y = y as i16 + dy;
                if expanded_x < graph_area.left() as i16
                    || expanded_x >= graph_area.right() as i16
                    || expanded_y < graph_area.top() as i16
                    || expanded_y >= graph_area.bottom() as i16
                {
                    continue;
                }
                expanded.insert((expanded_x as u16, expanded_y as u16));
            }
        }
    }
    expanded
}

fn connector_cells(
    attach_x: u16,
    label_y: u16,
    anchor_x: u16,
    anchor_y: u16,
    graph_area: Rect,
) -> Vec<(u16, u16)> {
    let mut cells = Vec::new();
    for x in anchor_x.min(attach_x)..=anchor_x.max(attach_x) {
        if x == anchor_x || x < graph_area.left() || x >= graph_area.right() {
            continue;
        }
        cells.push((x, anchor_y));
    }
    for y in anchor_y.min(label_y)..=anchor_y.max(label_y) {
        if y == anchor_y || y == label_y || y < graph_area.top() || y >= graph_area.bottom() {
            continue;
        }
        cells.push((attach_x, y));
    }
    cells
}

fn draw_label_connector(frame: &mut Frame, label: &PlacedLabel, graph_area: Rect) {
    let style = Style::default().fg(label.color).add_modifier(Modifier::DIM);

    for x in label.anchor_x.min(label.attach_x)..=label.anchor_x.max(label.attach_x) {
        if x == label.anchor_x || x < graph_area.left() || x >= graph_area.right() {
            continue;
        }
        let cell = &mut frame.buffer_mut()[(x, label.anchor_y)];
        if cell.symbol() == " " {
            cell.set_symbol("-").set_style(style);
        }
    }
    for y in label.anchor_y.min(label.y)..=label.anchor_y.max(label.y) {
        if y == label.anchor_y || y < graph_area.top() || y >= graph_area.bottom() {
            continue;
        }
        let cell = &mut frame.buffer_mut()[(label.attach_x, y)];
        if cell.symbol() == " " {
            cell.set_symbol("|").set_style(style);
        }
    }
}

fn connector_attach_x(label_x: u16, label_width: u16, anchor_x: u16) -> u16 {
    let end_x = label_x.saturating_add(label_width.saturating_sub(1));
    anchor_x.clamp(label_x, end_x)
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
                fallback_texts: vec![],
                color: Color::Cyan,
                x: 12,
                y: 4,
            },
            LabelAnchor {
                text: "Beta".to_string(),
                fallback_texts: vec![],
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
    fn layout_end_labels_prefers_full_variant_over_closer_compact_slot() {
        let anchors = vec![LabelAnchor {
            text: "FULLFULL".to_string(),
            fallback_texts: vec!["MID".to_string(), "M".to_string()],
            color: Color::Cyan,
            x: 8,
            y: 3,
        }];

        let occupied = HashSet::from([(0, 3), (9, 3)]);

        let labels = layout_end_labels(&anchors, Rect::new(0, 3, 20, 1), &occupied, &HashSet::new());

        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].text, "FULLFULL");
        assert_eq!(labels[0].x, 11);
    }

    #[test]
    fn layout_end_labels_preserves_full_compact_minimal_chain() {
        let anchor = LabelAnchor {
            text: "FULLFULL".to_string(),
            fallback_texts: vec!["MID".to_string(), "M".to_string()],
            color: Color::Yellow,
            x: 8,
            y: 3,
        };

        let cases = [
            (HashSet::from([(0, 3)]), "FULLFULL"),
            (HashSet::from([(0, 3), (9, 3), (11, 3)]), "MID"),
            (HashSet::from([(0, 3), (5, 3), (9, 3), (11, 3)]), "M"),
        ];

        for (occupied, expected) in cases {
            let labels = layout_end_labels(&[anchor.clone()], Rect::new(0, 3, 20, 1), &occupied, &HashSet::new());
            assert_eq!(labels.len(), 1);
            assert_eq!(labels[0].text, expected);
        }
    }

    #[test]
    fn layout_end_labels_clamps_left_edge_instead_of_dropping_label() {
        let anchors = vec![LabelAnchor {
            text: "[codex 7d] comet.jc 7%/0%".to_string(),
            fallback_texts: vec!["comet.jc 7%".to_string(), "comet.jc".to_string()],
            color: Color::Cyan,
            x: 10,
            y: 9,
        }];

        let occupied = HashSet::from([
            (13, 9), (14, 9), (15, 9), (16, 9), (17, 9), (18, 9), (19, 9), (20, 9),
            (21, 9), (22, 9), (23, 9), (24, 9), (25, 9), (26, 9), (27, 9), (28, 9),
        ]);
        let labels = layout_end_labels(&anchors, Rect::new(8, 0, 32, 10), &occupied, &HashSet::new());

        assert_eq!(labels.len(), 1, "label should still render when left-edge clamping is required");
        assert!(labels[0].x >= 8);
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
            window_label: "7d",
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
            forecast_label: None,
            five_hour_subframe: FiveHourSubframeState {
                available: true,
                start_x: Some(6.0),
                end_x: Some(7.0),
                lower_y: Some(20.0),
                upper_y: Some(35.0),
                reason: None,
            },
            is_zero_state: false,
        };

        assert_eq!(format_end_label(&series), "[codex 7d] Alpha 76%/40%");
    }

    #[test]
    fn format_end_label_omits_five_hour_for_copilot() {
        let series = ChartSeries {
            profile: RenderProfile {
                id: "team",
                label: "teamt5-it",
                is_current: false,
                agent_type: "copilot",
                window_label: "30d",
            },
            style: ChartSeriesStyle {
                color_slot: 1,
                is_selected: false,
                is_current: false,
                hidden: false,
            },
            points: vec![ChartPoint { x: 7.0, y: 70.0 }],
            last_seven_day_percent: Some(70.0),
            five_hour_used_percent: Some(25.0),
            forecast_label: None,
            five_hour_subframe: FiveHourSubframeState {
                available: false,
                start_x: None,
                end_x: None,
                lower_y: None,
                upper_y: None,
                reason: Some("no 5h window"),
            },
            is_zero_state: false,
        };

        assert_eq!(format_end_label(&series), "[copilot 30d] teamt5-it 70%");
    }

    #[test]
    fn format_end_label_appends_forecast_label_when_available() {
        let series = ChartSeries {
            profile: RenderProfile {
                id: "cc",
                label: "CC",
                is_current: false,
                agent_type: "claude",
                window_label: "7d",
            },
            style: ChartSeriesStyle {
                color_slot: 2,
                is_selected: false,
                is_current: false,
                hidden: false,
            },
            points: vec![ChartPoint { x: 7.0, y: 46.0 }],
            last_seven_day_percent: Some(46.0),
            five_hour_used_percent: Some(16.0),
            forecast_label: Some("reset 3.5h"),
            five_hour_subframe: FiveHourSubframeState {
                available: true,
                start_x: Some(6.7),
                end_x: Some(7.0),
                lower_y: Some(44.0),
                upper_y: Some(46.0),
                reason: None,
            },
            is_zero_state: false,
        };

        assert_eq!(format_end_label(&series), "[claude 7d] CC 46%/16% reset 3.5h");
        assert_eq!(compact_end_label_variants(&series), vec!["CC 46%".to_string(), "CC".to_string()]);
    }

    #[test]
    fn format_end_label_appends_forecast_for_copilot_without_five_hour_suffix() {
        let series = ChartSeries {
            profile: RenderProfile {
                id: "team",
                label: "teamt5-it",
                is_current: false,
                agent_type: "copilot",
                window_label: "30d",
            },
            style: ChartSeriesStyle {
                color_slot: 1,
                is_selected: false,
                is_current: false,
                hidden: false,
            },
            points: vec![ChartPoint { x: 7.0, y: 88.0 }],
            last_seven_day_percent: Some(88.0),
            five_hour_used_percent: None,
            forecast_label: Some("~hit 6.4h"),
            five_hour_subframe: FiveHourSubframeState {
                available: false,
                start_x: None,
                end_x: None,
                lower_y: None,
                upper_y: None,
                reason: Some("no 5h window"),
            },
            is_zero_state: false,
        };

        assert_eq!(format_end_label(&series), "[copilot 30d] teamt5-it 88% ~hit 6.4h");
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
                window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "beta",
                    label: "Beta",
                    is_current: true,
                    agent_type: "codex",
                window_label: "7d",
                }),
            },
            chart: ChartState {
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "alpha",
                        label: "Alpha",
                        is_current: false,
                        agent_type: "codex",
                    window_label: "7d",
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
                    forecast_label: None,
                    five_hour_subframe: FiveHourSubframeState {
                        available: true,
                        start_x: Some(5.0),
                        end_x: Some(6.0),
                        lower_y: Some(20.0),
                        upper_y: Some(35.0),
                        reason: None,
                    },
                    is_zero_state: false,
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

        assert!(joined.contains("Usage chart"));
        assert!(joined.contains("Window view:100%"));
        assert!(joined.contains("[codex 7d] Alpha 76%/40%"));
        assert!(joined.contains("start"));
        assert!(joined.contains("50%"));
        assert!(joined.contains("reset"));
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
    fn render_chart_renders_single_zero_state_with_anchor_and_origin_marker() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "comet",
                    label: "comet",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "comet",
                    label: "comet",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
            },
            chart: ChartState {
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "comet",
                        label: "comet",
                        is_current: true,
                        agent_type: "codex",
                        window_label: "7d",
                    },
                    style: ChartSeriesStyle {
                        color_slot: 0,
                        is_selected: true,
                        is_current: true,
                        hidden: false,
                    },
                    points: vec![],
                    last_seven_day_percent: None,
                    five_hour_used_percent: None,
                    forecast_label: None,
                    five_hour_subframe: FiveHourSubframeState {
                        available: false,
                        start_x: None,
                        end_x: None,
                        lower_y: None,
                        upper_y: None,
                        reason: Some("zero-state"),
                    },
                    is_zero_state: true,
                }],
                seven_day_points: vec![],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("zero-state"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("zero-state"),
                },
                total_points: 0,
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

        let joined = render_lines(&state, 72, 18).join("\n");

        assert!(
            joined.contains("┌─ [codex] comet reset / no usage yet"),
            "single zero-state series should render as a connected ┌─ anchor above the origin, got:\n{joined}"
        );
        assert!(
            joined.contains("•"),
            "expected a dedicated zero-state marker at the chart origin, got:\n{joined}"
        );
        assert!(!joined.contains("|comet"), "single zero-state series should not fan out a branch label, got:\n{joined}");
    }

    #[test]
    fn render_chart_branches_multiple_zero_state_series_from_shared_anchor() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "beta",
                    label: "Beta",
                    is_current: false,
                    agent_type: "copilot",
                    window_label: "30d",
                }),
            },
            chart: ChartState {
                series: vec![
                    ChartSeries {
                        profile: RenderProfile {
                            id: "alpha",
                            label: "Alpha",
                            is_current: true,
                            agent_type: "codex",
                            window_label: "7d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 0,
                            is_selected: true,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![],
                        last_seven_day_percent: None,
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("zero-state"),
                        },
                        is_zero_state: true,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "beta",
                            label: "Beta",
                            is_current: false,
                            agent_type: "copilot",
                            window_label: "30d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 1,
                            is_selected: false,
                            is_current: false,
                            hidden: false,
                        },
                        points: vec![],
                        last_seven_day_percent: None,
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("zero-state"),
                        },
                        is_zero_state: true,
                    },
                ],
                seven_day_points: vec![],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("zero-state"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("zero-state"),
                },
                total_points: 0,
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

        assert!(joined.contains("┌─ [codex] Alpha"), "expected first zero-state branch to use ┌─ geometry, got:\n{joined}");
        assert!(joined.contains("├─ [copilot] Beta"), "expected second zero-state branch to use ├─ geometry, got:\n{joined}");
        assert!(joined.contains("•"), "expected origin marker below the zero-state branches, got:\n{joined}");
    }

    #[test]
    fn render_chart_keeps_zero_state_and_normal_series_coexisting() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "beta",
                    label: "Beta",
                    is_current: false,
                    agent_type: "copilot",
                    window_label: "30d",
                }),
            },
            chart: ChartState {
                series: vec![
                    ChartSeries {
                        profile: RenderProfile {
                            id: "alpha",
                            label: "Alpha",
                            is_current: true,
                            agent_type: "codex",
                            window_label: "7d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 0,
                            is_selected: true,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![],
                        last_seven_day_percent: None,
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("zero-state"),
                        },
                        is_zero_state: true,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "beta",
                            label: "Beta",
                            is_current: false,
                            agent_type: "copilot",
                            window_label: "30d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 1,
                            is_selected: false,
                            is_current: false,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.0, y: 3.0 },
                            ChartPoint { x: 7.0, y: 5.0 },
                        ],
                        last_seven_day_percent: Some(5.0),
                        five_hour_used_percent: Some(2.0),
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                ],
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 3.0 },
                    ChartPoint { x: 7.0, y: 5.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("mixed"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("mixed"),
                },
                total_points: 2,
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

        let joined = render_lines(&state, 72, 18).join("\n");

        assert!(
            joined.contains("┌─ [codex] Alpha reset / no usage yet"),
            "single zero-state series should keep a connected labeled anchor even when other series are visible, got:\n{joined}"
        );
        assert!(joined.contains("•"), "expected the zero-state origin marker to remain visible, got:\n{joined}");
        assert!(!joined.contains("|Alpha"), "single zero-state series should not fan out a branch label when mixed with normal series, got:\n{joined}");
        assert!(joined.contains("[copilot 30d] Beta 5%"), "expected normal end label to remain unchanged, got:\n{joined}");
    }

    #[test]
    fn render_chart_uses_compact_label_when_full_label_would_not_fit() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "comet",
                    label: "comet.jc",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "comet",
                    label: "comet.jc",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
            },
            chart: ChartState {
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "comet",
                        label: "comet.jc",
                        is_current: true,
                        agent_type: "codex",
                        window_label: "7d",
                    },
                    style: ChartSeriesStyle {
                        color_slot: 0,
                        is_selected: true,
                        is_current: true,
                        hidden: false,
                    },
                    points: vec![
                        ChartPoint { x: 0.0, y: 4.0 },
                        ChartPoint { x: 7.0, y: 7.0 },
                    ],
                    last_seven_day_percent: Some(7.0),
                    five_hour_used_percent: Some(0.0),
                    forecast_label: None,
                    five_hour_subframe: FiveHourSubframeState {
                        available: false,
                        start_x: None,
                        end_x: None,
                        lower_y: None,
                        upper_y: None,
                        reason: Some("no 5h window"),
                    },
                    is_zero_state: false,
                }],
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 4.0 },
                    ChartPoint { x: 7.0, y: 7.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("no 5h window"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("no 5h window"),
                },
                total_points: 2,
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

        let lines = render_lines(&state, 32, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("comet.jc"), "expected compact label to remain visible, got:\n{joined}");
    }

    #[test]
    fn render_chart_keeps_comet_label_visible_beside_neighboring_series() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "comet",
                    label: "comet.jc",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "team",
                    label: "teamt5-it",
                    is_current: false,
                    agent_type: "copilot",
                    window_label: "30d",
                }),
            },
            chart: ChartState {
                series: vec![
                    ChartSeries {
                        profile: RenderProfile {
                            id: "team",
                            label: "teamt5-it",
                            is_current: false,
                            agent_type: "copilot",
                            window_label: "30d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 1,
                            is_selected: false,
                            is_current: false,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.0, y: 3.0 },
                            ChartPoint { x: 7.0, y: 5.0 },
                        ],
                        last_seven_day_percent: Some(5.0),
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "comet",
                            label: "comet.jc",
                            is_current: true,
                            agent_type: "codex",
                            window_label: "7d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 0,
                            is_selected: true,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.0, y: 4.0 },
                            ChartPoint { x: 7.0, y: 7.0 },
                        ],
                        last_seven_day_percent: Some(7.0),
                        five_hour_used_percent: Some(0.0),
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                ],
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 4.0 },
                    ChartPoint { x: 7.0, y: 7.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("no 5h window"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("no 5h window"),
                },
                total_points: 4,
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

        let lines = render_lines(&state, 34, 18);
        let joined = lines.join("\n");

        assert!(joined.contains("comet.jc"), "expected comet label to remain visible beside neighboring series, got:\n{joined}");
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
                window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "alpha",
                    label: "Alpha",
                    is_current: true,
                    agent_type: "codex",
                window_label: "7d",
                }),
            },
            chart: ChartState {
                series: vec![ChartSeries {
                    profile: RenderProfile {
                        id: "alpha",
                        label: "Alpha",
                        is_current: true,
                        agent_type: "codex",
                    window_label: "7d",
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
                    forecast_label: None,
                    five_hour_subframe: FiveHourSubframeState {
                        available: false,
                        start_x: None,
                        end_x: None,
                        lower_y: None,
                        upper_y: None,
                        reason: Some("insufficient 5h overlap"),
                    },
                    is_zero_state: false,
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

    #[test]
    fn render_chart_gives_labels_an_opaque_background_for_readability() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "comet",
                    label: "comet.jc",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "team",
                    label: "teamt5-it",
                    is_current: false,
                    agent_type: "copilot",
                    window_label: "30d",
                }),
            },
            chart: ChartState {
                series: vec![
                    ChartSeries {
                        profile: RenderProfile {
                            id: "team",
                            label: "teamt5-it",
                            is_current: false,
                            agent_type: "copilot",
                            window_label: "30d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 1,
                            is_selected: false,
                            is_current: false,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.0, y: 3.0 },
                            ChartPoint { x: 7.0, y: 5.0 },
                        ],
                        last_seven_day_percent: Some(5.0),
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "comet",
                            label: "comet.jc",
                            is_current: true,
                            agent_type: "codex",
                            window_label: "7d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 0,
                            is_selected: true,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.0, y: 4.0 },
                            ChartPoint { x: 7.0, y: 7.0 },
                        ],
                        last_seven_day_percent: Some(7.0),
                        five_hour_used_percent: Some(0.0),
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                ],
                seven_day_points: vec![
                    ChartPoint { x: 0.0, y: 4.0 },
                    ChartPoint { x: 7.0, y: 7.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("no 5h window"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("no 5h window"),
                },
                total_points: 2,
                y_lower: -10.0,
                y_upper: 72.0,
                x_lower: 0.0,
                x_upper: 7.0,
                solo: false,
                tab_zoom_label: None,
                focused: false,
                fullscreen: false,
            },
        };

        let backend = TestBackend::new(72, 18);
        let mut terminal = Terminal::new(backend).unwrap();
        let frame = terminal
            .draw(|frame| render_chart(frame, &RenderContext::new(&state, frame.area())))
            .unwrap();

        let label_chars = "comet.jc".chars().collect::<Vec<_>>();
        let label_cells = frame
            .buffer
            .content
            .iter()
            .filter(|cell| {
                let symbol = cell.symbol();
                symbol.chars().count() == 1 && label_chars.contains(&symbol.chars().next().unwrap())
            })
            .collect::<Vec<_>>();

        assert!(
            label_cells.iter().any(|cell| cell.bg != Color::Reset),
            "expected end-label cells to use a non-reset background for readability"
        );
    }


    #[test]
    fn render_chart_keeps_label_visible_even_when_own_5h_band_overlaps_connector() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "comet",
                    label: "comet.jc",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
                current: Some(RenderProfile {
                    id: "comet",
                    label: "comet.jc",
                    is_current: true,
                    agent_type: "codex",
                    window_label: "7d",
                }),
            },
            chart: ChartState {
                series: vec![
                    ChartSeries {
                        profile: RenderProfile {
                            id: "comet",
                            label: "comet.jc",
                            is_current: true,
                            agent_type: "codex",
                            window_label: "7d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 0,
                            is_selected: true,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.05, y: 18.0 },
                            ChartPoint { x: 0.18, y: 22.0 },
                            ChartPoint { x: 0.37, y: 24.0 },
                        ],
                        last_seven_day_percent: Some(24.0),
                        five_hour_used_percent: Some(42.0),
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: true,
                            start_x: Some(0.22),
                            end_x: Some(0.37),
                            lower_y: Some(12.0),
                            upper_y: Some(42.0),
                            reason: None,
                        },
                        is_zero_state: false,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "team",
                            label: "teamt5-it",
                            is_current: false,
                            agent_type: "copilot",
                            window_label: "30d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 1,
                            is_selected: false,
                            is_current: false,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.03, y: 0.0 },
                            ChartPoint { x: 0.16, y: 0.0 },
                        ],
                        last_seven_day_percent: Some(0.0),
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                ],
                seven_day_points: vec![
                    ChartPoint { x: 0.05, y: 18.0 },
                    ChartPoint { x: 0.18, y: 22.0 },
                    ChartPoint { x: 0.37, y: 24.0 },
                ],
                five_hour_band: FiveHourBandState {
                    available: true,
                    used_percent: Some(42.0),
                    lower_y: Some(12.0),
                    upper_y: Some(42.0),
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: None,
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: true,
                    start_x: Some(0.22),
                    end_x: Some(0.37),
                    lower_y: Some(12.0),
                    upper_y: Some(42.0),
                    reason: None,
                },
                total_points: 5,
                y_lower: -10.0,
                y_upper: 72.0,
                x_lower: 0.0,
                x_upper: 7.0,
                solo: false,
                tab_zoom_label: None,
                focused: false,
                fullscreen: false,
            },
        };

        let lines = render_lines(&state, 237, 49);
        let joined = lines.join("\n");

        assert!(
            joined.contains("comet.jc"),
            "expected comet label to remain visible even when its own 5h band overlaps the connector, got:\n{joined}"
        );
    }


    #[test]
    fn render_chart_keeps_codex_label_visible_with_live_like_initial_state() {
        let state = MockState {
            selection: SelectionState {
                selected: Some(RenderProfile {
                    id: "cc",
                    label: "CC",
                    is_current: true,
                    agent_type: "claude",
                    window_label: "?d",
                }),
                current: Some(RenderProfile {
                    id: "cc",
                    label: "CC",
                    is_current: true,
                    agent_type: "claude",
                    window_label: "?d",
                }),
            },
            chart: ChartState {
                series: vec![
                    ChartSeries {
                        profile: RenderProfile {
                            id: "cc",
                            label: "CC",
                            is_current: true,
                            agent_type: "claude",
                            window_label: "?d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 0,
                            is_selected: true,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![],
                        last_seven_day_percent: None,
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no usage"),
                        },
                        is_zero_state: false,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "comet",
                            label: "comet",
                            is_current: true,
                            agent_type: "codex",
                            window_label: "7d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 1,
                            is_selected: false,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.16, y: 22.0 },
                            ChartPoint { x: 0.20, y: 24.0 },
                            ChartPoint { x: 0.24, y: 25.0 },
                            ChartPoint { x: 0.28, y: 27.0 },
                            ChartPoint { x: 0.32, y: 29.0 },
                            ChartPoint { x: 0.36, y: 31.0 },
                            ChartPoint { x: 0.40, y: 32.0 },
                            ChartPoint { x: 0.4486, y: 33.0 },
                        ],
                        last_seven_day_percent: Some(33.0),
                        five_hour_used_percent: Some(14.0),
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: true,
                            start_x: Some(0.25),
                            end_x: Some(0.4486),
                            lower_y: Some(31.0),
                            upper_y: Some(55.5),
                            reason: None,
                        },
                        is_zero_state: false,
                    },
                    ChartSeries {
                        profile: RenderProfile {
                            id: "team",
                            label: "teamt5-it",
                            is_current: true,
                            agent_type: "copilot",
                            window_label: "30d",
                        },
                        style: ChartSeriesStyle {
                            color_slot: 2,
                            is_selected: false,
                            is_current: true,
                            hidden: false,
                        },
                        points: vec![
                            ChartPoint { x: 0.05, y: 0.0 },
                            ChartPoint { x: 0.10, y: 0.0 },
                            ChartPoint { x: 0.15, y: 0.0 },
                            ChartPoint { x: 0.1814, y: 0.0 },
                        ],
                        last_seven_day_percent: Some(0.0),
                        five_hour_used_percent: None,
                        forecast_label: None,
                        five_hour_subframe: FiveHourSubframeState {
                            available: false,
                            start_x: None,
                            end_x: None,
                            lower_y: None,
                            upper_y: None,
                            reason: Some("no 5h window"),
                        },
                        is_zero_state: false,
                    },
                ],
                seven_day_points: vec![],
                five_hour_band: FiveHourBandState {
                    available: false,
                    used_percent: None,
                    lower_y: None,
                    upper_y: None,
                    delta_seven_day_percent: None,
                    delta_five_hour_percent: None,
                    reason: Some("no usage"),
                },
                five_hour_subframe: FiveHourSubframeState {
                    available: false,
                    start_x: None,
                    end_x: None,
                    lower_y: None,
                    upper_y: None,
                    reason: Some("no usage"),
                },
                total_points: 12,
                y_lower: -10.0,
                y_upper: 100.0,
                x_lower: 0.0,
                x_upper: 7.0,
                solo: false,
                tab_zoom_label: None,
                focused: false,
                fullscreen: true,
            },
        };

        let lines = render_lines(&state, 237, 49);
        let joined = lines.join("\n");

        assert!(
            joined.contains("comet"),
            "expected live-like layout to keep codex label visible, got:\n{joined}"
        );
    }


    #[test]
    fn layout_end_labels_allows_label_to_overlay_plot_cells() {
        let anchors = vec![LabelAnchor {
            text: "[codex 7d] comet 26%/14%".to_string(),
            fallback_texts: vec!["comet 26%".to_string(), "comet".to_string()],
            color: Color::Yellow,
            x: 16,
            y: 20,
        }];

        let graph_area = Rect::new(0, 0, 80, 30);
        let occupied = (17..26).map(|x| (x, 20)).collect::<HashSet<_>>();
        let labels = layout_end_labels(&anchors, graph_area, &occupied, &HashSet::new());

        assert_eq!(labels.len(), 1, "label should still place even when its preferred row has plot glyphs");
        assert!(labels[0].text.contains("comet"));
    }

    #[test]
    fn layout_end_labels_keeps_one_row_gap_from_blocked_band_cells() {
        let anchors = vec![LabelAnchor {
            text: "tag".to_string(),
            fallback_texts: vec![],
            color: Color::Yellow,
            x: 10,
            y: 5,
        }];

        let graph_area = Rect::new(10, 0, 12, 10);
        let blocked = (10..=15).map(|x| (x, 5)).collect::<HashSet<_>>();
        let labels = layout_end_labels(&anchors, graph_area, &HashSet::new(), &blocked);

        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].y, 3, "label should skip the blocked row and its 1-cell safety margin");
    }


    #[test]
    fn layout_end_labels_omits_label_when_band_claims_every_candidate() {
        let anchors = vec![LabelAnchor {
            text: "[codex 7d] comet 33%/14%".to_string(),
            fallback_texts: vec!["comet 33%".to_string(), "comet".to_string()],
            color: Color::Yellow,
            x: 20,
            y: 8,
        }];

        let graph_area = Rect::new(5, 0, 60, 12);
        let occupied = (5..60)
            .flat_map(|x| (0..12).map(move |y| (x, y)))
            .collect::<HashSet<_>>();
        let blocked = occupied.clone();
        let labels = layout_end_labels(&anchors, graph_area, &occupied, &blocked);

        assert!(
            labels.is_empty(),
            "label should be omitted when every candidate would violate the blocked band exclusion zone"
        );
    }


    #[test]
    fn layout_end_labels_force_fallback_prefers_full_label_when_space_exists() {
        let anchors = vec![LabelAnchor {
            text: "[codex 7d] comet 33%/14%".to_string(),
            fallback_texts: vec!["comet 33%".to_string(), "comet".to_string()],
            color: Color::Yellow,
            x: 12,
            y: 8,
        }];

        let graph_area = Rect::new(0, 0, 80, 12);
        let occupied = (0..80)
            .flat_map(|x| (0..12).filter(move |y| *y != 10).map(move |y| (x, y)))
            .collect::<HashSet<_>>();
        let blocked = (0..80)
            .flat_map(|x| {
                (0..12)
                    .filter(move |y| !matches!(*y, 9..=11))
                    .map(move |y| (x, y))
            })
            .collect::<HashSet<_>>();

        let labels = layout_end_labels(&anchors, graph_area, &occupied, &blocked);

        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].text, "[codex 7d] comet 33%/14%");
    }

    #[test]
    fn connector_attach_x_clamps_to_label_span() {
        assert_eq!(connector_attach_x(20, 10, 5), 20);
        assert_eq!(connector_attach_x(20, 10, 26), 26);
        assert_eq!(connector_attach_x(20, 10, 40), 29);
    }

    #[test]
    fn connector_cells_allow_attachment_within_label_span() {
        let graph_area = Rect::new(0, 0, 80, 20);
        let cells = connector_cells(24, 10, 12, 8, graph_area);

        assert!(cells.contains(&(24, 9)));
        assert!(cells.contains(&(13, 8)));
        assert!(!cells.contains(&(24, 10)));
        assert!(!cells.contains(&(12, 8)));
    }


}
