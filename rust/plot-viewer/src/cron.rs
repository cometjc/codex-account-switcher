use std::path::Path;
use std::process::Command;

use anyhow::Result;

/// Status of the cron tracker job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronStatus {
    pub installed: bool,
    pub last_run: Option<i64>,
}

impl CronStatus {
    pub fn uninstalled() -> Self {
        Self { installed: false, last_run: None }
    }
}

/// Read the cron status file (contains a Unix timestamp as a decimal string).
pub fn read_status(path: &Path) -> CronStatus {
    let installed = is_installed();
    let last_run = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok());
    CronStatus { installed, last_run }
}

/// Write the current Unix timestamp to the status file, marking a successful run.
pub fn write_last_run_success(path: &Path) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", now))?;
    Ok(())
}

/// Check if our cron entry is present in the user's crontab.
pub fn is_installed() -> bool {
    Command::new("crontab")
        .args(["-l"])
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("codex-auth"))
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
