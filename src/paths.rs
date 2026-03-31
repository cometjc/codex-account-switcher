use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    codex_dir: PathBuf,
    accounts_dir: PathBuf,
    auth_path: PathBuf,
    current_name_path: PathBuf,
    limit_cache_path: PathBuf,
    usage_history_path: PathBuf,
    ui_state_path: PathBuf,
    cron_status_path: PathBuf,
}

impl AppPaths {
    pub fn detect() -> Self {
        let codex_dir = detect_codex_dir();
        Self::from_codex_dir(codex_dir)
    }

    pub fn from_codex_dir(codex_dir: PathBuf) -> Self {
        Self {
            accounts_dir: codex_dir.join("accounts"),
            auth_path: codex_dir.join("auth.json"),
            current_name_path: codex_dir.join("current"),
            limit_cache_path: codex_dir.join("codex-auth-limit-cache.json"),
            usage_history_path: codex_dir.join("codex-auth-usage-history.json"),
            ui_state_path: codex_dir.join("codex-auth-ui-state.json"),
            cron_status_path: codex_dir.join("codex-auth-cron-last-run"),
            codex_dir,
        }
    }

    pub fn codex_dir(&self) -> &Path {
        &self.codex_dir
    }

    pub fn accounts_dir(&self) -> &Path {
        &self.accounts_dir
    }

    pub fn auth_path(&self) -> &Path {
        &self.auth_path
    }

    pub fn current_name_path(&self) -> &Path {
        &self.current_name_path
    }

    pub fn limit_cache_path(&self) -> &Path {
        &self.limit_cache_path
    }

    pub fn usage_history_path(&self) -> &Path {
        &self.usage_history_path
    }

    pub fn ui_state_path(&self) -> &Path {
        &self.ui_state_path
    }

    pub fn cron_status_path(&self) -> &Path {
        &self.cron_status_path
    }

    pub fn refresh_log_path(&self) -> PathBuf {
        self.codex_dir.join("agent-switch-refresh.log")
    }

    pub fn database_path(&self) -> PathBuf {
        detect_agent_switch_config_dir().join("agent-switch.db")
    }

    /// Playwright `storageState` JSON files for Cursor dashboard sessions (one file per profile name).
    /// Stored under the app config dir (not `~/.codex`, which is Codex-only).
    pub fn cursor_profiles_dir(&self) -> PathBuf {
        detect_agent_switch_config_dir().join("cursor-profiles")
    }

    pub fn cursor_storage_state_path(&self, profile_name: &str) -> PathBuf {
        self.cursor_profiles_dir().join(format!("{profile_name}.json"))
    }
}

/// XDG-style config: `~/.config/agent-switch` (Unix), `%APPDATA%\agent-switch` (Windows).
fn detect_agent_switch_config_dir() -> PathBuf {
    if cfg!(windows) {
        if let Ok(app) = std::env::var("APPDATA") {
            if !app.is_empty() {
                return PathBuf::from(app).join("agent-switch");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("agent-switch");
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("agent-switch")
}

pub fn agent_switch_config_dir() -> PathBuf {
    detect_agent_switch_config_dir()
}

fn detect_codex_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".codex")
}
