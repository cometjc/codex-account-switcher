use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use chrono::{SecondsFormat, Utc};

pub fn append_refresh_log(path: &Path, scope: &str, detail: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let _ = writeln!(file, "[{ts}] [{scope}] {detail}");
}
