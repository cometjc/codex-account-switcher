use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

pub mod async_adapter;

const MIGRATION_VERSION: i64 = 1;
const SQLITE_BUSY_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq)]
pub struct UsageCacheRow {
    pub account_key: String,
    pub fetched_at: i64,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageObservationRow {
    pub account_key: String,
    pub observed_at: i64,
    pub weekly_used_percent: Option<f64>,
    pub five_hour_used_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileWindowRow {
    pub account_key: String,
    pub weekly_reset_at: Option<i64>,
    pub weekly_window_seconds: i64,
    pub five_hour_reset_at: Option<i64>,
    pub five_hour_window_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronStatusRow {
    pub last_attempt: Option<i64>,
    pub last_success: Option<i64>,
    pub codex_error: Option<String>,
    pub claude_error: Option<String>,
    pub copilot_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SqliteStore {
    path: PathBuf,
}

impl SqliteStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn with_conn<T>(&self, action: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = open_connection(&self.path)?;
        action(&conn)
    }

    pub fn read_meta(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT value FROM kv_meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context("read kv_meta")
        })
    }

    pub fn write_meta(&self, key: &str, value: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO kv_meta(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .context("write kv_meta")?;
            Ok(())
        })
    }

    pub fn read_ui_state_value(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT value_json FROM ui_state WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context("read ui_state")
        })
    }

    pub fn write_ui_state_value(&self, key: &str, value_json: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO ui_state(key, value_json) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
                params![key, value_json],
            )
            .context("write ui_state")?;
            Ok(())
        })
    }

    pub fn read_cron_status(&self) -> Result<Option<CronStatusRow>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT last_attempt, last_success, codex_error, claude_error, copilot_error
                 FROM cron_status WHERE id = 1",
                [],
                |row| {
                    Ok(CronStatusRow {
                        last_attempt: row.get(0)?,
                        last_success: row.get(1)?,
                        codex_error: row.get(2)?,
                        claude_error: row.get(3)?,
                        copilot_error: row.get(4)?,
                    })
                },
            )
            .optional()
            .context("read cron_status")
        })
    }

    pub fn write_cron_status(&self, status: &CronStatusRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO cron_status(
                    id, last_attempt, last_success, codex_error, claude_error, copilot_error
                 ) VALUES (1, ?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    last_attempt = excluded.last_attempt,
                    last_success = excluded.last_success,
                    codex_error = excluded.codex_error,
                    claude_error = excluded.claude_error,
                    copilot_error = excluded.copilot_error",
                params![
                    status.last_attempt,
                    status.last_success,
                    status.codex_error,
                    status.claude_error,
                    status.copilot_error
                ],
            )
            .context("write cron_status")?;
            Ok(())
        })
    }

    pub fn read_usage_cache(&self, service: &str) -> Result<Vec<UsageCacheRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT p.account_key, c.fetched_at, c.payload_json
                 FROM usage_cache c
                 JOIN profiles p ON p.id = c.profile_id
                 WHERE p.service = ?1",
            )?;
            let rows = stmt
                .query_map(params![service], |row| {
                    Ok(UsageCacheRow {
                        account_key: row.get(0)?,
                        fetched_at: row.get(1)?,
                        payload_json: row.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
                .context("collect usage_cache rows")?;
            Ok(rows)
        })
    }

    pub fn write_usage_cache_rows(&self, service: &str, rows: &[UsageCacheRow]) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction().context("start cache transaction")?;
            tx.execute(
                "DELETE FROM usage_cache
                 WHERE profile_id IN (SELECT id FROM profiles WHERE service = ?1)",
                params![service],
            )
            .context("clear usage_cache for service")?;
            for row in rows {
                let profile_id = ensure_profile(&tx, service, &row.account_key, None)?;
                tx.execute(
                    "INSERT INTO usage_cache(profile_id, fetched_at, payload_json, updated_at)
                     VALUES(?1, ?2, ?3, ?4)
                     ON CONFLICT(profile_id) DO UPDATE SET
                        fetched_at = excluded.fetched_at,
                        payload_json = excluded.payload_json,
                        updated_at = excluded.updated_at",
                    params![profile_id, row.fetched_at, row.payload_json, now_unix_seconds()],
                )
                .context("upsert usage_cache")?;
            }
            tx.commit().context("commit cache transaction")?;
            Ok(())
        })
    }

    pub fn read_usage_history(
        &self,
        service: &str,
    ) -> Result<(Vec<UsageObservationRow>, Vec<ProfileWindowRow>)> {
        self.with_conn(|conn| {
            let mut obs_stmt = conn.prepare(
                "SELECT p.account_key, o.observed_at, o.weekly_used_percent, o.five_hour_used_percent
                 FROM usage_observations o
                 JOIN profiles p ON p.id = o.profile_id
                 WHERE p.service = ?1",
            )?;
            let observations = obs_stmt
                .query_map(params![service], |row| {
                    Ok(UsageObservationRow {
                        account_key: row.get(0)?,
                        observed_at: row.get(1)?,
                        weekly_used_percent: row.get(2)?,
                        five_hour_used_percent: row.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
                .context("collect usage observation rows")?;

            let mut win_stmt = conn.prepare(
                "SELECT p.account_key, w.weekly_reset_at, w.weekly_window_seconds,
                        w.five_hour_reset_at, w.five_hour_window_seconds
                 FROM profile_windows w
                 JOIN profiles p ON p.id = w.profile_id
                 WHERE p.service = ?1",
            )?;
            let windows = win_stmt
                .query_map(params![service], |row| {
                    Ok(ProfileWindowRow {
                        account_key: row.get(0)?,
                        weekly_reset_at: row.get(1)?,
                        weekly_window_seconds: row.get(2)?,
                        five_hour_reset_at: row.get(3)?,
                        five_hour_window_seconds: row.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
                .context("collect profile window rows")?;
            Ok((observations, windows))
        })
    }

    pub fn upsert_usage_history(
        &self,
        service: &str,
        windows: &[ProfileWindowRow],
        observations: &[UsageObservationRow],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction().context("start history transaction")?;
            tx.execute(
                "DELETE FROM usage_observations
                 WHERE profile_id IN (SELECT id FROM profiles WHERE service = ?1)",
                params![service],
            )
            .context("clear usage_observations for service")?;
            tx.execute(
                "DELETE FROM profile_windows
                 WHERE profile_id IN (SELECT id FROM profiles WHERE service = ?1)",
                params![service],
            )
            .context("clear profile_windows for service")?;
            for window in windows {
                let profile_id = ensure_profile(&tx, service, &window.account_key, None)?;
                tx.execute(
                    "INSERT INTO profile_windows(
                       profile_id, weekly_reset_at, weekly_window_seconds,
                       five_hour_reset_at, five_hour_window_seconds
                     ) VALUES(?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(profile_id) DO UPDATE SET
                        weekly_reset_at = excluded.weekly_reset_at,
                        weekly_window_seconds = excluded.weekly_window_seconds,
                        five_hour_reset_at = excluded.five_hour_reset_at,
                        five_hour_window_seconds = excluded.five_hour_window_seconds",
                    params![
                        profile_id,
                        window.weekly_reset_at,
                        window.weekly_window_seconds,
                        window.five_hour_reset_at,
                        window.five_hour_window_seconds
                    ],
                )
                .context("upsert profile_windows")?;
            }

            for observation in observations {
                let profile_id = ensure_profile(&tx, service, &observation.account_key, None)?;
                tx.execute(
                    "INSERT INTO usage_observations(
                        profile_id, observed_at, weekly_used_percent, five_hour_used_percent
                     ) VALUES(?1, ?2, ?3, ?4)
                     ON CONFLICT(profile_id, observed_at) DO UPDATE SET
                        weekly_used_percent = excluded.weekly_used_percent,
                        five_hour_used_percent = excluded.five_hour_used_percent",
                    params![
                        profile_id,
                        observation.observed_at,
                        observation.weekly_used_percent,
                        observation.five_hour_used_percent
                    ],
                )
                .context("upsert usage_observations")?;
            }
            tx.commit().context("commit history transaction")?;
            Ok(())
        })
    }
}

fn open_connection(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let conn = Connection::open(path).with_context(|| format!("open sqlite {}", path.display()))?;
    conn.busy_timeout(std::time::Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
        .context("set busy_timeout")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;")
        .context("set sqlite pragmas")?;
    let _ = conn.execute("PRAGMA journal_mode = WAL", []);
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations(
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )
    .context("create schema_migrations")?;
    let current: Option<i64> = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| row.get(0))
        .optional()
        .context("read schema version")?
        .flatten();
    if current.unwrap_or(0) >= MIGRATION_VERSION {
        return Ok(());
    }
    conn.execute_batch(include_str!("schema.sql"))
        .context("apply schema.sql")?;
    conn.execute(
        "INSERT OR REPLACE INTO schema_migrations(version, applied_at) VALUES(?1, ?2)",
        params![MIGRATION_VERSION, now_unix_seconds()],
    )
    .context("record migration version")?;
    Ok(())
}

fn ensure_profile(
    conn: &Connection,
    service: &str,
    account_key: &str,
    display_name: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO profiles(service, account_key, display_name)
         VALUES(?1, ?2, ?3)
         ON CONFLICT(service, account_key) DO UPDATE SET
            display_name = COALESCE(excluded.display_name, profiles.display_name)",
        params![service, account_key, display_name],
    )
    .context("upsert profile")?;
    conn.query_row(
        "SELECT id FROM profiles WHERE service = ?1 AND account_key = ?2",
        params![service, account_key],
        |row| row.get(0),
    )
    .context("select profile id")
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

