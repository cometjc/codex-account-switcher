use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths::AppPaths;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    pub hidden_profiles: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorePlatform {
    Symlink,
    Copy,
}

impl StorePlatform {
    pub fn detect() -> Self {
        if cfg!(windows) {
            Self::Copy
        } else {
            Self::Symlink
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SavedProfile {
    pub name: String,
    pub file_path: PathBuf,
    pub snapshot: Value,
}

#[derive(Debug, Clone)]
pub struct AccountStore {
    paths: AppPaths,
    platform: StorePlatform,
}

impl AccountStore {
    pub fn new(paths: AppPaths, platform: StorePlatform) -> Self {
        Self { paths, platform }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn read_ui_state(&self) -> UiState {
        let Ok(text) = fs::read_to_string(self.paths.ui_state_path()) else {
            return UiState::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn write_ui_state(&self, state: &UiState) -> Result<()> {
        fs::write(
            self.paths.ui_state_path(),
            serde_json::to_string_pretty(state)?,
        )
        .with_context(|| format!("write {}", self.paths.ui_state_path().display()))
    }

    pub fn list_account_names(&self) -> Result<Vec<String>> {
        if !self.paths.accounts_dir().exists() {
            return Ok(Vec::new());
        }

        let mut names = fs::read_dir(self.paths.accounts_dir())
            .with_context(|| format!("read accounts dir {}", self.paths.accounts_dir().display()))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_file() {
                    return None;
                }
                let file_name = path.file_name()?.to_str()?;
                if !file_name.ends_with(".json") {
                    return None;
                }
                Some(file_name.trim_end_matches(".json").to_string())
            })
            .collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }

    pub fn list_saved_profiles(&self) -> Result<Vec<SavedProfile>> {
        self.list_account_names()?
            .into_iter()
            .map(|name| {
                let file_path = self.account_file_path(&name);
                Ok(SavedProfile {
                    name,
                    snapshot: self.read_snapshot(&file_path)?,
                    file_path,
                })
            })
            .collect()
    }

    pub fn get_current_snapshot(&self) -> Result<Value> {
        self.ensure_auth_file_exists()?;
        self.read_snapshot(self.paths.auth_path())
    }

    pub fn get_current_account_name(&self) -> Result<Option<String>> {
        if let Some(name) = self.read_current_name_file()? {
            return Ok(Some(name));
        }

        if !self.paths.auth_path().exists() {
            return Ok(None);
        }

        let metadata = fs::symlink_metadata(self.paths.auth_path())
            .with_context(|| format!("stat {}", self.paths.auth_path().display()))?;
        if !metadata.file_type().is_symlink() {
            return Ok(None);
        }

        let raw_target = fs::read_link(self.paths.auth_path())
            .with_context(|| format!("read link {}", self.paths.auth_path().display()))?;
        let resolved_target = self
            .paths
            .auth_path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(raw_target)
            .canonicalize()
            .with_context(|| format!("canonicalize {}", self.paths.auth_path().display()))?;
        let accounts_root = self
            .paths
            .accounts_dir()
            .canonicalize()
            .with_context(|| format!("canonicalize {}", self.paths.accounts_dir().display()))?;
        if !resolved_target.starts_with(&accounts_root) {
            return Ok(None);
        }
        Ok(resolved_target
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(ToOwned::to_owned))
    }

    pub fn save_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_auth_file_exists()?;
        self.ensure_accounts_dir()?;
        fs::copy(self.paths.auth_path(), self.account_file_path(&name))
            .with_context(|| format!("copy auth to {}", self.account_file_path(&name).display()))?;
        Ok(name)
    }

    pub fn save_snapshot(&self, raw_name: &str, snapshot: &Value) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_accounts_dir()?;
        let destination = self.account_file_path(&name);
        if destination.exists() {
            bail!("saved profile already exists: {}", name);
        }
        write_json(&destination, snapshot)?;
        Ok(name)
    }

    pub fn use_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        let source = self.account_file_path(&name);
        if !source.exists() {
            bail!("saved profile not found: {}", name);
        }

        fs::create_dir_all(self.paths.codex_dir())
            .with_context(|| format!("create {}", self.paths.codex_dir().display()))?;

        match self.platform {
            StorePlatform::Copy => {
                fs::copy(&source, self.paths.auth_path())
                    .with_context(|| format!("copy {} -> {}", source.display(), self.paths.auth_path().display()))?;
            }
            StorePlatform::Symlink => {
                if self.paths.auth_path().exists() {
                    let _ = fs::remove_file(self.paths.auth_path());
                }
                #[cfg(unix)]
                std::os::unix::fs::symlink(&source, self.paths.auth_path())
                    .with_context(|| format!("symlink {} -> {}", self.paths.auth_path().display(), source.display()))?;
                #[cfg(not(unix))]
                fs::copy(&source, self.paths.auth_path())
                    .with_context(|| format!("copy {} -> {}", source.display(), self.paths.auth_path().display()))?;
            }
        }

        fs::write(self.paths.current_name_path(), format!("{name}\n"))
            .with_context(|| format!("write {}", self.paths.current_name_path().display()))?;
        Ok(name)
    }

    pub fn delete_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        let target = self.account_file_path(&name);
        if !target.exists() {
            bail!("saved profile not found: {}", name);
        }
        fs::remove_file(&target).with_context(|| format!("delete {}", target.display()))?;
        Ok(name)
    }

    pub fn rename_account(&self, raw_current_name: &str, raw_next_name: &str) -> Result<String> {
        let current = normalize_account_name(raw_current_name)?;
        let next = normalize_account_name(raw_next_name)?;

        let current_path = self.account_file_path(&current);
        if !current_path.exists() {
            bail!("saved profile not found: {}", current);
        }

        let next_path = self.account_file_path(&next);
        if current != next && next_path.exists() {
            bail!("saved profile already exists: {}", next);
        }

        if current != next {
            fs::rename(&current_path, &next_path).with_context(|| {
                format!("rename {} -> {}", current_path.display(), next_path.display())
            })?;
        }

        if self.get_current_account_name()?.as_deref() == Some(current.as_str()) {
            fs::write(self.paths.current_name_path(), format!("{next}\n"))
                .with_context(|| format!("write {}", self.paths.current_name_path().display()))?;
        }

        Ok(next)
    }

    fn ensure_auth_file_exists(&self) -> Result<()> {
        if !self.paths.auth_path().exists() {
            bail!("no Codex auth file found at {}", self.paths.auth_path().display());
        }
        Ok(())
    }

    fn ensure_accounts_dir(&self) -> Result<()> {
        fs::create_dir_all(self.paths.accounts_dir())
            .with_context(|| format!("create {}", self.paths.accounts_dir().display()))
    }

    fn account_file_path(&self, name: &str) -> PathBuf {
        self.paths.accounts_dir().join(format!("{name}.json"))
    }

    fn read_snapshot(&self, path: impl AsRef<Path>) -> Result<Value> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read auth snapshot {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("parse JSON {}", path.display()))
    }

    fn read_current_name_file(&self) -> Result<Option<String>> {
        match fs::read_to_string(self.paths.current_name_path()) {
            Ok(contents) => {
                let trimmed = contents.trim();
                Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error)
                .with_context(|| format!("read {}", self.paths.current_name_path().display())),
        }
    }
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("write {}", path.display()))
}

fn normalize_account_name(raw_name: &str) -> Result<String> {
    let trimmed = raw_name.trim().trim_end_matches(".json");
    if trimmed.is_empty() {
        bail!("invalid account name");
    }
    if !trimmed
        .chars()
        .enumerate()
        .all(|(index, ch)| {
            if index == 0 {
                ch.is_ascii_alphanumeric()
            } else {
                ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')
            }
        })
    {
        bail!("invalid account name");
    }
    Ok(trimmed.to_string())
}
