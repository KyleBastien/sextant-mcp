//! Minimal Model Context Protocol over stdio.
//!
//! MCP wraps JSON-RPC 2.0 in newline-delimited JSON over stdin/stdout.
//! Only the methods we serve are modeled here; everything else is reflected
//! back as a method-not-found error.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Deserialize)]
pub struct Request {
    #[allow(dead_code)]
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

impl Request {
    pub(crate) fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseSuccess {
    pub jsonrpc: &'static str,
    pub id: Value,
    pub result: Value,
}

#[derive(Debug, Serialize)]
pub struct ResponseError {
    pub jsonrpc: &'static str,
    pub id: Value,
    pub error: ErrorObj,
}

#[derive(Debug, Serialize)]
pub struct ErrorObj {
    pub code: i32,
    pub message: String,
}

pub mod codes {
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// Build a successful response wire-frame (a single JSON line).
pub(crate) fn success(id: Value, result: Value) -> String {
    let resp = ResponseSuccess {
        jsonrpc: "2.0",
        id,
        result,
    };
    serde_json::to_string(&resp).expect("serialize success response")
}

/// Build an error response wire-frame.
pub(crate) fn error(id: Value, code: i32, message: impl Into<String>) -> String {
    let resp = ResponseError {
        jsonrpc: "2.0",
        id,
        error: ErrorObj {
            code,
            message: message.into(),
        },
    };
    serde_json::to_string(&resp).expect("serialize error response")
}
