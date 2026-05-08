//! LLM-as-judge providers for Sextant.
//!
//! A `Judge` wraps a [`JudgeProvider`] (Anthropic, OpenAI, or any
//! OpenAI-compatible endpoint) with a content-addressed disk cache and a
//! synchronous entry point that the rule layer can call without being
//! async itself. The cache key folds in provider, model, rule body, and
//! the normalized code window — so identical reviews never hit the
//! network twice across runs.
//!
//! Errors are surfaced as `JudgeError` so callers can decide whether to
//! degrade them to `info` findings (the policy) or fail loudly.

mod cache;
mod fake;
mod providers;

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use cache::{Cache, CacheError};
pub use fake::FakeJudge;
pub use providers::{AnthropicJudge, OpenAiJudge};

/// Severity hint produced by the judge. Mirrors `sextant_core::Severity`
/// but is parsed from JSON so we keep it as our own type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JudgeSeverity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JudgeFinding {
    pub severity: JudgeSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JudgeResult {
    pub findings: Vec<JudgeFinding>,
}

/// One unit of work for a provider. The rule layer renders the prompt
/// from a markdown body and passes it here.
#[derive(Debug, Clone)]
pub struct JudgeRequest<'a> {
    pub system_prompt: Option<&'a str>,
    pub user_prompt: &'a str,
    pub model: &'a str,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Error)]
pub enum JudgeError {
    #[error("http: {0}")]
    Http(String),
    #[error("api error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("response shape: {0}")]
    Parse(String),
    #[error("missing api key in env var `{0}`")]
    MissingKey(String),
    #[error(transparent)]
    Cache(#[from] CacheError),
    #[error("judge disabled (no provider configured)")]
    Disabled,
}

#[async_trait::async_trait]
pub trait JudgeProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn judge(&self, req: JudgeRequest<'_>) -> Result<JudgeResult, JudgeError>;
}

/// Public-facing wrapper. Holds a provider + cache + an internal Tokio
/// runtime so the synchronous rule layer can call `judge_blocking`.
pub struct Judge {
    provider: Arc<dyn JudgeProvider>,
    cache: Cache,
    runtime: tokio::runtime::Runtime,
}

impl Judge {
    pub fn new(provider: Arc<dyn JudgeProvider>, cache_dir: PathBuf) -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            provider,
            cache: Cache::new(cache_dir),
            runtime,
        })
    }

    pub fn provider_name(&self) -> &'static str {
        self.provider.name()
    }

    /// Synchronous entry point used by the rule layer. Tries the cache
    /// first, falls back to the live provider, and writes successful
    /// results back. Cache write errors are logged but don't fail the
    /// call — a transient disk hiccup shouldn't sink a grade.
    pub fn judge_blocking(&self, req: JudgeRequest<'_>) -> Result<JudgeResult, JudgeError> {
        let key = Cache::key(self.provider.name(), req.model, req.user_prompt);
        if let Some(hit) = self.cache.get(&key)? {
            tracing::debug!(provider = self.provider.name(), %key, "judge cache hit");
            return Ok(hit);
        }
        let res = self.runtime.block_on(self.provider.judge(req))?;
        if let Err(err) = self.cache.put(&key, &res) {
            tracing::warn!(?err, "judge cache write failed");
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req<'a>(prompt: &'a str) -> JudgeRequest<'a> {
        JudgeRequest {
            system_prompt: None,
            user_prompt: prompt,
            model: "m",
            max_tokens: 64,
            temperature: 0.0,
        }
    }

    fn result_with(msg: &str) -> JudgeResult {
        JudgeResult {
            findings: vec![JudgeFinding {
                severity: JudgeSeverity::Warn,
                message: msg.into(),
                line: None,
                end_line: None,
            }],
        }
    }

    #[test]
    fn provider_name_forwards_to_inner_provider() {
        let dir = tempfile::tempdir().unwrap();
        let provider = Arc::new(FakeJudge::always("fake", JudgeResult { findings: vec![] }));
        let judge = Judge::new(provider, dir.path().to_path_buf()).unwrap();
        assert_eq!(judge.provider_name(), "fake");
    }

    #[test]
    fn judge_blocking_caches_results() {
        let dir = tempfile::tempdir().unwrap();
        // Two responses available; if the cache works we should only
        // ever pull the first.
        let provider = Arc::new(FakeJudge::new(
            "fake",
            vec![result_with("first"), result_with("second")],
        ));
        let captured = Arc::clone(&provider);
        let judge = Judge::new(provider, dir.path().to_path_buf()).unwrap();

        let a = judge.judge_blocking(req("hello")).unwrap();
        assert_eq!(a.findings[0].message, "first");
        let b = judge.judge_blocking(req("hello")).unwrap();
        assert_eq!(b.findings[0].message, "first", "expected cache hit");
        assert_eq!(
            captured.received().len(),
            1,
            "second call should be served from cache"
        );

        let c = judge.judge_blocking(req("different")).unwrap();
        assert_eq!(c.findings[0].message, "second");
    }
}
