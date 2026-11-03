use dayboard_bridge::{http, Bridge};
use dayboard_core::Core;
use std::{path::PathBuf, sync::Arc};

fn usage() -> &'static str {
    "Usage: dayboard-bridge --db <path> [--port <port> --token <token>]\n\n\
Without --port, serves newline-delimited MCP JSON-RPC on stdio.\n\
The database defaults to $DAYBRIDGE_DB, then $DAYBRIDGE_DATA_DIR/daybridge.sqlite3."
}

fn main() {
    let mut db = std::env::var("DAYBRIDGE_DB")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("DAYBRIDGE_DATA_DIR")
                .map(|dir| PathBuf::from(dir).join("daybridge.sqlite3"))
        });
    let mut port: Option<u16> = None;
    let mut token = String::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => db = args.next().map(PathBuf::from),
            "--port" => port = args.next().and_then(|value| value.parse().ok()),
            "--token" => token = args.next().unwrap_or_default(),
            "--help" | "-h" => {
                println!("{}", usage());
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}\n{}", usage());
                std::process::exit(2);
            }
        }
    }
    let Some(db) = db else {
        eprintln!("{}", usage());
        std::process::exit(2);
    };
    let core = Core::open(&db).unwrap_or_else(|error| {
        eprintln!("Could not open {}: {error}", db.display());
        std::process::exit(1);
    });
    let bridge = Arc::new(Bridge::new(core, token.clone(), Arc::new(|_| {})));
    if let Some(port) = port {
        if token.trim().is_empty() {
            eprintln!("HTTP mode requires --token");
            std::process::exit(2);
        }
        let bound = http::spawn(bridge, port).unwrap_or_else(|error| {
            eprintln!("Bridge failed to start: {error}");
            std::process::exit(1);
        });
        eprintln!("Daybridge bridge listening on http://127.0.0.1:{bound}");
        loop {
            std::thread::park();
        }
    } else if let Err(error) = dayboard_bridge::serve_stdio(&bridge) {
        eprintln!("stdio bridge failed: {error}");
        std::process::exit(1);
    }
}
