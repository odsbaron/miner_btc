use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub struct PoolConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

impl PoolConfig {
    #[must_use]
    pub fn redacted(&self) -> String {
        format!("{} {} password=<redacted>", self.url, self.username)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WorkMode {
    Eco,
    Standard,
    Super,
    Standby,
}

impl std::fmt::Display for WorkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Eco => "Eco",
            Self::Standard => "Standard",
            Self::Super => "Super",
            Self::Standby => "Standby",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeviceStatus {
    pub name: String,
    pub online: bool,
    pub hashrate_ths: f64,
    pub temperature_c: Option<f64>,
    pub active_pool: Option<String>,
    pub work_mode: Option<WorkMode>,
}

impl DeviceStatus {
    #[must_use]
    pub fn dashboard_line(&self) -> String {
        let online = if self.online { "online" } else { "offline" };
        let temperature = self
            .temperature_c
            .map(|value| format!("{value:.1}°C"))
            .unwrap_or_else(|| "temp=n/a".to_string());
        let mode = self
            .work_mode
            .map(|value| value.to_string())
            .unwrap_or_else(|| "mode=n/a".to_string());
        let pool = self
            .active_pool
            .clone()
            .unwrap_or_else(|| "pool=n/a".to_string());
        format!(
            "{} {online} {:.2} TH/s {temperature} {mode} {pool}",
            self.name, self.hashrate_ths
        )
    }
}

/// Hardware/control-plane abstraction inspired by MinerWatch and Avalon Q Controller.
///
/// The mining core remains Bitcoin Core RPC / Stratum oriented. Real ASIC devices
/// can implement this trait later through cgminer JSON-RPC, Avalon APIs, Bitaxe
/// HTTP APIs, or an Umbrel-local controller without exposing pool credentials.
pub trait MinerDevice {
    fn status(&self) -> Result<DeviceStatus>;
    fn set_pool(&self, pool: &PoolConfig) -> Result<()>;
    fn set_work_mode(&self, mode: WorkMode) -> Result<()>;
    fn standby(&self) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEndpoint {
    pub kind: String,
    pub host: String,
    pub port: u16,
}

impl DeviceEndpoint {
    #[must_use]
    pub fn new(kind: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            kind: kind.into(),
            host: host.into(),
            port,
        }
    }

    #[must_use]
    pub fn label(&self) -> String {
        format!("{}@{}:{}", self.kind, self.host, self.port)
    }
}

macro_rules! dry_run_adapter {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            endpoint: DeviceEndpoint,
        }

        impl $name {
            #[must_use]
            pub fn new(endpoint: DeviceEndpoint) -> Self {
                Self { endpoint }
            }
        }

        impl MinerDevice for $name {
            fn status(&self) -> Result<DeviceStatus> {
                Ok(DeviceStatus {
                    name: self.endpoint.label(),
                    online: false,
                    hashrate_ths: 0.0,
                    temperature_c: None,
                    active_pool: None,
                    work_mode: Some(WorkMode::Standby),
                })
            }

            fn set_pool(&self, _pool: &PoolConfig) -> Result<()> {
                Ok(())
            }

            fn set_work_mode(&self, _mode: WorkMode) -> Result<()> {
                Ok(())
            }

            fn standby(&self) -> Result<()> {
                Ok(())
            }
        }
    };
}

dry_run_adapter!(CgminerAdapter);
dry_run_adapter!(BitaxeAdapter);
dry_run_adapter!(AvalonAdapter);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveWritePolicy {
    DryRun,
    AllowWrites,
}

impl LiveWritePolicy {
    pub fn guard(self) -> Result<()> {
        match self {
            Self::AllowWrites => Ok(()),
            Self::DryRun => anyhow::bail!("live ASIC writes require explicit AllowWrites policy"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpApiCommand {
    pub method: String,
    pub url: String,
    pub path: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgminerApiCommand {
    pub json_line: String,
}

impl CgminerApiCommand {
    #[must_use]
    pub fn switch_pool(pool_index: u32) -> Self {
        Self {
            json_line: format!(
                r#"{{"command":"switchpool","parameter":"{pool_index}"}}
"#
            ),
        }
    }

    #[must_use]
    pub fn add_pool(pool: &PoolConfig) -> Self {
        Self {
            json_line: format!(
                r#"{{"command":"addpool","parameter":"{},{},{}"}}
"#,
                pool.url, pool.username, pool.password
            ),
        }
    }
}

pub struct BitaxeApiCommand;

impl BitaxeApiCommand {
    #[must_use]
    pub fn set_pool(
        endpoint: &DeviceEndpoint,
        url: &str,
        username: &str,
        password: &str,
    ) -> HttpApiCommand {
        HttpApiCommand {
            method: "POST".to_string(),
            url: format!(
                "http://{}:{}{}",
                endpoint.host, endpoint.port, "/api/system"
            ),
            path: "/api/system".to_string(),
            body: serde_json::json!({
                "stratumURL": url,
                "stratumUser": username,
                "stratumPassword": password,
            })
            .to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvalonApiCommand {
    pub json_body: String,
}

impl AvalonApiCommand {
    #[must_use]
    pub fn set_voltage_offset(offset: i32) -> Self {
        Self {
            json_body:
                serde_json::json!({ "command": "set_voltage_offset", "voltage_offset": offset })
                    .to_string(),
        }
    }
}
