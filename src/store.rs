use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use xattr::{get as get_xattr, set as set_xattr};

use crate::db::SqliteStore;
use crate::paths::AppPaths;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    pub hidden_profiles: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorePlatform {
    Copy,
}

impl StorePlatform {
    pub fn detect() -> Self {
        Self::Copy
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
        let db = SqliteStore::new(self.paths.database_path());
        if let Ok(Some(text)) = db.read_ui_state_value("hidden_profiles") {
            return serde_json::from_str(&text).unwrap_or_default();
        }
        let Ok(text) = fs::read_to_string(self.paths.ui_state_path()) else {
            return UiState::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn write_ui_state(&self, state: &UiState) -> Result<()> {
        let db = SqliteStore::new(self.paths.database_path());
        db.write_ui_state_value("hidden_profiles", &serde_json::to_string(state)?)
            .context("write ui_state to sqlite")?;
        fs::write(
            self.paths.ui_state_path(),
            serde_json::to_string_pretty(state)?,
        )
        .with_context(|| format!("write {}", self.paths.ui_state_path().display()))
    }

    pub fn list_account_names(&self) -> Result<Vec<String>> {
        self.ensure_accounts_dir()?;
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
        // Prefer the explicit current-name file when present; this keeps existing flows
        // and tests stable while xattr-based resolution becomes the primary path when
        // no current-name hint is available.
        if let Some(name) = self.read_current_name_file()? {
            return Ok(Some(name));
        }

        if !self.paths.auth_path().exists() {
            return Ok(None);
        }
        self.ensure_accounts_dir()?;
        let active_uuid = read_profile_uuid(self.paths.auth_path())
            .with_context(|| format!("read auth uuid from {}", self.paths.auth_path().display()))?;

        let mut matched_name: Option<String> = None;
        for name in self.list_account_names()? {
            let profile_path = self.account_file_path(&name);
            let profile_uuid = read_profile_uuid(&profile_path)
                .with_context(|| format!("read profile uuid from {}", profile_path.display()))?;
            if profile_uuid == active_uuid {
                if matched_name.is_some() {
                    bail!("multiple profiles match auth uuid {}", active_uuid);
                }
                matched_name = Some(name);
            }
        }
        matched_name.context("no saved profile matches auth uuid").map(Some)
    }

    pub fn save_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_auth_file_exists()?;
        self.ensure_accounts_dir()?;
        let target = self.account_file_path(&name);
        fs::copy(self.paths.auth_path(), &target)
            .with_context(|| format!("copy auth to {}", target.display()))?;
        ensure_profile_uuid(&target)?;
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
        ensure_profile_uuid(&destination)?;
        Ok(name)
    }

    /// Overwrite a saved profile with the current auth snapshot (e.g. after token rotation).
    pub fn update_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_auth_file_exists()?;
        self.ensure_accounts_dir()?;
        let target = self.account_file_path(&name);
        fs::copy(self.paths.auth_path(), &target)
            .with_context(|| format!("update saved profile {}", name))?;
        ensure_profile_uuid(&target)?;
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

        let profile_uuid = read_profile_uuid(&source)
            .with_context(|| format!("read profile uuid from {}", source.display()))?;
        replace_auth_atomically_with_uuid(&source, self.paths.auth_path(), &profile_uuid)?;

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
            ensure_profile_uuid(&next_path)?;
        }

        if let Ok(Some(active)) = self.get_current_account_name() {
            if active == current {
                fs::write(self.paths.current_name_path(), format!("{next}\n"))
                    .with_context(|| format!("write {}", self.paths.current_name_path().display()))?;
            }
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
        self.migrate_legacy_accounts_dir()?;
        fs::create_dir_all(self.paths.accounts_dir())
            .with_context(|| format!("create {}", self.paths.accounts_dir().display()))?;
        for entry in fs::read_dir(self.paths.accounts_dir())
            .with_context(|| format!("read {}", self.paths.accounts_dir().display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                ensure_profile_uuid(&path)?;
            }
        }
        Ok(())
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

    fn migrate_legacy_accounts_dir(&self) -> Result<()> {
        let legacy_accounts_dir = self
            .paths
            .codex_dir()
            .join("accounts");
        if !legacy_accounts_dir.exists() {
            return Ok(());
        }
        fs::create_dir_all(self.paths.accounts_dir())
            .with_context(|| format!("create {}", self.paths.accounts_dir().display()))?;
        for entry in fs::read_dir(&legacy_accounts_dir)
            .with_context(|| format!("read {}", legacy_accounts_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let destination = self.paths.accounts_dir().join(file_name);
            if destination.exists() {
                continue;
            }
            fs::rename(&path, &destination).with_context(|| {
                format!("migrate account {} -> {}", path.display(), destination.display())
            })?;
        }
        Ok(())
    }
}

const PROFILE_UUID_XATTR: &str = "user.agent-switch.profile-uuid";

fn ensure_profile_uuid(path: &Path) -> Result<String> {
    match read_profile_uuid(path) {
        Ok(existing) => Ok(existing),
        Err(_) => {
            let uuid = Uuid::new_v4().to_string();
            write_profile_uuid(path, &uuid)?;
            Ok(uuid)
        }
    }
}

fn read_profile_uuid(path: &Path) -> Result<String> {
    let raw = get_xattr(path, PROFILE_UUID_XATTR)
        .with_context(|| format!("xattr get {} on {}", PROFILE_UUID_XATTR, path.display()))?
        .context(format!("missing {} on {}", PROFILE_UUID_XATTR, path.display()))?;
    let value = String::from_utf8(raw)
        .with_context(|| format!("invalid UTF-8 xattr on {}", path.display()))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("empty {} on {}", PROFILE_UUID_XATTR, path.display());
    }
    Uuid::parse_str(trimmed).with_context(|| {
        format!("invalid UUID {} on {}", trimmed, path.display())
    })?;
    Ok(trimmed.to_string())
}

fn write_profile_uuid(path: &Path, uuid: &str) -> Result<()> {
    Uuid::parse_str(uuid).with_context(|| format!("invalid UUID value {}", uuid))?;
    set_xattr(path, PROFILE_UUID_XATTR, uuid.as_bytes())
        .with_context(|| format!("xattr set {} on {}", PROFILE_UUID_XATTR, path.display()))
}

fn replace_auth_atomically_with_uuid(source: &Path, auth_path: &Path, uuid: &str) -> Result<()> {
    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let tmp_path = auth_path.with_extension("json.tmp");
    fs::copy(source, &tmp_path)
        .with_context(|| format!("copy {} -> {}", source.display(), tmp_path.display()))?;
    write_profile_uuid(&tmp_path, uuid)
        .with_context(|| format!("write auth uuid to {}", tmp_path.display()))?;
    fs::rename(&tmp_path, auth_path)
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), auth_path.display()))
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let serialized = format!("{}\n", serde_json::to_string_pretty(value)?);
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|ext| ext.to_str()).unwrap_or("json")
    ));
    fs::write(&tmp_path, serialized).with_context(|| format!("write {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("rename {} -> {}", tmp_path.display(), path.display()))
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
