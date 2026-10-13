use serde_json::{Value, json};
use soundcraft_ai_studio::transport::LocalControl;
use soundcraft_automation::Backend;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn loopback_json_lines_preserve_request_correlation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        for expected in 1..=2 {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let request: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(request["id"], expected);
            assert_eq!(request["method"], "session.inspect");
            writeln!(reader.get_mut(), "{}", json!({"id":expected,"ok":true,"result":{"tracks":[]}})).unwrap();
        }
    });
    let mut client = LocalControl::new(port).unwrap();
    for _ in 0..2 {
        assert_eq!(client.call("session.inspect", json!({})).unwrap(), json!({"tracks":[]}));
    }
    server.join().unwrap();
}

#[test]
fn unknown_edit_outcome_is_not_replayed() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        drop(listener); // Any retry would receive connection refused.
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["params"]["command"], "track.new");
        // App may already have applied this command. Close before replying.
    });
    let mut client = LocalControl::new(port).unwrap();
    let error = client.call("engine.execute", json!({"command":"track.new","params":{}})).unwrap_err();
    assert!(error.contains("outcome may be unknown"), "{error}");
    server.join().unwrap();
}

#[test]
fn mismatched_reply_is_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        writeln!(reader.get_mut(), "{}", json!({"id":999,"ok":true,"result":{}})).unwrap();
    });
    let mut client = LocalControl::new(port).unwrap();
    assert!(client.call("session.inspect", json!({})).unwrap_err().contains("id mismatch"));
    server.join().unwrap();
}
