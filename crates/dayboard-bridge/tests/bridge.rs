use dayboard_bridge::{http, Bridge};
use dayboard_core::Core;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

const TOKEN: &str = "0123456789abcdef";

fn bridge() -> Arc<Bridge> {
    Arc::new(Bridge::new(
        Core::memory().unwrap(),
        TOKEN.to_string(),
        Arc::new(|_| {}),
    ))
}

#[test]
fn rpc_initialize_tools_command_summary_and_undo() {
    let bridge = bridge();
    let init = bridge
        .handle_rpc(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }))
        .unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "daybridge");

    let tools = bridge
        .handle_rpc(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
        .unwrap();
    assert!(tools["result"]["tools"].as_array().unwrap().len() >= 7);

    let call = bridge
        .handle_rpc(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "daybridge_command",
                "arguments": {
                    "command": {
                        "command": "create_task",
                        "input": { "text": "Bridge task today", "parent_id": null }
                    }
                }
            }
        }))
        .unwrap();
    assert!(!call["result"]["isError"].as_bool().unwrap());
    assert!(call["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Bridge task"));

    let summary = bridge
        .handle_rpc(&json!({ "jsonrpc": "2.0", "id": 4, "method": "dayboard.today_summary" }))
        .unwrap();
    assert!(summary["result"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|task| task["title"] == "Bridge task"));

    let search = bridge
        .handle_rpc(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "dayboard.search",
            "params": { "query": "Bridge" }
        }))
        .unwrap();
    assert_eq!(search["result"]["tasks"].as_array().unwrap().len(), 1);

    let undo = bridge
        .handle_rpc(&json!({ "jsonrpc": "2.0", "id": 6, "method": "dayboard.undo" }))
        .unwrap();
    assert_eq!(undo["result"]["action"], "create_task");

    assert!(bridge
        .handle_rpc(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
        .is_none());
    let unknown = bridge
        .handle_rpc(&json!({ "jsonrpc": "2.0", "id": 7, "method": "nope" }))
        .unwrap();
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn mutations_notify_the_app() {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    let bridge = Bridge::new(
        Core::memory().unwrap(),
        TOKEN.to_string(),
        Arc::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        }),
    );
    bridge
        .handle_rpc(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "dayboard.command",
            "params": {
                "command": {
                    "command": "create_task",
                    "input": { "text": "Notify", "parent_id": null }
                }
            }
        }))
        .unwrap();
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

fn post(port: u16, token: Option<&str>, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "POST /rpc HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn http_requires_the_bearer_token_and_executes_requests() {
    let port = http::spawn(bridge(), 0).unwrap();
    let unauthorized = post(port, None, r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
    assert!(unauthorized.starts_with("HTTP/1.1 401"));
    let authorized = post(
        port,
        Some(TOKEN),
        r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#,
    );
    assert!(authorized.starts_with("HTTP/1.1 200"));
    assert!(authorized.contains("\"result\":{}"));

    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("\"ok\":true"));
}
