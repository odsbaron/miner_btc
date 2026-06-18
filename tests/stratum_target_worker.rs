use miner_btc::stratum::{
    difficulty_to_target_be_hex, Extranonce2Roller, ReconnectPolicy, StratumRuntime,
};
use miner_btc::worker::{dispatch_nonce_ranges, CpuWorker, WorkUnit};

#[test]
fn difficulty_one_matches_bitcoin_diff1_target() {
    assert_eq!(
        difficulty_to_target_be_hex(1.0).expect("difficulty 1 target"),
        "000000ffff000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn higher_difficulty_lowers_share_target() {
    let diff1 = difficulty_to_target_be_hex(1.0).expect("difficulty 1");
    let diff2 = difficulty_to_target_be_hex(2.0).expect("difficulty 2");

    assert!(diff2 < diff1);
    assert_eq!(
        diff2,
        "0000007fff800000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn extranonce2_rolls_fixed_width_little_endian_hex() {
    let mut roller = Extranonce2Roller::new(4);

    assert_eq!(roller.next_hex().expect("first"), "00000000");
    assert_eq!(roller.next_hex().expect("second"), "01000000");
    assert_eq!(roller.next_hex().expect("third"), "02000000");
}

#[test]
fn extranonce2_detects_overflow_for_configured_width() {
    let mut roller = Extranonce2Roller::with_start(1, 255);

    assert_eq!(roller.next_hex().expect("last byte value"), "ff");
    assert!(roller.next_hex().is_err());
}

#[test]
fn dispatcher_splits_nonce_ranges_across_workers() {
    let ranges = dispatch_nonce_ranges(0, 99, 4);

    assert_eq!(ranges.len(), 4);
    assert_eq!(ranges[0], 0..=24);
    assert_eq!(ranges[3], 75..=99);
}

#[test]
fn cpu_worker_finds_share_under_easy_target() {
    let unit = WorkUnit {
        job_id: "unit-test".to_string(),
        header_prefix: [0u8; 76],
        nonce_start: 0,
        nonce_end: 10,
        target_be_hex: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            .to_string(),
        extranonce2: "00000000".to_string(),
        ntime: "65a0bc00".to_string(),
    };

    let share = CpuWorker::default()
        .scan(unit)
        .expect("scan succeeds")
        .expect("easy target finds nonce 0");

    assert_eq!(share.job_id, "unit-test");
    assert_eq!(share.nonce, 0);
    assert_eq!(share.extranonce2, "00000000");
}

#[test]
fn reconnect_policy_uses_capped_exponential_backoff() {
    let policy = ReconnectPolicy::new(1, 8);

    assert_eq!(policy.delay_secs(0), 1);
    assert_eq!(policy.delay_secs(1), 2);
    assert_eq!(policy.delay_secs(2), 4);
    assert_eq!(policy.delay_secs(3), 8);
    assert_eq!(policy.delay_secs(10), 8);
}

#[test]
fn stratum_runtime_records_reconnect_attempts_after_failures() {
    let mut runtime = StratumRuntime::new(ReconnectPolicy::new(1, 4));

    runtime.record_connect_failure();
    runtime.record_connect_failure();
    runtime.record_connected();

    assert_eq!(runtime.total_failures(), 2);
    assert_eq!(runtime.current_attempt(), 0);
}
