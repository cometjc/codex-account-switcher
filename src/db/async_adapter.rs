use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tokio_rusqlite::Connection;

/// Async adapter for future background workers that run on tokio.
/// Current TUI flow remains synchronous and keeps using `SqliteStore`.
#[derive(Clone, Debug)]
pub struct AsyncSqliteStore {
    path: PathBuf,
}

impl AsyncSqliteStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn with_conn<T, F>(&self, action: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut rusqlite::Connection) -> tokio_rusqlite::Result<T> + Send + 'static,
    {
        let conn = Connection::open(&self.path)
            .await
            .with_context(|| format!("open tokio-rusqlite {}", self.path.display()))?;
        conn.call(action)
            .await
            .context("tokio-rusqlite call failed")
    }
}

