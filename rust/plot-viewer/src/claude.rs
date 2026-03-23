use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Credential model ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeOauthToken {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: i64,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeCredentials {
    #[serde(rename = "claudeAiOauth")]
    pub claude_ai_oauth: ClaudeOauthToken,
}

impl ClaudeCredentials {
    /// Stable identifier derived from the refresh token (first 20 chars after prefix).
    /// Stays consistent between access-token rotations.
    pub fn account_id(&self) -> String {
        let token = &self.claude_ai_oauth.refresh_token;
        let body = token.strip_prefix("sk-ant-ort01-").unwrap_or(token.as_str());
        format!("claude-{}", &body[..body.len().min(20)])
    }

    pub fn access_token(&self) -> &str {
        &self.claude_ai_oauth.access_token
    }

    pub fn subscription_type(&self) -> &str {
        &self.claude_ai_oauth.subscription_type
    }
}

// ── Path helpers ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudePaths {
    claude_dir: PathBuf,
    credentials_path: PathBuf,
    accounts_dir: PathBuf,
    current_name_path: PathBuf,
    limit_cache_path: PathBuf,
    usage_history_path: PathBuf,
}

impl ClaudePaths {
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::from_claude_dir(home.join(".claude"))
    }

    pub fn from_claude_dir(claude_dir: PathBuf) -> Self {
        Self {
            credentials_path: claude_dir.join(".credentials.json"),
            accounts_dir: claude_dir.join("accounts"),
            current_name_path: claude_dir.join("current"),
            limit_cache_path: claude_dir.join("claude-auth-limit-cache.json"),
            usage_history_path: claude_dir.join("claude-auth-usage-history.json"),
            claude_dir,
        }
    }

    pub fn claude_dir(&self) -> &Path { &self.claude_dir }
    pub fn credentials_path(&self) -> &Path { &self.credentials_path }
    pub fn accounts_dir(&self) -> &Path { &self.accounts_dir }
    pub fn current_name_path(&self) -> &Path { &self.current_name_path }
    pub fn limit_cache_path(&self) -> &Path { &self.limit_cache_path }
    pub fn usage_history_path(&self) -> &Path { &self.usage_history_path }
}

// ── Account store ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClaudeStore {
    paths: ClaudePaths,
}

impl ClaudeStore {
    pub fn new(paths: ClaudePaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &ClaudePaths { &self.paths }

    pub fn get_current_credentials(&self) -> Result<ClaudeCredentials> {
        self.ensure_credentials_exist()?;
        read_credentials(self.paths.credentials_path())
    }

    pub fn get_current_snapshot(&self) -> Result<Value> {
        self.ensure_credentials_exist()?;
        read_snapshot(self.paths.credentials_path())
    }

    pub fn get_current_account_name(&self) -> Result<Option<String>> {
        read_current_name_file(self.paths.current_name_path())
    }

    pub fn list_account_names(&self) -> Result<Vec<String>> {
        if !self.paths.accounts_dir().exists() {
            return Ok(Vec::new());
        }
        let mut names = fs::read_dir(self.paths.accounts_dir())
            .with_context(|| format!("read {}", self.paths.accounts_dir().display()))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if !path.is_file() { return None; }
                let name = path.file_name()?.to_str()?;
                if !name.ends_with(".json") { return None; }
                Some(name.trim_end_matches(".json").to_string())
            })
            .collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }

    pub fn list_saved_profiles(&self) -> Result<Vec<ClaudeSavedProfile>> {
        self.list_account_names()?
            .into_iter()
            .map(|name| {
                let file_path = self.account_file_path(&name);
                Ok(ClaudeSavedProfile {
                    name,
                    snapshot: read_snapshot(&file_path)?,
                    file_path,
                })
            })
            .collect()
    }

    pub fn save_account(&self, raw_name: &str) -> Result<String> {
        let name = normalize_account_name(raw_name)?;
        self.ensure_credentials_exist()?;
        self.ensure_accounts_dir()?;
        fs::copy(self.paths.credentials_path(), self.account_file_path(&name))
            .with_context(|| format!("copy credentials to {}", self.account_file_path(&name).display()))?;
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
        fs::create_dir_all(self.paths.claude_dir())
            .with_context(|| format!("create {}", self.paths.claude_dir().display()))?;
        fs::copy(&source, self.paths.credentials_path())
            .with_context(|| format!("copy {} -> {}", source.display(), self.paths.credentials_path().display()))?;
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
        fs::remove_file(&target)
            .with_context(|| format!("delete {}", target.display()))?;
        Ok(name)
    }

    pub fn rename_account(&self, raw_current: &str, raw_next: &str) -> Result<String> {
        let current = normalize_account_name(raw_current)?;
        let next = normalize_account_name(raw_next)?;
        let current_path = self.account_file_path(&current);
        if !current_path.exists() {
            bail!("saved profile not found: {}", current);
        }
        let next_path = self.account_file_path(&next);
        if current != next && next_path.exists() {
            bail!("saved profile already exists: {}", next);
        }
        if current != next {
            fs::rename(&current_path, &next_path)
                .with_context(|| format!("rename {} -> {}", current_path.display(), next_path.display()))?;
        }
        if self.get_current_account_name()?.as_deref() == Some(current.as_str()) {
            fs::write(self.paths.current_name_path(), format!("{next}\n"))
                .with_context(|| format!("write {}", self.paths.current_name_path().display()))?;
        }
        Ok(next)
    }

    fn ensure_credentials_exist(&self) -> Result<()> {
        if !self.paths.credentials_path().exists() {
            bail!("no Claude credentials file found at {}", self.paths.credentials_path().display());
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeSavedProfile {
    pub name: String,
    pub file_path: PathBuf,
    pub snapshot: Value,
}

// ── Helpers (private) ──────────────────────────────────────────────────────────

fn read_credentials(path: impl AsRef<Path>) -> Result<ClaudeCredentials> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse Claude credentials at {}", path.display()))
}

fn read_snapshot(path: impl AsRef<Path>) -> Result<Value> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse JSON at {}", path.display()))
}

fn read_current_name_file(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let trimmed = contents.trim();
            Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("write {}", path.display()))
}

fn normalize_account_name(raw: &str) -> Result<String> {
    let trimmed = raw.trim().trim_end_matches(".json");
    if trimmed.is_empty() {
        bail!("invalid account name");
    }
    if !trimmed.chars().enumerate().all(|(i, ch)| {
        if i == 0 { ch.is_ascii_alphanumeric() }
        else { ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') }
    }) {
        bail!("invalid account name");
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir_pair() -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "claude-store-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        let claude_dir = base.join("dot-claude");
        fs::create_dir_all(&claude_dir).unwrap();
        (claude_dir, base)
    }

    fn sample_creds_json() -> &'static str {
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-aaa","refreshToken":"sk-ant-ort01-bbb","expiresAt":9999999999,"subscriptionType":"pro","rateLimitTier":"x","scopes":[]}}"#
    }

    #[test]
    fn claude_store_list_save_use_roundtrip() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();
        let store = ClaudeStore::new(paths);

        assert!(store.list_account_names().unwrap().is_empty());

        let name = store.save_account("work").unwrap();
        assert_eq!(name, "work");
        assert_eq!(store.list_account_names().unwrap(), vec!["work"]);

        store.use_account("work").unwrap();
        let current = store.get_current_credentials().unwrap();
        assert_eq!(current.claude_ai_oauth.subscription_type, "pro");
    }

    #[test]
    fn claude_store_rename_delete() {
        let (claude_dir, _base) = temp_dir_pair();
        let paths = ClaudePaths::from_claude_dir(claude_dir.clone());
        fs::write(paths.credentials_path(), sample_creds_json()).unwrap();
        let store = ClaudeStore::new(paths);
        store.save_account("work").unwrap();
        store.rename_account("work", "personal").unwrap();
        assert_eq!(store.list_account_names().unwrap(), vec!["personal"]);
        store.delete_account("personal").unwrap();
        assert!(store.list_account_names().unwrap().is_empty());
    }
}
