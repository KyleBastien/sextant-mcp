//! HTTP transport for the MCP server.
//!
//! POST `/` with a JSON-RPC body, get a JSON-RPC response back. No SSE,
//! no batching, no notifications-back — strictly request/response. That
//! covers every host that wants to drive the server over a network
//! socket without forking a child process; the streaming variants of
//! the MCP transport are deferred until a real consumer asks for them.

use std::net::SocketAddr;

use axum::{
    extract::Json as ExtractJson,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde_json::{json, Value};

use crate::handler::handle_line;

pub(crate) async fn serve(addr: SocketAddr) -> std::io::Result<()> {
    let app = Router::new().route("/", post(rpc));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "sextant-mcp http listening");
    axum::serve(listener, app)
        .await
        .map_err(std::io::Error::other)
}

async fn rpc(ExtractJson(body): ExtractJson<Value>) -> Response {
    // Re-serialize so handle_line — which expects a single JSON-RPC line
    // — gets exactly the bytes the client sent. axum's extractor has
    // already validated that the body is well-formed JSON.
    let line = match serde_json::to_string(&body) {
        Ok(s) => s,
        Err(err) => return parse_error(err.to_string()),
    };
    match handle_line(&line) {
        Some(resp) => match serde_json::from_str::<Value>(&resp) {
            Ok(v) => (StatusCode::OK, axum::Json(v)).into_response(),
            Err(err) => parse_error(err.to_string()),
        },
        // Notifications: spec says return 202 Accepted with no body.
        None => StatusCode::ACCEPTED.into_response(),
    }
}

fn parse_error(message: String) -> Response {
    let body = json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": { "code": -32700, "message": format!("parse error: {message}") }
    });
    (StatusCode::BAD_REQUEST, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request;
    use tower::util::ServiceExt;

    fn app() -> Router {
        Router::new().route("/", post(rpc))
    }

    async fn post_json(body: Value) -> (StatusCode, Value) {
        let req = Request::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();
        let resp = app().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let v: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, v)
    }

    #[tokio::test]
    async fn initialize_round_trips_over_http() {
        let (status, v) = post_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }))
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["result"]["serverInfo"]["name"], "sextant-mcp");
    }

    #[tokio::test]
    async fn notification_returns_202_with_no_body() {
        let (status, v) = post_json(json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(v, Value::Null);
    }

    #[tokio::test]
    async fn unknown_method_returns_jsonrpc_error_in_body() {
        let (status, v) = post_json(json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "nope"
        }))
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["error"]["code"], -32601);
    }
}
