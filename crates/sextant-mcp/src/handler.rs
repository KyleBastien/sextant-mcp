//! Transport-agnostic JSON-RPC dispatch.
//!
//! Both the stdio loop in `main.rs` and the axum POST handler in
//! `transport::http` funnel each incoming request line through
//! [`handle_line`]. Keeps wire-format concerns on the edges.

use serde_json::{json, Value};

use crate::protocol::{codes, error, success, Request, PROTOCOL_VERSION};
use crate::tools;

/// Parse, dispatch, and serialize one JSON-RPC line. Returns `None` for
/// notifications (no response on the wire) and for malformed input we
/// can't even attribute to an id.
pub(crate) fn handle_line(line: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_json_returns_none() {
        assert!(handle_line("{not json").is_none());
    }

    #[test]
    fn notification_returns_none_even_on_unknown_method() {
        // No `id` field => notification; even an unknown method must not
        // produce a response.
        let line = r#"{"jsonrpc":"2.0","method":"nope","params":{}}"#;
        assert!(handle_line(line).is_none());
    }

    #[test]
    fn initialize_returns_protocol_version() {
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let resp = handle_line(line).expect("response");
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["result"]["serverInfo"]["name"], "sextant-mcp");
        assert!(v["result"]["protocolVersion"].is_string());
    }

    #[test]
    fn unknown_method_returns_method_not_found() {
        let line = r#"{"jsonrpc":"2.0","id":2,"method":"nope/missing"}"#;
        let resp = handle_line(line).expect("response");
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"]["code"], codes::METHOD_NOT_FOUND);
    }
}
