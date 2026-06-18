use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use crate::hardware::DeviceStatus;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub title: String,
    pub devices: Vec<DeviceStatus>,
    pub stratum_state: String,
    #[serde(default)]
    pub metrics: Option<MetricsSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MetricsSnapshot {
    pub submitted_shares: u64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub reconnects: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MetricsStore {
    path: PathBuf,
}

impl MetricsStore {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn save(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        let json = serde_json::to_string_pretty(snapshot).context("serialize metrics snapshot")?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("write metrics {}", self.path.display()))
    }

    pub fn load(&self) -> Result<MetricsSnapshot> {
        let json = std::fs::read_to_string(&self.path)
            .with_context(|| format!("read metrics {}", self.path.display()))?;
        serde_json::from_str(&json).context("parse metrics snapshot")
    }
}

#[derive(Clone)]
pub struct DashboardAuth {
    bearer_token: Option<String>,
}

impl DashboardAuth {
    #[must_use]
    pub fn disabled() -> Self {
        Self { bearer_token: None }
    }

    #[must_use]
    pub fn bearer(token: impl Into<String>) -> Self {
        Self {
            bearer_token: Some(token.into()),
        }
    }

    #[must_use]
    pub fn is_authorized(&self, request_text: &str) -> bool {
        let Some(token) = &self.bearer_token else {
            return true;
        };
        request_text
            .lines()
            .any(|line| line.trim() == format!("Authorization: Bearer {token}"))
    }
}

impl fmt::Debug for DashboardAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DashboardAuth")
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[must_use]
pub fn render_status_html(snapshot: &DashboardSnapshot) -> String {
    let mut rows = String::new();
    for device in &snapshot.devices {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.2} TH/s</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape_html(&device.name),
            if device.online { "online" } else { "offline" },
            device.hashrate_ths,
            device
                .temperature_c
                .map(|value| format!("{value:.1}°C"))
                .unwrap_or_else(|| "n/a".to_string()),
            device
                .work_mode
                .map(|mode| mode.to_string())
                .unwrap_or_else(|| "n/a".to_string()),
            escape_html(device.active_pool.as_deref().unwrap_or("n/a")),
        ));
    }
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>body{{font-family:system-ui;margin:2rem;background:#0f172a;color:#e2e8f0}}table{{border-collapse:collapse;width:100%}}td,th{{border-bottom:1px solid #334155;padding:.7rem;text-align:left}}.card{{background:#111827;border:1px solid #334155;border-radius:12px;padding:1rem}}</style>
</head>
<body>
  <main class="card">
    <h1>{title}</h1>
    <p>Stratum state: <strong>{state}</strong></p>
    <table><thead><tr><th>Device</th><th>Status</th><th>Hashrate</th><th>Temp</th><th>Mode</th><th>Pool</th></tr></thead><tbody>{rows}</tbody></table>
  </main>
</body>
</html>"#,
        title = escape_html(&snapshot.title),
        state = escape_html(&snapshot.stratum_state),
        rows = rows,
    )
}

#[must_use]
pub fn render_status_json(snapshot: &DashboardSnapshot) -> String {
    serde_json::to_string_pretty(snapshot).expect("dashboard snapshot serializes")
}

pub fn serve_dashboard(addr: &str, snapshot: DashboardSnapshot) -> Result<()> {
    serve_dashboard_with_auth(addr, snapshot, DashboardAuth::disabled())
}

pub fn serve_dashboard_with_auth(
    addr: &str,
    snapshot: DashboardSnapshot,
    auth: DashboardAuth,
) -> Result<()> {
    let listener = TcpListener::bind(addr).with_context(|| format!("bind dashboard {addr}"))?;
    for stream in listener.incoming() {
        let mut stream = stream.context("accept dashboard client")?;
        let mut request = [0u8; 2048];
        let bytes = stream.read(&mut request).unwrap_or(0);
        let request_text = String::from_utf8_lossy(&request[..bytes]);
        if !auth.is_authorized(&request_text) {
            write_response(
                &mut stream,
                "401 Unauthorized",
                "text/plain",
                "unauthorized",
            )?;
            continue;
        }
        let (content_type, body) = if request_text.starts_with("GET /health") {
            ("application/json", "{\"status\":\"ok\"}".to_string())
        } else if request_text.starts_with("GET /api/status") {
            ("application/json", render_status_json(&snapshot))
        } else {
            ("text/html; charset=utf-8", render_status_html(&snapshot))
        };
        write_response(&mut stream, "200 OK", content_type, &body)?;
    }
    Ok(())
}

fn write_response(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .context("write dashboard response")
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
