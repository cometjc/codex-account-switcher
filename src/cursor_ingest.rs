//! Minimal HTTP server to receive Playwright `storageState` JSON during "add Cursor profile".
//! Only IPv4 `100.*` (Tailscale CGNAT) may access; `::ffff:100.*` for IPv4-mapped IPv6.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{Context, Result};

const DEFAULT_MAX_BODY: usize = 8 * 1024 * 1024;

pub fn sanitize_cursor_profile_name(raw: &str) -> String {
    let t = raw.trim();
    let mut s = t
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>();
    if s.is_empty() {
        return "cursor".to_string();
    }
    if s.len() > 80 {
        s.truncate(80);
    }
    s
}

pub fn peer_is_tailscale_cgnat(addr: SocketAddr) -> bool {
    match addr {
        SocketAddr::V4(a) => a.ip().octets()[0] == 100,
        SocketAddr::V6(a) => a
            .ip()
            .to_ipv4_mapped()
            .map(|v4| v4.octets()[0] == 100)
            .unwrap_or(false),
    }
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_headers(headers_block: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for line in headers_block.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        m.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
    }
    m
}

fn read_http_request(stream: &mut TcpStream, max_body: usize) -> std::io::Result<Option<(String, String, HashMap<String, String>, Vec<u8>)>> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        if buf.len() > 256 * 1024 {
            return Ok(None);
        }
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_double_crlf(&buf) {
            let header_end = pos + 4;
            let header_text = String::from_utf8_lossy(&buf[..header_end]).to_string();
            let first_line = header_text.lines().next().unwrap_or("").to_string();
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() < 2 {
                return Ok(None);
            }
            let method = parts[0].to_string();
            let path = parts[1].to_string();
            let headers_raw = header_text
                .lines()
                .skip(1)
                .take_while(|l| !l.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            let headers = parse_headers(&headers_raw);
            let mut content_length: Option<usize> = None;
            if let Some(cl) = headers.get("content-length") {
                content_length = cl.parse().ok();
            }
            let rest = buf[header_end..].to_vec();
            let body = if let Some(len) = content_length {
                if len > max_body {
                    return Ok(None);
                }
                let mut body = rest;
                while body.len() < len {
                    let n = stream.read(&mut tmp)?;
                    if n == 0 {
                        return Ok(None);
                    }
                    body.extend_from_slice(&tmp[..n]);
                }
                body.truncate(len);
                body
            } else {
                Vec::new()
            };
            return Ok(Some((method, path, headers, body)));
        }
    }
}

fn write_all(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let mut off = 0;
    while off < data.len() {
        let n = stream.write(&data[off..])?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "short write",
            ));
        }
        off += n;
    }
    Ok(())
}

fn http_response(status: u16, body: &[u8], content_type: &str) -> Vec<u8> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let mut out = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).with_context(|| format!("create_dir_all {}", dir.display()))?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("storage-state.json");
    let tmp = dir.join(format!(".{name}.ingest.{}.tmp", std::process::id()));
    fs::write(&tmp, data).with_context(|| format!("write {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("rename to {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn bearer_from_headers(headers: &HashMap<String, String>) -> Option<String> {
    let raw = headers.get("authorization")?;
    let rest = raw.strip_prefix("Bearer")?;
    let t = rest.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Spawn background thread: listens until `stop` is set or one successful POST /ingest.
pub fn spawn_cursor_ingest_server(
    bind: &str,
    out_path: PathBuf,
    token: Option<String>,
    done_tx: Sender<Result<usize, String>>,
    stop: Arc<AtomicBool>,
) -> Result<JoinHandle<()>> {
    let listener = TcpListener::bind(bind).with_context(|| format!("bind {bind}"))?;
    listener
        .set_nonblocking(true)
        .context("set_nonblocking listener")?;
    let token = token.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let handle = std::thread::spawn(move || {
        'outer: loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let (mut stream, addr) = match listener.accept() {
                Ok(x) => x,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(_) => break,
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
            let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

            if !peer_is_tailscale_cgnat(addr) {
                let _ = write_all(
                    &mut stream,
                    &http_response(403, br#"{"error":"forbidden"}"#, "application/json"),
                );
                continue;
            }

            let Ok(Some((method, path, headers, body))) = read_http_request(&mut stream, DEFAULT_MAX_BODY)
            else {
                let _ = write_all(
                    &mut stream,
                    &http_response(400, br#"{"error":"bad request"}"#, "application/json"),
                );
                continue;
            };

            if method.eq_ignore_ascii_case("GET") && path == "/health" {
                let _ = write_all(
                    &mut stream,
                    &http_response(200, br#"{"status":"ok"}"#, "application/json"),
                );
                continue;
            }

            if method.eq_ignore_ascii_case("POST") && path == "/ingest" {
                if let Some(ref expected) = token {
                    let ok = bearer_from_headers(&headers)
                        .map(|b| b == *expected)
                        .unwrap_or(false);
                    if !ok {
                        let _ = write_all(
                            &mut stream,
                            &http_response(401, br#"{"error":"unauthorized"}"#, "application/json"),
                        );
                        continue;
                    }
                }
                if body.is_empty() {
                    let _ = write_all(
                        &mut stream,
                        &http_response(400, br#"{"error":"empty body"}"#, "application/json"),
                    );
                    continue;
                }
                if serde_json::from_slice::<serde_json::Value>(&body).is_err() {
                    let _ = write_all(
                        &mut stream,
                        &http_response(400, br#"{"error":"invalid json"}"#, "application/json"),
                    );
                    continue;
                }
                match atomic_write(&out_path, &body) {
                    Ok(()) => {
                        let msg = format!(
                            r#"{{"ok":true,"path":{},"bytes":{}}}"#,
                            serde_json::to_string(&out_path.display().to_string()).unwrap_or_default(),
                            body.len()
                        );
                        let _ = write_all(&mut stream, &http_response(200, msg.as_bytes(), "application/json"));
                        let _ = done_tx.send(Ok(body.len()));
                        stop.store(true, Ordering::SeqCst);
                        break 'outer;
                    }
                    Err(e) => {
                        let err_body = format!(r#"{{"error":"write failed: {e}"}}"#);
                        let _ = write_all(
                            &mut stream,
                            &http_response(500, err_body.as_bytes(), "application/json"),
                        );
                    }
                }
                continue;
            }

            let _ = write_all(
                &mut stream,
                &http_response(404, br#"{"error":"not found"}"#, "application/json"),
            );
        }
    });

    Ok(handle)
}

pub fn cursor_ingest_bind_addr() -> String {
    std::env::var("AGENT_SWITCH_CURSOR_INGEST_BIND")
        .unwrap_or_else(|_| "0.0.0.0:9847".to_string())
}

pub fn cursor_ingest_token() -> Option<String> {
    let t = std::env::var("SESSION_INGEST_TOKEN").ok()?;
    let t = t.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailscale_only_100() {
        assert!(peer_is_tailscale_cgnat("100.64.0.1:1234".parse().unwrap()));
        assert!(!peer_is_tailscale_cgnat("10.0.0.1:1".parse().unwrap()));
    }
}
