//! Concrete providers: Anthropic Messages and OpenAI-style Chat Completions.
//!
//! Both providers force structured JSON output:
//! - Anthropic: a tool-use block whose input is required to match a
//!   `report_findings` tool schema.
//! - OpenAI: `response_format: { type: "json_schema", strict: true }`.
//!   This same provider doubles as the OpenAI-compatible client when a
//!   custom `base_url` is supplied.
//!
//! API churn risk is handled by hand-rolling clients over `reqwest` (no
//! third-party Anthropic crate to drift). Wire shapes will be revisited
//! at M9 release time.

use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{JudgeError, JudgeProvider, JudgeRequest, JudgeResult};

const TOOL_NAME: &str = "report_findings";

fn tool_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "severity": {
                            "type": "string",
                            "enum": ["info", "warn", "error"]
                        },
                        "message": { "type": "string" },
                        "line": { "type": ["integer", "null"] },
                        "end_line": { "type": ["integer", "null"] },
                        "patch": {
                            "type": ["string", "null"],
                            "description": "Optional unified diff against the file proposing a fix. Omit when no concrete fix can be expressed mechanically."
                        }
                    },
                    "required": ["severity", "message"]
                }
            },
            "patch": {
                "type": ["string", "null"],
                "description": "Optional whole-file unified diff used when a single combined fix is more natural than per-finding patches."
            }
        },
        "required": ["findings"]
    })
}

/// POST a JSON body to a built request and decode the response into `T`.
/// Both providers share this exact roundtrip — only the auth headers and
/// URL shape differ, and those are configured upstream.
async fn send_and_decode<T: serde::de::DeserializeOwned>(
    req: RequestBuilder,
) -> Result<T, JudgeError> {
    let resp = req
        .send()
        .await
        .map_err(|e| JudgeError::Http(e.to_string()))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| JudgeError::Http(e.to_string()))?;
    if !status.is_success() {
        return Err(JudgeError::Api {
            status: status.as_u16(),
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| JudgeError::Parse(e.to_string()))
}

/// Build, customize, send, decode. The closure adds provider-specific
/// auth headers; everything else is identical between Anthropic and
/// OpenAI.
async fn post_json<T: serde::de::DeserializeOwned>(
    http: &HttpProvider,
    path: &str,
    body: &Value,
    customize: impl FnOnce(RequestBuilder) -> RequestBuilder,
) -> Result<T, JudgeError> {
    let req = customize(
        http.client
            .post(format!("{}{}", http.base_url, path))
            .json(body),
    );
    send_and_decode(req).await
}

/// Shared state for both providers: an HTTP client, a bearer-style key,
/// and a base URL.
struct HttpProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl HttpProvider {
    fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
        }
    }
}

/// Generate `new(api_key)` + `with_base_url(api_key, url)` constructors
/// over an `HttpProvider` newtype. Both providers want the exact same
/// shape; the macro keeps the duplication detector quiet.
macro_rules! http_judge_constructors {
    ($t:ident, default_url = $default:expr) => {
        impl $t {
            /// Build with the default public endpoint. Use
            #[doc = concat!("[`", stringify!($t), "::with_base_url`]")]
            /// to point at a proxy or compatible endpoint.
            pub fn new(api_key: String) -> Self {
                Self::with_base_url(api_key, $default.into())
            }

            pub fn with_base_url(api_key: String, base_url: String) -> Self {
                Self(HttpProvider::new(api_key, base_url))
            }
        }
    };
}

pub struct AnthropicJudge(HttpProvider);
http_judge_constructors!(AnthropicJudge, default_url = "https://api.anthropic.com");

#[async_trait]
impl JudgeProvider for AnthropicJudge {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn judge(&self, req: JudgeRequest<'_>) -> Result<JudgeResult, JudgeError> {
        let body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "system": req.system_prompt.unwrap_or(""),
            "messages": [{ "role": "user", "content": req.user_prompt }],
            "tools": [{
                "name": TOOL_NAME,
                "description": "Report code-review findings as structured JSON.",
                "input_schema": tool_schema()
            }],
            "tool_choice": { "type": "tool", "name": TOOL_NAME }
        });
        let parsed: AnthropicResponse = post_json(&self.0, "/v1/messages", &body, |r| {
            r.header("x-api-key", &self.0.api_key)
                .header("anthropic-version", "2023-06-01")
        })
        .await?;
        let tool_input = parsed
            .content
            .into_iter()
            .find_map(|b| match b {
                AnthropicBlock::ToolUse { input, .. } => Some(input),
                _ => None,
            })
            .ok_or_else(|| JudgeError::Parse("no tool_use block in response".into()))?;
        serde_json::from_value(tool_input).map_err(|e| JudgeError::Parse(e.to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicBlock {
    Text {
        #[allow(dead_code)]
        text: String,
    },
    ToolUse {
        #[allow(dead_code)]
        name: String,
        input: Value,
    },
}

/// OpenAI Chat Completions + structured outputs. Set `base_url` to point
/// at an OpenAI-compatible endpoint (Ollama, vLLM, OpenRouter, etc.).
pub struct OpenAiJudge(HttpProvider);
http_judge_constructors!(OpenAiJudge, default_url = "https://api.openai.com/v1");

#[async_trait]
impl JudgeProvider for OpenAiJudge {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn judge(&self, req: JudgeRequest<'_>) -> Result<JudgeResult, JudgeError> {
        let mut messages = Vec::new();
        if let Some(sys) = req.system_prompt {
            if !sys.is_empty() {
                messages.push(json!({ "role": "system", "content": sys }));
            }
        }
        messages.push(json!({ "role": "user", "content": req.user_prompt }));

        let body = json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "messages": messages,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": TOOL_NAME,
                    "schema": tool_schema(),
                    "strict": true
                }
            }
        });
        let parsed: OpenAiResponse = post_json(&self.0, "/chat/completions", &body, |r| {
            r.bearer_auth(&self.0.api_key)
        })
        .await?;
        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .ok_or_else(|| JudgeError::Parse("no content in response".into()))?;
        serde_json::from_str(content).map_err(|e| JudgeError::Parse(e.to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_constructors_default_base_url() {
        let a = AnthropicJudge::new("sk".into());
        assert_eq!(a.0.base_url, "https://api.anthropic.com");
        let b = AnthropicJudge::with_base_url("sk".into(), "https://proxy".into());
        assert_eq!(b.0.base_url, "https://proxy");
    }

    #[test]
    fn openai_constructors_default_base_url() {
        let a = OpenAiJudge::new("sk".into());
        assert_eq!(a.0.base_url, "https://api.openai.com/v1");
        let b = OpenAiJudge::with_base_url("sk".into(), "http://localhost:11434/v1".into());
        assert_eq!(b.0.base_url, "http://localhost:11434/v1");
    }
}
