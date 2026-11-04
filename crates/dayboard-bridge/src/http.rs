use crate::Bridge;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
};

const MAX_BODY: usize = 1_048_576;

pub fn spawn(bridge: Arc<Bridge>, port: u16) -> std::io::Result<u16> {
    let listener = bind(&bridge, port)?;
    let actual = listener.local_addr()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let bridge = bridge.clone();
            std::thread::spawn(move || {
                let _ = handle(stream, bridge);
            });
        }
    });
    Ok(actual)
}

pub fn serve(bridge: Arc<Bridge>, port: u16) -> std::io::Result<u16> {
    let listener = bind(&bridge, port)?;
    let actual = listener.local_addr()?.port();
    for stream in listener.incoming().flatten() {
        let bridge = bridge.clone();
        std::thread::spawn(move || {
            let _ = handle(stream, bridge);
        });
    }
    Ok(actual)
}

fn bind(bridge: &Bridge, port: u16) -> std::io::Result<TcpListener> {
    if bridge.token().trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "A bridge token is required for HTTP mode",
        ));
    }
    TcpListener::bind(("127.0.0.1", port))
}

fn handle(mut stream: TcpStream, bridge: Arc<Bridge>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .to_string();
    let mut content_length = 0usize;
    let mut authorization = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            match name.to_ascii_lowercase().as_str() {
                "content-length" => content_length = value.trim().parse().unwrap_or(0),
                "authorization" => authorization = value.trim().to_string(),
                _ => {}
            }
        }
    }
    if content_length > MAX_BODY {
        return respond(
            &mut stream,
            413,
            "Payload Too Large",
            "text/plain",
            b"payload too large",
        );
    }
    if method == "GET" && path == "/health" {
        return respond_json(
            &mut stream,
            200,
            &json!({ "ok": true, "service": "dayboard-bridge" }),
        );
    }
    if method == "GET" && path == "/events" {
        if !authorized(&authorization, bridge.token()) {
            return respond_json(&mut stream, 401, &json!({ "error": "unauthorized" }));
        }
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n")?;
        stream.write_all(b"data: {\"type\":\"ready\"}\n\n")?;
        stream.flush()?;
        let receiver = bridge.hub().subscribe();
        for message in receiver {
            if stream.write_all(message.as_bytes()).is_err() {
                break;
            }
            let _ = stream.flush();
        }
        return Ok(());
    }
    if method == "POST" && (path == "/rpc" || path == "/mcp") {
        if !authorized(&authorization, bridge.token()) {
            return respond_json(&mut stream, 401, &json!({ "error": "unauthorized" }));
        }
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body)?;
        let text = String::from_utf8_lossy(&body);
        let parsed: Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(_) => {
                return respond_json(
                    &mut stream,
                    200,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": "Parse error" }
                    }),
                )
            }
        };
        let response = if let Some(batch) = parsed.as_array() {
            let responses: Vec<Value> = batch
                .iter()
                .filter_map(|request| bridge.handle_rpc(request))
                .collect();
            if responses.is_empty() {
                None
            } else {
                Some(Value::Array(responses))
            }
        } else {
            bridge.handle_rpc(&parsed)
        };
        return match response {
            Some(value) => respond_json(&mut stream, 200, &value),
            None => respond(&mut stream, 204, "No Content", "application/json", b""),
        };
    }
    respond_json(&mut stream, 404, &json!({ "error": "not found" }))
}

fn authorized(header: &str, token: &str) -> bool {
    constant_time_eq(header.as_bytes(), format!("Bearer {token}").as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

fn respond_json(stream: &mut TcpStream, status: u16, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap_or_default();
    respond(
        stream,
        status,
        status_reason(status),
        "application/json",
        &body,
    )
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "OK",
    }
}
