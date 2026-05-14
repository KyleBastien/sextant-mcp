//! Config loading for Sextant.
//!
//! In M1 this is just a struct read from `.sextant/config.toml`; later
//! milestones will layer env-var overrides and per-rule threshold tables.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use sextant_core::{VerdictMode, VerdictThresholds};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid glob `{pattern}`: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub verdict: VerdictSection,
    pub size: SizeRuleConfig,
    pub complexity: ComplexityRuleConfig,
    pub duplication: DuplicationRuleConfig,
    pub judge: JudgeConfig,
    pub autofix: AutofixConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VerdictSection {
    pub max_errors: u32,
    pub max_warns: u32,
    pub max_info: u32,
    /// `absolute` (default) counts every finding; `regression` only
    /// counts findings new vs the baseline. `--pr` overrides this to
    /// `regression` regardless of the file value.
    pub mode: VerdictMode,
}

impl Default for VerdictSection {
    fn default() -> Self {
        Self {
            max_errors: 0,
            max_warns: u32::MAX,
            max_info: u32::MAX,
            mode: VerdictMode::Absolute,
        }
    }
}

impl From<&VerdictSection> for VerdictThresholds {
    fn from(v: &VerdictSection) -> Self {
        VerdictThresholds {
            max_errors: v.max_errors,
            max_warns: v.max_warns,
            max_info: v.max_info,
        }
    }
}

/// Per-rule thresholds for the built-in size rules. Kept under `[size]`
/// rather than nested under each rule id so the common case stays terse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SizeRuleConfig {
    pub file_length_warn: u32,
    pub file_length_error: u32,
    pub fn_length_warn: u32,
    pub fn_length_error: u32,
    pub param_count_warn: u32,
    pub param_count_error: u32,
}

impl Default for SizeRuleConfig {
    fn default() -> Self {
        Self {
            file_length_warn: 400,
            file_length_error: 800,
            fn_length_warn: 60,
            fn_length_error: 120,
            param_count_warn: 6,
            param_count_error: 10,
        }
    }
}

/// Per-rule thresholds for the built-in complexity rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ComplexityRuleConfig {
    pub cyclomatic_warn: u32,
    pub cyclomatic_error: u32,
    pub nesting_warn: u32,
    pub nesting_error: u32,
}

impl Default for ComplexityRuleConfig {
    fn default() -> Self {
        Self {
            cyclomatic_warn: 10,
            cyclomatic_error: 20,
            nesting_warn: 4,
            nesting_error: 6,
        }
    }
}

/// Per-rule thresholds for the built-in duplication detector.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DuplicationRuleConfig {
    /// Minimum token-window size that qualifies as a clone. Smaller values
    /// catch more (and noisier) duplication; larger values flag only
    /// substantial copy-paste. The default is calibrated to roughly the
    /// "10 lines of typical code" mark.
    pub min_tokens: u32,
}

impl Default for DuplicationRuleConfig {
    fn default() -> Self {
        // 100 tokens is roughly 20 lines of typical code — what other
        // duplication tools (Sonar, CodeScene) calibrate "substantial"
        // duplication to. Lower it to surface more, raise it for less noise.
        Self { min_tokens: 100 }
    }
}

/// The hardcoded list of paths sextant never grades — generated and
/// vendored files (`Cargo.lock`, build outputs, dependency directories,
/// `.git`). Baked into the engine so the choice of what to skip is not a
/// configuration knob agents can edit to hide findings.
const DEFAULT_EXCLUDES: &[&str] = &[
    "**/Cargo.lock",
    "**/package-lock.json",
    "**/yarn.lock",
    "**/pnpm-lock.yaml",
    "**/poetry.lock",
    "**/uv.lock",
    "**/target/**",
    "**/node_modules/**",
    "**/dist/**",
    "**/build/**",
    "**/.git/**",
    "**/.sextant/cache/**",
];

/// Compile the hardcoded skip list into a `GlobSet`. Sextant never grades
/// these paths (generated artifacts, vendored deps); the set is not
/// user-configurable on purpose.
pub fn default_exclude_matcher() -> Result<GlobSet, ConfigError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in DEFAULT_EXCLUDES {
        let glob = Glob::new(pattern).map_err(|source| ConfigError::Glob {
            pattern: (*pattern).to_string(),
            source,
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|source| ConfigError::Glob {
        pattern: "<set>".into(),
        source,
    })
}

/// LLM-as-judge configuration. Defaults to disabled — projects opt in by
/// setting `provider` to one of `anthropic`, `openai`, or `openai-compatible`.
/// `api_key_env` names the env var holding the credential; storing the key
/// itself in the file would be a footgun.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JudgeConfig {
    pub enabled: bool,
    pub provider: JudgeProvider,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub api_key_env: String,
    /// Used by `openai-compatible` to point at Ollama, vLLM, etc. The
    /// `openai` provider also honours this if set.
    pub base_url: Option<String>,
    /// Cap on parallel in-flight LLM calls. Currently unused (rules run
    /// per-file sequentially), reserved for future bounded concurrency.
    pub max_concurrent: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JudgeProvider {
    #[default]
    None,
    Anthropic,
    Openai,
    OpenaiCompatible,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: JudgeProvider::None,
            model: "claude-sonnet-4-6".into(),
            max_tokens: 1024,
            temperature: 0.0,
            api_key_env: "ANTHROPIC_API_KEY".into(),
            base_url: None,
            max_concurrent: 4,
        }
    }
}

/// Autofix-pass tuning. By default, native generators (regex
/// `replacement`, pub_fn_test stubs, LLM-rule patches) are on and the
/// LLM-synthesis fallback is off. Opt in with `llm_synthesis = true` to
/// have the judge propose patches for findings whose evaluator can't
/// produce one mechanically. `max_synthesis_findings` is a cost guard so
/// a noisy run doesn't translate into a noisy bill.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutofixConfig {
    pub enabled: bool,
    pub llm_synthesis: bool,
    pub max_synthesis_findings: u32,
}

impl Default for AutofixConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_synthesis: false,
            max_synthesis_findings: 25,
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&raw)?;
        Ok(cfg)
    }

    /// Resolve a config given a repo root. Looks for `<root>/.sextant/config.toml`.
    pub fn from_repo_root(root: &Path) -> Result<Self, ConfigError> {
        Self::load_or_default(&root.join(".sextant").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spin up a temp repo, write `body` to its `.sextant/config.toml`,
    /// and return the loaded `Config`. The test holds onto the
    /// `TempDir` so files survive until the assertion phase.
    fn write_and_load(body: &str) -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".sextant").join("config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        let cfg = Config::from_repo_root(dir.path()).unwrap();
        (dir, cfg)
    }

    #[test]
    fn defaults_load_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::from_repo_root(dir.path()).unwrap();
        assert_eq!(cfg.verdict.max_errors, 0);
        assert_eq!(cfg.size.file_length_warn, 400);
    }

    #[test]
    fn load_or_default_returns_default_for_nonexistent_path() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::load_or_default(&dir.path().join("nope.toml")).unwrap();
        assert_eq!(cfg.size.file_length_warn, 400);
    }

    #[test]
    fn parses_overrides() {
        let (_dir, cfg) = write_and_load(
            "[verdict]\nmax_errors = 2\nmax_warns = 5\nmax_info = 0\n[size]\nfile_length_warn = 100\nfile_length_error = 200\n",
        );
        assert_eq!(cfg.verdict.max_errors, 2);
        assert_eq!(cfg.verdict.max_warns, 5);
        assert_eq!(cfg.verdict.max_info, 0);
        assert_eq!(cfg.size.file_length_warn, 100);
        assert_eq!(cfg.size.file_length_error, 200);
    }

    #[test]
    fn default_path_matcher_excludes_generated_files() {
        let m = default_exclude_matcher().unwrap();
        assert!(m.is_match("Cargo.lock"));
        assert!(m.is_match("crates/foo/Cargo.lock"));
        assert!(m.is_match("target/debug/build/foo"));
        assert!(m.is_match("node_modules/some-pkg/index.js"));
        assert!(!m.is_match("src/main.rs"));
        assert!(!m.is_match("crates/sextant-core/src/lib.rs"));
    }

    #[test]
    fn autofix_defaults_keep_llm_synthesis_off() {
        let cfg = Config::default();
        assert!(cfg.autofix.enabled);
        assert!(!cfg.autofix.llm_synthesis);
        assert_eq!(cfg.autofix.max_synthesis_findings, 25);
    }

    #[test]
    fn autofix_section_round_trips() {
        let (_dir, cfg) =
            write_and_load("[autofix]\nllm_synthesis = true\nmax_synthesis_findings = 5\n");
        assert!(cfg.autofix.llm_synthesis);
        assert_eq!(cfg.autofix.max_synthesis_findings, 5);
    }

    #[test]
    fn unknown_paths_section_is_ignored() {
        // The `[paths]` config section was removed. Existing repos that
        // still ship it must keep loading cleanly — serde silently drops
        // the unknown key.
        let (_dir, cfg) = write_and_load("[paths]\nexclude = [\"**/secret/**\"]\n");
        let _ = cfg;
        let m = default_exclude_matcher().unwrap();
        assert!(!m.is_match("a/secret/b.rs"));
        assert!(m.is_match("Cargo.lock"));
    }
}
