/// Format a duration in seconds as "3d 9h", "9h 38m", "45m", etc.
/// Uses the two largest non-zero units.
pub fn format_duration_short(secs: i64) -> String {
    let secs = secs.max(0) as u64;
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        if hours > 0 {
            format!("{days}d {hours}h")
        } else {
            format!("{days}d")
        }
    } else if hours > 0 {
        if mins > 0 {
            format!("{hours}h {mins}m")
        } else {
            format!("{hours}h")
        }
    } else {
        format!("{}m", mins.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::format_duration_short;

    #[test]
    fn format_duration_short_covers_all_tiers() {
        assert_eq!(format_duration_short(293_927), "3d 9h"); // 3*86400 + 9*3600 + 38*60 + 47
        assert_eq!(format_duration_short(86_400), "1d");
        assert_eq!(format_duration_short(9_000), "2h 30m");
        assert_eq!(format_duration_short(3_600), "1h");
        assert_eq!(format_duration_short(120), "2m");
        assert_eq!(format_duration_short(30), "1m"); // < 1m rounds up to 1m
        assert_eq!(format_duration_short(0), "1m");
        assert_eq!(format_duration_short(-1), "1m"); // negative clamped
    }
}
