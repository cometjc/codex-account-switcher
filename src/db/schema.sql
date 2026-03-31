CREATE TABLE IF NOT EXISTS profiles (
  id INTEGER PRIMARY KEY,
  service TEXT NOT NULL,
  account_key TEXT NOT NULL,
  display_name TEXT,
  is_current INTEGER NOT NULL DEFAULT 0,
  UNIQUE(service, account_key)
);

CREATE TABLE IF NOT EXISTS usage_cache (
  profile_id INTEGER PRIMARY KEY,
  fetched_at INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS usage_observations (
  profile_id INTEGER NOT NULL,
  observed_at INTEGER NOT NULL,
  weekly_used_percent REAL,
  five_hour_used_percent REAL,
  PRIMARY KEY(profile_id, observed_at),
  FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS profile_windows (
  profile_id INTEGER PRIMARY KEY,
  weekly_reset_at INTEGER,
  weekly_window_seconds INTEGER NOT NULL DEFAULT 604800,
  five_hour_reset_at INTEGER,
  five_hour_window_seconds INTEGER NOT NULL DEFAULT 18000,
  FOREIGN KEY(profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ui_state (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cron_status (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  last_attempt INTEGER,
  last_success INTEGER,
  codex_error TEXT,
  claude_error TEXT,
  copilot_error TEXT
);

CREATE TABLE IF NOT EXISTS kv_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_observations_profile_time
  ON usage_observations(profile_id, observed_at);

