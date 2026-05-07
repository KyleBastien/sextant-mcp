//! Config loading for Sextant.
//!
//! In M1 this is just a struct read from `.sextant/config.toml`; later
//! milestones will layer env-var overrides and per-rule threshold tables.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sextant_core::VerdictThresholds;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub verdict: VerdictSection,
    pub size: SizeRuleConfig,
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
}

impl Default for SizeRuleConfig {
    fn default() -> Self {
        Self {
            file_length_warn: 400,
            file_length_error: 800,
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
}
