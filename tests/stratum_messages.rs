use miner_btc::stratum::{ClientRequest, ServerMessage, SubscribeResponse};

#[test]
fn parses_subscribe_response_extranonce_contract() {
    let raw = r#"{
        "id": 1,
        "result": [[ ["mining.set_difficulty", "deadbeef"], ["mining.notify", "cafebabe"] ], "08000002", 4],
        "error": null
    }"#;

    let response = SubscribeResponse::from_json(raw).expect("valid subscribe response");

    assert_eq!(response.extranonce1, "08000002");
    assert_eq!(response.extranonce2_size, 4);
}

#[test]
fn parses_set_difficulty_notification() {
    let raw = r#"{"id":null,"method":"mining.set_difficulty","params":[16384]}"#;

    let message = ServerMessage::from_json(raw).expect("valid difficulty notification");

    assert_eq!(
        message,
        ServerMessage::SetDifficulty {
            difficulty: 16384.0
        }
    );
}

#[test]
fn parses_notify_job_from_python_bitcoin_miner_shape() {
    let raw = r#"{
        "id": null,
        "method": "mining.notify",
        "params": [
          "job-1",
          "0000000000000000000000000000000000000000000000000000000000000001",
          "0100000001",
          "ffffffff02",
          ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
          "20000000",
          "207fffff",
          "65a0bc00",
          true
        ]
    }"#;

    let message = ServerMessage::from_json(raw).expect("valid notify");

    match message {
        ServerMessage::Notify(job) => {
            assert_eq!(job.job_id, "job-1");
            assert_eq!(job.merkle_branch.len(), 1);
            assert!(job.clean_jobs);
        }
        other => panic!("expected notify, got {other:?}"),
    }
}

#[test]
fn builds_authorize_request_payload() {
    let payload = ClientRequest::authorize(2, "wallet.worker", "x").to_json_line();

    assert_eq!(
        payload.trim_end(),
        r#"{"id":2,"method":"mining.authorize","params":["wallet.worker","x"]}"#
    );
}
