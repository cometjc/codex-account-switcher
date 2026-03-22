use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    codex_dir: PathBuf,
    accounts_dir: PathBuf,
    auth_path: PathBuf,
    current_name_path: PathBuf,
    limit_cache_path: PathBuf,
    ui_state_path: PathBuf,
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
            ui_state_path: codex_dir.join("codex-auth-ui-state.json"),
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

    pub fn ui_state_path(&self) -> &Path {
        &self.ui_state_path
    }
}

fn detect_codex_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".codex")
}
