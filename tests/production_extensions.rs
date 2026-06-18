use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use miner_btc::dashboard::{DashboardAuth, MetricsSnapshot, MetricsStore};
use miner_btc::hardware::{
    AvalonApiCommand, BitaxeApiCommand, CgminerApiCommand, DeviceEndpoint, LiveWritePolicy,
};
use miner_btc::stratum::{
    ClientRequest, NtimeRoller, StratumLoopConfig, StratumMinerLoop, VersionRollingMask,
};

#[test]
fn ntime_and_version_rolling_are_bounded_and_hex_encoded() {
    let mut ntime = NtimeRoller::new("5f5e1000", 2).unwrap();
    assert_eq!(ntime.next_hex().unwrap(), "5f5e1000");
    assert_eq!(ntime.next_hex().unwrap(), "5f5e1001");
    assert_eq!(ntime.next_hex().unwrap(), "5f5e1002");
    assert!(ntime.next_hex().is_err());

    let mask = VersionRollingMask::new("20000000", "1fffe000").unwrap();
    assert_eq!(mask.apply(0x2000).unwrap(), "20002000");
    assert!(mask.apply(0x8000_0000).is_err());
}

#[test]
fn client_request_submit_line_matches_public_pool_shape() {
    let line = ClientRequest::submit(
        3,
        "user.worker",
        "job-1",
        "00000000",
        "5f5e1000",
        "01000000",
    )
    .to_json_line();
    assert!(line.contains("\"method\":\"mining.submit\""));
    assert!(line.contains("user.worker"));
    assert!(line.ends_with('\n'));
}

#[test]
fn stratum_loop_subscribes_authorizes_and_submits_share_to_mock_public_pool() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut seen = Vec::new();
        for _ in 0..2 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            seen.push(line);
        }
        writeln!(
            stream,
            "{{\"id\":1,\"result\":[[],\"abcd\",4],\"error\":null}}"
        )
        .unwrap();
        writeln!(stream, "{{\"id\":2,\"result\":true,\"error\":null}}").unwrap();
        writeln!(
            stream,
            "{{\"id\":null,\"method\":\"mining.set_difficulty\",\"params\":[0.00000001]}}"
        )
        .unwrap();
        writeln!(stream, "{{\"id\":null,\"method\":\"mining.notify\",\"params\":[\"job-1\",\"00\",\"aa\",\"bb\",[],\"20000000\",\"207fffff\",\"5f5e1000\",true]}}").unwrap();
        let mut submit = String::new();
        reader.read_line(&mut submit).unwrap();
        seen.push(submit);
        seen
    });

    let mut miner = StratumMinerLoop::new(StratumLoopConfig {
        addr,
        username: "user.worker".to_string(),
        password: "x".to_string(),
        max_shares: 1,
        read_timeout: Duration::from_secs(3),
    });
    let summary = miner.run_once().unwrap();
    assert_eq!(summary.submitted_shares, 1);
    let seen = server.join().unwrap();
    assert!(seen[0].contains("mining.subscribe"));
    assert!(seen[1].contains("mining.authorize"));
    assert!(seen[2].contains("mining.submit"));
}

#[test]
fn live_asic_commands_build_real_vendor_api_requests_but_require_write_opt_in() {
    let endpoint = DeviceEndpoint::new("bitaxe", "192.0.2.10", 80);
    let bitaxe = BitaxeApiCommand::set_pool(&endpoint, "stratum+tcp://pool.example:3333", "u", "p");
    assert_eq!(bitaxe.method, "POST");
    assert_eq!(bitaxe.path, "/api/system");
    assert!(bitaxe.body.contains("pool.example"));
    assert!(LiveWritePolicy::DryRun.guard().is_err());
    assert!(LiveWritePolicy::AllowWrites.guard().is_ok());

    let cgminer = CgminerApiCommand::switch_pool(1);
    assert!(cgminer.json_line.contains("switchpool"));

    let avalon = AvalonApiCommand::set_voltage_offset(-1);
    assert!(avalon.json_body.contains("voltage"));
}

#[test]
fn dashboard_auth_and_metrics_persistence_work_without_leaking_secret() {
    let dir = std::env::temp_dir().join(format!("miner-btc-metrics-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("metrics.json");
    let store = MetricsStore::new(path.clone());
    let snapshot = MetricsSnapshot {
        submitted_shares: 7,
        accepted_shares: 6,
        rejected_shares: 1,
        reconnects: 2,
        last_error: Some("stale share".to_string()),
    };
    store.save(&snapshot).unwrap();
    assert_eq!(store.load().unwrap(), snapshot);

    let auth = DashboardAuth::bearer("secret-token");
    assert!(auth.is_authorized("GET / HTTP/1.1\r\nAuthorization: Bearer secret-token\r\n\r\n"));
    assert!(!auth.is_authorized("GET / HTTP/1.1\r\nAuthorization: Bearer wrong\r\n\r\n"));
    assert!(!format!("{:?}", auth).contains("secret-token"));

    let _ = fs::remove_dir_all(dir);
}
