//! Build a `Judge` from `[judge]` config.
//!
//! Failures here are non-fatal: a missing API key, an unsupported provider,
//! or a runtime build error all log a warning and return `None`. Callers
//! treat `None` the same as `--no-llm` — LLM rules just get skipped.

use std::path::Path;
use std::sync::Arc;

use sextant_config::{Config, JudgeConfig, JudgeProvider as JudgeProviderKind};
use sextant_judge::{AnthropicJudge, Judge, JudgeProvider, OpenAiJudge};

use crate::GradeOptions;

pub(crate) fn build_judge(
    repo_root: &Path,
    config: &Config,
    opts: &GradeOptions,
) -> Option<Arc<Judge>> {
    if opts.no_llm || !config.judge.enabled {
        return None;
    }
    let provider = make_provider(&config.judge)?;
    let cache_dir = repo_root.join(".sextant").join("cache").join("judge");
    Judge::new(provider, cache_dir)
        .inspect_err(|err| tracing::warn!(?err, "could not build judge runtime"))
        .ok()
        .map(Arc::new)
}

fn make_provider(cfg: &JudgeConfig) -> Option<Arc<dyn JudgeProvider>> {
    if matches!(cfg.provider, JudgeProviderKind::None) {
        return None;
    }
    let key = read_api_key(cfg)?;
    Some(match cfg.provider {
        JudgeProviderKind::Anthropic => build_anthropic(key, cfg.base_url.as_deref()),
        JudgeProviderKind::Openai | JudgeProviderKind::OpenaiCompatible => {
            build_openai(key, cfg.base_url.as_deref())
        }
        JudgeProviderKind::None => unreachable!("filtered above"),
    })
}

fn read_api_key(cfg: &JudgeConfig) -> Option<String> {
    std::env::var(&cfg.api_key_env)
        .inspect_err(|err| {
            tracing::warn!(
                provider = ?cfg.provider,
                env = %cfg.api_key_env,
                ?err,
                "skipping judge: api key missing"
            );
        })
        .ok()
}

fn build_anthropic(key: String, base_url: Option<&str>) -> Arc<dyn JudgeProvider> {
    match base_url {
        Some(url) => Arc::new(AnthropicJudge::with_base_url(key, url.into())),
        None => Arc::new(AnthropicJudge::new(key)),
    }
}

fn build_openai(key: String, base_url: Option<&str>) -> Arc<dyn JudgeProvider> {
    match base_url {
        Some(url) => Arc::new(OpenAiJudge::with_base_url(key, url.into())),
        None => Arc::new(OpenAiJudge::new(key)),
    }
}
