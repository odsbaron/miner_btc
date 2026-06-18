use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{Read, Write};
use std::net::TcpListener;

use crate::hardware::DeviceStatus;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub title: String,
    pub devices: Vec<DeviceStatus>,
    pub stratum_state: String,
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
    let listener = TcpListener::bind(addr).with_context(|| format!("bind dashboard {addr}"))?;
    for stream in listener.incoming() {
        let mut stream = stream.context("accept dashboard client")?;
        let mut request = [0u8; 1024];
        let bytes = stream.read(&mut request).unwrap_or(0);
        let request_text = String::from_utf8_lossy(&request[..bytes]);
        let (content_type, body) = if request_text.starts_with("GET /health") {
            ("application/json", "{\"status\":\"ok\"}".to_string())
        } else if request_text.starts_with("GET /api/status") {
            ("application/json", render_status_json(&snapshot))
        } else {
            ("text/html; charset=utf-8", render_status_html(&snapshot))
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .context("write dashboard response")?;
    }
    Ok(())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
