use miner_btc::dashboard::{render_status_html, DashboardSnapshot};
use miner_btc::hardware::{
    AvalonAdapter, BitaxeAdapter, CgminerAdapter, DeviceEndpoint, DeviceStatus, MinerDevice,
    PoolConfig, WorkMode,
};

#[test]
fn adapters_expose_distinct_device_names_without_network_side_effects() {
    let cgminer = CgminerAdapter::new(DeviceEndpoint::new("cgminer", "127.0.0.1", 4028));
    let bitaxe = BitaxeAdapter::new(DeviceEndpoint::new("bitaxe", "192.0.2.10", 80));
    let avalon = AvalonAdapter::new(DeviceEndpoint::new("avalon-q", "192.0.2.11", 80));

    assert_eq!(
        cgminer.status().expect("cgminer status").name,
        "cgminer@127.0.0.1:4028"
    );
    assert_eq!(
        bitaxe.status().expect("bitaxe status").name,
        "bitaxe@192.0.2.10:80"
    );
    assert_eq!(
        avalon.status().expect("avalon status").name,
        "avalon-q@192.0.2.11:80"
    );
}

#[test]
fn adapters_accept_pool_and_workmode_commands_as_safe_dry_run() {
    let adapter = AvalonAdapter::new(DeviceEndpoint::new("avalon-q", "192.0.2.11", 80));
    let pool = PoolConfig {
        url: "stratum+tcp://pool.example:3333".to_string(),
        username: "wallet.worker".to_string(),
        password: "secret".to_string(),
    };

    adapter.set_pool(&pool).expect("dry-run pool set");
    adapter
        .set_work_mode(WorkMode::Eco)
        .expect("dry-run work mode set");
    adapter.standby().expect("dry-run standby");
}

#[test]
fn dashboard_html_renders_statuses_without_pool_passwords() {
    let snapshot = DashboardSnapshot {
        title: "miner_btc dashboard".to_string(),
        devices: vec![DeviceStatus {
            name: "bitaxe@192.0.2.10:80".to_string(),
            online: false,
            hashrate_ths: 0.0,
            temperature_c: None,
            active_pool: Some("stratum+tcp://pool.example:3333".to_string()),
            work_mode: Some(WorkMode::Standby),
        }],
        stratum_state: "Disconnected".to_string(),
        metrics: None,
    };

    let html = render_status_html(&snapshot);

    assert!(html.contains("miner_btc dashboard"));
    assert!(html.contains("bitaxe@192.0.2.10:80"));
    assert!(html.contains("Disconnected"));
    assert!(!html.contains("secret"));
}
