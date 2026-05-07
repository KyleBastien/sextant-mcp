//! Sextant MCP server.
//!
//! Reads newline-delimited JSON-RPC 2.0 requests from stdin, writes responses
//! to stdout. **stdout is reserved for protocol traffic only** — all logging
//! goes to stderr. Closing stdin causes the server to exit cleanly.

mod protocol;
mod tools;

use std::io::{BufRead, BufReader, Write};

use serde_json::{json, Value};

use crate::protocol::{codes, error, success, Request, PROTOCOL_VERSION};

fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            Ok(_) => continue,
            Err(err) => {
                tracing::error!(?err, "stdin read error; exiting");
                break;
            }
        };
        if let Some(response) = handle_line(&line) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

/// Process one JSON-RPC line. Returns `None` for notifications (which never
/// produce a response) and for malformed lines we can't even attribute an id
/// to.
fn handle_line(line: &str) -> Option<String> {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(?err, "ignoring malformed request");
            return None;
        }
    };
    let id = req.id.clone().unwrap_or(Value::Null);

    let outcome = dispatch(&req);

    if req.is_notification() {
        // Notifications never produce a response — even if they returned an
        // error from our handlers, we drop it.
        return None;
    }

    Some(match outcome {
        Ok(result) => success(id, result),
        Err((code, message)) => error(id, code, message),
    })
}

fn dispatch(req: &Request) -> Result<Value, (i32, String)> {
    match req.method.as_str() {
        "initialize" => Ok(initialize_result()),
        "notifications/initialized" => Ok(Value::Null),
        "tools/list" => Ok(json!({ "tools": tools::descriptors() })),
        "tools/call" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or((codes::INVALID_PARAMS, "missing `name`".into()))?
                .to_string();
            let args = req.params.get("arguments").cloned().unwrap_or(Value::Null);
            tools::dispatch(&name, args)
        }
        "ping" => Ok(json!({})),
        other => Err((
            codes::METHOD_NOT_FOUND,
            format!("method not found: {other}"),
        )),
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": { "listChanged": false }
        },
        "serverInfo": {
            "name": "sextant-mcp",
            "version": env!("CARGO_PKG_VERSION"),
        }
    })
}
