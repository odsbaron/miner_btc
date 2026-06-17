use anyhow::Result;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq)]
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
