//! Config loading for Sextant.
//!
//! In M1 this is just a struct read from `.sextant/config.toml`; later
//! milestones will layer env-var overrides and per-rule threshold tables.

use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use sextant_core::VerdictThresholds;
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
    pub paths: PathsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VerdictSection {
    pub max_errors: u32,
    pub max_warns: u32,
}

impl Default for VerdictSection {
    fn default() -> Self {
        Self {
            max_errors: 0,
            max_warns: u32::MAX,
        }
    }
}

impl From<&VerdictSection> for VerdictThresholds {
    fn from(v: &VerdictSection) -> Self {
        VerdictThresholds {
            max_errors: v.max_errors,
            max_warns: v.max_warns,
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

/// Path filters applied before any rule runs. The default list matches
/// generated and vendored files that we never want to grade — `Cargo.lock`,
/// build outputs, dependency directories. User config replaces (not extends)
/// the default list, so projects can opt out.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    pub exclude: Vec<String>,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            exclude: vec![
                "**/Cargo.lock".into(),
                "**/package-lock.json".into(),
                "**/yarn.lock".into(),
                "**/pnpm-lock.yaml".into(),
                "**/poetry.lock".into(),
                "**/uv.lock".into(),
                "**/target/**".into(),
                "**/node_modules/**".into(),
                "**/dist/**".into(),
                "**/build/**".into(),
                "**/.git/**".into(),
                "**/.sextant/cache/**".into(),
            ],
        }
    }
}

impl PathsConfig {
    pub fn matcher(&self) -> Result<GlobSet, ConfigError> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &self.exclude {
            let glob = Glob::new(pattern).map_err(|source| ConfigError::Glob {
                pattern: pattern.clone(),
                source,
            })?;
            builder.add(glob);
        }
        builder.build().map_err(|source| ConfigError::Glob {
            pattern: "<set>".into(),
            source,
        })
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

    #[test]
    fn defaults_load_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::from_repo_root(dir.path()).unwrap();
        assert_eq!(cfg.verdict.max_errors, 0);
        assert_eq!(cfg.size.file_length_warn, 400);
    }

    #[test]
    fn parses_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".sextant").join("config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "[verdict]\nmax_errors = 2\nmax_warns = 5\n[size]\nfile_length_warn = 100\nfile_length_error = 200\n",
        )
        .unwrap();
        let cfg = Config::from_repo_root(dir.path()).unwrap();
        assert_eq!(cfg.verdict.max_errors, 2);
        assert_eq!(cfg.verdict.max_warns, 5);
        assert_eq!(cfg.size.file_length_warn, 100);
        assert_eq!(cfg.size.file_length_error, 200);
    }

    #[test]
    fn default_path_matcher_excludes_generated_files() {
        let cfg = PathsConfig::default();
        let m = cfg.matcher().unwrap();
        assert!(m.is_match("Cargo.lock"));
        assert!(m.is_match("crates/foo/Cargo.lock"));
        assert!(m.is_match("target/debug/build/foo"));
        assert!(m.is_match("node_modules/some-pkg/index.js"));
        assert!(!m.is_match("src/main.rs"));
        assert!(!m.is_match("crates/sextant-core/src/lib.rs"));
    }

    #[test]
    fn user_paths_replaces_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".sextant").join("config.toml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[paths]\nexclude = [\"**/secret/**\"]\n").unwrap();
        let cfg = Config::from_repo_root(dir.path()).unwrap();
        let m = cfg.paths.matcher().unwrap();
        assert!(m.is_match("a/secret/b.rs"));
        assert!(!m.is_match("Cargo.lock"));
    }
}
