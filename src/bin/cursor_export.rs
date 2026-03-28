//! Export Chrome cookies for the selected profile via CDP as a Playwright `storageState` JSON blob, then POST it to `agent-switch` ingest (Tailscale 100.* only on the server).
//!
//! Close all Chrome windows using this user-data dir before running (Chrome locks the profile).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use dialoguer::Select;
use serde_json::{json, Value};
use tungstenite::Message;

#[derive(Parser, Debug)]
#[command(name = "cursor-export")]
struct Args {
    /// Ingest URL, e.g. `http://100.x.y.z:9847/ingest`
    #[arg(long)]
    url: String,
    /// Same as `SESSION_INGEST_TOKEN` on the Linux host (optional).
    #[arg(long)]
    token: Option<String>,
    /// Remote debugging port for a dedicated Chrome instance.
    #[arg(long, default_value_t = 9222)]
    cdp_port: u16,
    /// Path to `chrome.exe` / `google-chrome` (auto-detected if omitted).
    #[arg(long)]
    chrome: Option<PathBuf>,
    /// Chrome user data directory (auto-detected if omitted).
    #[arg(long)]
    user_data_dir: Option<PathBuf>,
}

struct KillChrome(Option<Child>);

impl Drop for KillChrome {
    fn drop(&mut self) {
        if let Some(mut c) = self.0.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn default_user_data_dir() -> Result<PathBuf> {
    if cfg!(windows) {
        let local = std::env::var("LOCALAPPDATA").context("LOCALAPPDATA")?;
        Ok(PathBuf::from(local).join("Google").join("Chrome").join("User Data"))
    } else {
        let home = std::env::var("HOME").context("HOME")?;
        Ok(PathBuf::from(home).join(".config").join("google-chrome"))
    }
}

fn default_chrome_exe() -> Result<PathBuf> {
    if cfg!(windows) {
        let p = PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe");
        if p.is_file() {
            return Ok(p);
        }
        anyhow::bail!("Chrome not found; pass --chrome");
    }
    for name in ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser"] {
        if let Ok(p) = which::which(name) {
            return Ok(p);
        }
    }
    anyhow::bail!("Chrome/Chromium not found in PATH; pass --chrome");
}

fn list_profiles(user_data: &Path) -> Result<Vec<(String, String)>> {
    let local_state = user_data.join("Local State");
    let text = fs::read_to_string(&local_state)
        .with_context(|| format!("read {}", local_state.display()))?;
    let v: Value = serde_json::from_str(&text).context("parse Local State")?;
    let cache = v["profile"]["info_cache"]
        .as_object()
        .context("profile.info_cache missing")?;
    let mut rows: Vec<(String, String)> = cache
        .iter()
        .map(|(dir, info)| {
            let label = info["name"]
                .as_str()
                .unwrap_or(dir)
                .to_string();
            (dir.clone(), label)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(rows)
}

fn wait_for_ws_url(port: u16) -> Result<String> {
    for _ in 0..120 {
        let url = format!("http://127.0.0.1:{port}/json/version");
        if let Ok(resp) = reqwest::blocking::get(&url) {
            if resp.status().is_success() {
                if let Ok(v) = resp.json::<Value>() {
                    if let Some(ws) = v.get("webSocketDebuggerUrl").and_then(|x| x.as_str()) {
                        return Ok(ws.to_string());
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(250));
    }
    anyhow::bail!("timed out waiting for Chrome CDP on port {port}");
}

fn cdp_get_all_cookies(ws_url: &str) -> Result<Vec<Value>> {
    let (mut socket, _) = tungstenite::connect(ws_url).context("ws connect")?;
    let cmd = json!({"id": 1, "method": "Network.getAllCookies", "params": {}});
    socket
        .send(Message::Text(cmd.to_string()))
        .context("send CDP")?;
    let msg = socket.read().context("read CDP")?;
    let text = match msg {
        Message::Text(t) => t,
        Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
        _ => anyhow::bail!("unexpected websocket message type"),
    };
    let v: Value = serde_json::from_str(&text).context("parse CDP response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("CDP error: {err}");
    }
    Ok(v["result"]["cookies"]
        .as_array()
        .context("result.cookies")?
        .clone())
}

fn map_cdp_cookie_to_playwright(c: &Value) -> Option<Value> {
    let name = c.get("name")?.as_str()?;
    let value = c.get("value")?.as_str()?;
    let domain = c.get("domain")?.as_str()?;
    let path = c.get("path")?.as_str().unwrap_or("/");
    let expires = c.get("expires").and_then(|x| x.as_f64()).unwrap_or(-1.0);
    let http_only = c.get("httpOnly").and_then(|x| x.as_bool()).unwrap_or(false);
    let secure = c.get("secure").and_then(|x| x.as_bool()).unwrap_or(false);
    let same_site = c
        .get("sameSite")
        .and_then(|x| x.as_str())
        .unwrap_or("Lax");
    Some(json!({
        "name": name,
        "value": value,
        "domain": domain,
        "path": path,
        "expires": expires,
        "httpOnly": http_only,
        "secure": secure,
        "sameSite": same_site,
    }))
}

fn build_storage_state(cookies: &[Value]) -> Value {
    let mapped: Vec<Value> = cookies.iter().filter_map(|c| map_cdp_cookie_to_playwright(c)).collect();
    json!({
        "cookies": mapped,
        "origins": [],
    })
}

fn post_ingest(url: &str, token: Option<&str>, body: &[u8]) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;
    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body.to_vec());
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {t}"));
    }
    let resp = req.send().context("POST /ingest")?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("ingest HTTP {status}: {text}");
    }
    Ok(text)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let user_data = match &args.user_data_dir {
        Some(p) => p.clone(),
        None => default_user_data_dir()?,
    };
    let chrome = match &args.chrome {
        Some(p) => p.clone(),
        None => default_chrome_exe()?,
    };

    let profiles = list_profiles(&user_data).context("list Chrome profiles")?;
    if profiles.is_empty() {
        anyhow::bail!("no profiles in {}", user_data.display());
    }

    let items: Vec<String> = profiles
        .iter()
        .map(|(dir, label)| format!("{label}  [{dir}]"))
        .collect();
    let idx = Select::new()
        .with_prompt("Chrome profile")
        .items(&items)
        .default(0)
        .interact()
        .context("select profile")?;
    let (profile_dir, _) = &profiles[idx];

    let port = args.cdp_port;
    let mut cmd = Command::new(&chrome);
    cmd.arg(format!("--user-data-dir={}", user_data.display()))
        .arg(format!("--profile-directory={profile_dir}"))
        .arg(format!("--remote-debugging-port={port}"))
        .arg("--no-first-run")
        .arg("--no-default-browser-check");
    if cfg!(unix) {
        cmd.arg("--disable-dev-shm-usage");
    }

    let child = cmd.spawn().with_context(|| format!("spawn {}", chrome.display()))?;
    let _guard = KillChrome(Some(child));

    let ws = wait_for_ws_url(port).context("CDP")?;
    let cookies = cdp_get_all_cookies(&ws).context("get cookies")?;
    let state = build_storage_state(&cookies);
    let bytes = serde_json::to_vec(&state).context("serialize storageState")?;

    let token = args
        .token
        .or_else(|| std::env::var("SESSION_INGEST_TOKEN").ok());
    let resp_text = post_ingest(&args.url, token.as_deref(), &bytes).context("upload")?;
    println!("{resp_text}");
    Ok(())
}
