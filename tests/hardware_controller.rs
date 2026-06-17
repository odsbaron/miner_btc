use miner_btc::hardware::{DeviceStatus, MinerDevice, PoolConfig, WorkMode};

struct MockDevice;

impl MinerDevice for MockDevice {
    fn status(&self) -> anyhow::Result<DeviceStatus> {
        Ok(DeviceStatus {
            name: "avalon-q-test".to_string(),
            online: true,
            hashrate_ths: 90.5,
            temperature_c: Some(61.0),
            active_pool: Some("stratum+tcp://pool.example:3333".to_string()),
            work_mode: Some(WorkMode::Standard),
        })
    }

    fn set_pool(&self, _pool: &PoolConfig) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_work_mode(&self, _mode: WorkMode) -> anyhow::Result<()> {
        Ok(())
    }

    fn standby(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn hardware_status_renders_local_first_dashboard_line() {
    let device = MockDevice;
    let status = device.status().expect("mock status");

    assert_eq!(
        status.dashboard_line(),
        "avalon-q-test online 90.50 TH/s 61.0°C Standard stratum+tcp://pool.example:3333"
    );
}

#[test]
fn pool_config_redacts_password_for_logs() {
    let pool = PoolConfig {
        url: "stratum+tcp://pool.example:3333".to_string(),
        username: "wallet.worker".to_string(),
        password: "secret".to_string(),
    };

    assert_eq!(
        pool.redacted(),
        "stratum+tcp://pool.example:3333 wallet.worker password=<redacted>"
    );
}
