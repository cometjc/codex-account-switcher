use super::ChartSeries;

pub(crate) fn full_label_lines(series: &ChartSeries<'_>) -> Vec<String> {
    let mut lines = vec![base_end_label(series)];
    if let Some(forecast) = series.forecast_label {
        lines.extend(split_hit_reset_lines(forecast));
    }
    if let Some(reset_line) = &series.reset_line_display {
        lines.extend(split_hit_reset_lines(&reset_line.text));
    }
    lines
}

pub(crate) fn compact_label_variants(series: &ChartSeries<'_>) -> Vec<Vec<String>> {
    let full_first_line = base_end_label(series);
    let compact = format!(
        "{} {}",
        series.profile.label,
        format_unsigned_percent(series.last_seven_day_percent),
    );
    let minimal = series.profile.label.to_string();
    let mut variants = Vec::new();
    for text in [full_first_line, compact, minimal] {
        if !variants.iter().any(|existing: &Vec<String>| existing.first() == Some(&text)) {
            variants.push(vec![text]);
        }
    }
    variants
}

fn base_end_label(series: &ChartSeries<'_>) -> String {
    let weekly = format_unsigned_percent(series.last_seven_day_percent);
    if series.profile.agent_type == "copilot" {
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
    }
}

fn format_unsigned_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}%"))
        .unwrap_or("?%".to_string())
}

fn split_hit_reset_lines(text: &str) -> Vec<String> {
    let mut parts = text
        .split('·')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        parts.push(text.trim().to_string());
    }
    parts
}
