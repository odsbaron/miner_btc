use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

use miner_btc::stratum::{ClientRequest, LineClient, ServerMessage, SubscribeResponse};

#[test]
fn mock_pool_handshake_parses_subscribe_authorize_and_notify() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock pool");
    let addr = listener.local_addr().expect("mock pool addr");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept miner client");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut line = String::new();

        reader.read_line(&mut line).expect("read subscribe");
        assert!(line.contains("mining.subscribe"));
        writeln!(
            stream,
            "{{\"id\":1,\"result\":[[],\"08000002\",4],\"error\":null}}"
        )
        .expect("write subscribe response");
        stream.flush().expect("flush subscribe");

        line.clear();
        reader.read_line(&mut line).expect("read authorize");
        assert!(line.contains("mining.authorize"));
        writeln!(stream, "{{\"id\":2,\"result\":true,\"error\":null}}")
            .expect("write authorize response");
        writeln!(
            stream,
            "{{\"id\":null,\"method\":\"mining.set_difficulty\",\"params\":[1]}}"
        )
        .expect("write difficulty");
        writeln!(stream, "{{\"id\":null,\"method\":\"mining.notify\",\"params\":[\"job-1\",\"0000000000000000000000000000000000000000000000000000000000000001\",\"0100000001\",\"ffffffff02\",[],\"20000000\",\"207fffff\",\"65a0bc00\",true]}}").expect("write notify");
        stream.flush().expect("flush notifications");
    });

    let mut client = LineClient::connect(addr).expect("connect mock pool");
    client
        .send(&ClientRequest::subscribe(1))
        .expect("send subscribe");
    let subscribe_line = client.read_line().expect("read subscribe response");
    let subscribe =
        SubscribeResponse::from_json(&subscribe_line).expect("parse subscribe response");
    assert_eq!(subscribe.extranonce1, "08000002");

    client
        .send(&ClientRequest::authorize(2, "wallet.worker", "x"))
        .expect("send authorize");
    let authorize_line = client.read_line().expect("read authorize response");
    assert!(authorize_line.contains("\"result\":true"));

    assert_eq!(
        ServerMessage::from_json(&client.read_line().expect("read difficulty"))
            .expect("parse difficulty"),
        ServerMessage::SetDifficulty { difficulty: 1.0 }
    );
    assert!(matches!(
        ServerMessage::from_json(&client.read_line().expect("read notify")).expect("parse notify"),
        ServerMessage::Notify(_)
    ));

    server.join().expect("mock pool thread");
}
