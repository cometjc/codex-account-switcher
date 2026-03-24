use std::path::Path;
use std::process::Command;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Status of the cron tracker job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronStatus {
    pub installed: bool,
    pub last_run: Option<i64>,
    pub last_attempt: Option<i64>,
    pub codex_error: Option<String>,
    pub claude_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CronStatusFile {
    #[serde(default)]
    last_attempt: Option<i64>,
    #[serde(default)]
    last_success: Option<i64>,
    #[serde(default)]
    codex_error: Option<String>,
    #[serde(default)]
    claude_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronRunReport {
    pub attempted_at: i64,
    pub codex_error: Option<String>,
    pub claude_error: Option<String>,
}

impl CronStatus {
    pub fn uninstalled() -> Self {
        Self {
            installed: false,
            last_run: None,
            last_attempt: None,
            codex_error: None,
            claude_error: None,
        }
    }
}

impl CronRunReport {
    pub fn succeeded_now() -> Self {
        Self {
            attempted_at: now_unix_seconds(),
            codex_error: None,
            claude_error: None,
        }
    }

    pub fn has_errors(&self) -> bool {
        self.codex_error.is_some() || self.claude_error.is_some()
    }
}

/// Read the cron status file (legacy decimal timestamp or JSON status payload).
pub fn read_status(path: &Path) -> CronStatus {
    let installed = is_installed();
    let parsed = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| parse_status_file(&raw));
    CronStatus {
        installed,
        last_run: parsed.as_ref().and_then(|status| status.last_success),
        last_attempt: parsed.as_ref().and_then(|status| status.last_attempt),
        codex_error: parsed.as_ref().and_then(|status| status.codex_error.clone()),
        claude_error: parsed.as_ref().and_then(|status| status.claude_error.clone()),
    }
}

/// Write the current Unix timestamp to the status file, marking a successful run.
pub fn write_last_run_success(path: &Path) -> Result<()> {
    write_run_report(path, &CronRunReport::succeeded_now())
}

pub fn write_run_report(path: &Path, report: &CronRunReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let previous = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| parse_status_file(&raw));
    let status = CronStatusFile {
        last_attempt: Some(report.attempted_at),
        last_success: if report.has_errors() {
            previous.and_then(|status| status.last_success)
        } else {
            Some(report.attempted_at)
        },
        codex_error: report.codex_error.clone(),
        claude_error: report.claude_error.clone(),
    };
    std::fs::write(path, serde_json::to_vec_pretty(&status)?)?;
    Ok(())
}

/// Check if our cron entry is present in the user's crontab.
pub fn is_installed() -> bool {
    Command::new("crontab")
        .args(["-l"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("agent-switch") || String::from_utf8_lossy(&out.stdout).contains("codex-auth"))
        .unwrap_or(false)
}

/// Install a cron entry that runs `binary --refresh-all` every 10 minutes.
/// If an entry already exists, does nothing.
pub fn ensure_installed(binary_path: &str) -> Result<bool> {
    if is_installed() {
        return Ok(false); // already there
    }
    let entry = format!("*/10 * * * * {} --refresh-all\n", binary_path);

    // Read existing crontab (may be empty)
    let existing = Command::new("crontab")
        .args(["-l"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();

    let new_crontab = format!("{}{}", existing, entry);

    // Write back via `crontab -`
    let mut child = Command::new("crontab")
        .args(["-"])
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.take() {
        use std::io::Write;
        let mut stdin = stdin;
        stdin.write_all(new_crontab.as_bytes())?;
    }
    child.wait()?;
    Ok(true)
}

fn parse_status_file(raw: &str) -> Option<CronStatusFile> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(timestamp) = trimmed.parse::<i64>() {
        return Some(CronStatusFile {
            last_attempt: Some(timestamp),
            last_success: Some(timestamp),
            codex_error: None,
            claude_error: None,
        });
    }
    serde_json::from_str(trimmed).ok()
}

fn now_unix_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_status_preserves_last_failure_details_from_json_file() {
        let path = std::env::temp_dir().join(format!(
            "agent-switch-cron-status-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"{"last_attempt":1700000300,"last_success":1700000000,"codex_error":null,"claude_error":"Claude usage request failed: 429"}"#,
        )
        .unwrap();

        let status = read_status(&path);

        std::fs::remove_file(&path).ok();
        assert_eq!(status.last_attempt, Some(1_700_000_300));
        assert_eq!(status.last_run, Some(1_700_000_000));
        assert_eq!(
            status.claude_error.as_deref(),
            Some("Claude usage request failed: 429")
        );
    }
}
