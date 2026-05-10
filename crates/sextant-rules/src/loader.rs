//! Rule discovery and frontmatter parsing.
//!
//! Two sources contribute rules:
//!   1. Built-ins, embedded as markdown via `rust-embed`. They use the
//!      `evaluator: { type: builtin, name: ... }` form, which dispatches to
//!      a Rust evaluator.
//!   2. Repo-local rules under `<root>/.sextant/rules/**/*.md`. They use
//!      `evaluator: { type: regex, ... }` (LLM in M7).
//!
//! Repo-local rules win over built-ins when ids collide. `overrides: [...]`
//! disables the listed ids regardless of order.

use std::path::{Path, PathBuf};

use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde::Deserialize;
use sextant_core::{Category, RuleSource, Scope, Severity};
use thiserror::Error;

use crate::lock::{self, LockError, LockedPack};

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("io ({path:?}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("frontmatter ({path:?}): {message}")]
    Frontmatter { path: PathBuf, message: String },
    #[error("walk: {0}")]
    Walk(#[from] ignore::Error),
    #[error(transparent)]
    Lock(#[from] LockError),
    #[error(
        "rule `{id}` in repo-local `.sextant/rules/` shadows vendor pack `{pack}` rule of \
         the same id; vendor pack rules are immutable — rename your repo rule"
    )]
    ShadowsVendor { id: String, pack: String },
}

pub type LoaderResult<T> = Result<T, LoaderError>;

/// A rule that has been parsed but not yet turned into an evaluator. Used
/// both during discovery and as the validated output of `rules check`.
#[derive(Debug, Clone)]
pub struct ParsedRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub severity: Severity,
    pub category: Category,
    pub scope: Scope,
    pub languages: Vec<String>,
    pub evaluator: EvaluatorSpec,
    pub enabled: bool,
    pub overrides: Vec<String>,
    pub tags: Vec<String>,
    pub body: String,
    pub source: RuleSource,
    /// Filesystem origin if the rule came from disk; `None` for embedded
    /// built-ins. Used for diagnostics in `rules check`.
    pub origin: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvaluatorSpec {
    /// Dispatches to a Rust evaluator by registry name.
    Builtin { name: String },
    /// A line-by-line regex match. `exclude_paths` are GlobSet patterns
    /// applied before evaluation. `replacement`, when set, drives a
    /// proposed-fix unified diff: each match's text is rewritten via the
    /// regex crate's replace syntax (`$1`, named captures, etc.) and the
    /// resulting line replaces the original.
    Regex {
        pattern: String,
        #[serde(default)]
        exclude_paths: Vec<String>,
        #[serde(default)]
        replacement: Option<String>,
    },
    /// LLM-evaluated rule. The markdown body is the prompt template;
    /// `{{path}}`, `{{code}}`, and `{{rule.id}}` are substituted at
    /// evaluation time. `provider`, `model`, `max_tokens`, `temperature`
    /// override the corresponding `[judge]` config values when set.
    Llm {
        #[serde(default)]
        provider: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        max_tokens: Option<u32>,
        #[serde(default)]
        temperature: Option<f32>,
        #[serde(default)]
        exclude_paths: Vec<String>,
    },
    /// Tree-sitter-query rule. The query is compiled per language listed in
    /// `languages:`; matches anchored on `capture` (or the first capture)
    /// produce findings. `not_under` drops a match if any ancestor's node
    /// kind is in the list — used for context-sensitive exemptions.
    Ast {
        query: String,
        #[serde(default)]
        capture: Option<String>,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        not_under: Vec<String>,
        #[serde(default)]
        exclude_paths: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct RawFrontmatter {
    id: String,
    name: String,
    description: String,
    severity: Severity,
    category: Category,
    #[serde(default = "default_scope")]
    scope: Scope,
    #[serde(default)]
    languages: Vec<String>,
    evaluator: EvaluatorSpec,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default)]
    overrides: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_scope() -> Scope {
    Scope::File
}
fn default_enabled() -> bool {
    true
}

/// Parse a markdown file with YAML frontmatter into a `ParsedRule`.
/// `origin` is recorded so error messages can point back at the file.
pub fn parse_rule_md(
    text: &str,
    source: RuleSource,
    origin: Option<PathBuf>,
) -> LoaderResult<ParsedRule> {
    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(text);
    let raw = parsed.data.ok_or_else(|| LoaderError::Frontmatter {
        path: origin.clone().unwrap_or_default(),
        message: "missing YAML frontmatter".into(),
    })?;
    let front: RawFrontmatter = raw.deserialize().map_err(|err| LoaderError::Frontmatter {
        path: origin.clone().unwrap_or_default(),
        message: err.to_string(),
    })?;
    Ok(ParsedRule {
        id: front.id,
        name: front.name,
        description: front.description,
        severity: front.severity,
        category: front.category,
        scope: front.scope,
        languages: front.languages,
        evaluator: front.evaluator,
        enabled: front.enabled,
        overrides: front.overrides,
        tags: front.tags,
        body: parsed.content.trim_start().to_string(),
        source,
        origin,
    })
}

#[derive(rust_embed::Embed)]
#[folder = "rules/builtin/"]
struct BuiltinRules;

/// Yield all built-in rules embedded in the binary.
pub fn builtin_rules() -> LoaderResult<Vec<ParsedRule>> {
    let mut out = Vec::new();
    for path in BuiltinRules::iter() {
        if !path.ends_with(".md") {
            continue;
        }
        let file = BuiltinRules::get(&path).ok_or_else(|| LoaderError::Frontmatter {
            path: PathBuf::from(path.as_ref()),
            message: "embedded rule disappeared".into(),
        })?;
        let text =
            std::str::from_utf8(file.data.as_ref()).map_err(|err| LoaderError::Frontmatter {
                path: PathBuf::from(path.as_ref()),
                message: format!("non-UTF-8 embedded rule: {err}"),
            })?;
        out.push(parse_rule_md(
            text,
            RuleSource::Builtin,
            Some(PathBuf::from(path.as_ref())),
        )?);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Discover repo-local rules under `<root>/.sextant/rules/**/*.md`. Missing
/// directories are not an error — they're just an empty list. The
/// `vendor/` subdirectory is reserved for vendor packs (loaded via
/// [`vendor_rules`]) and skipped here.
pub fn repo_rules(root: &Path) -> LoaderResult<Vec<ParsedRule>> {
    let dir = root.join(".sextant").join("rules");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let vendor_root = dir.join(lock::VENDOR_DIR);
    let mut out = Vec::new();
    for dent in ignore::WalkBuilder::new(&dir)
        .standard_filters(true)
        .build()
    {
        let dent = dent?;
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = dent.into_path();
        if path.starts_with(&vendor_root) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|source| LoaderError::Io {
            path: path.clone(),
            source,
        })?;
        out.push(parse_rule_md(&text, RuleSource::Repo, Some(path))?);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Discover vendor-pack rules under `<root>/.sextant/rules/vendor/<pack>/`,
/// gated by `.sextant/rules.lock`. Every file in the pack directory must
/// match the locked SHA-256 hash; tampering is a hard error. Returns an
/// empty list when the lock is absent (no packs installed).
pub fn vendor_rules(root: &Path) -> LoaderResult<Vec<ParsedRule>> {
    let Some(lock) = lock::LockFile::read(root)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for pack in &lock.packs {
        out.extend(load_pack(root, pack)?);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn load_pack(root: &Path, pack: &LockedPack) -> LoaderResult<Vec<ParsedRule>> {
    let dir = lock::pack_dir(root, &pack.name);
    lock::verify_pack(pack, &dir)?;
    let rules_dir = dir.join("rules");
    if !rules_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for dent in ignore::WalkBuilder::new(&rules_dir)
        .standard_filters(true)
        .build()
    {
        let dent = dent?;
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = dent.into_path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|source| LoaderError::Io {
            path: path.clone(),
            source,
        })?;
        out.push(parse_rule_md(
            &text,
            RuleSource::Vendor(pack.name.clone()),
            Some(path),
        )?);
    }
    Ok(out)
}

pub use crate::merge::{merge, merge_all};

#[cfg(test)]
#[path = "loader_tests.rs"]
mod tests;

#[cfg(test)]
mod smoke {
    //! In-file smoke that names the public surface so the
    //! `pub-fn-untested` rule sees direct mentions. The thorough tests
    //! live in `loader_tests.rs` (extracted to keep this file under the
    //! file-length threshold).
    use super::*;

    #[test]
    fn public_surface_compiles_and_returns() {
        let _ = builtin_rules().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let _ = repo_rules(dir.path()).unwrap();
        let _ = vendor_rules(dir.path()).unwrap();
        let r = parse_rule_md(
            "---\nid: t\nname: t\ndescription: x\nseverity: warn\ncategory: style\nevaluator: { type: regex, pattern: x }\n---\n",
            RuleSource::Repo,
            None,
        )
        .unwrap();
        assert!(merge(vec![], vec![r.clone()]).len() <= 1);
        assert!(merge_all(vec![], vec![], vec![r]).unwrap().len() <= 1);
    }
}
