//! Grading orchestration shared between the CLI and MCP server.
//!
//! This crate is the engine: load config, build the rule set, gather source
//! files (whole-tree or diff-restricted), grade, and return a `Report`. It
//! emits no I/O and no log lines — callers handle presentation. Both
//! `sextant-cli` and `sextant-mcp` are thin wrappers around the functions
//! exported here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use globset::GlobSet;
use ignore::WalkBuilder;
use serde::Serialize;
use sextant_config::{Config, ConfigError};
use sextant_core::{
    Category, EvalContext, Finding, Report, RuleSource, Scope, Severity, SourceFile,
    VerdictThresholds,
};
use sextant_diff::{compute, BaseSpec, DiffError, DiffSet, HeadSpec};
use sextant_rules::{RuleSet, RuleSetError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Rules(#[from] RuleSetError),
    #[error(transparent)]
    Diff(#[from] DiffError),
    #[error("io ({path:?}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// What to grade.
#[derive(Debug, Clone)]
pub enum GradeMode {
    /// Whole-file grade. Walk `paths` (defaulting to the repo root if empty).
    Files { paths: Vec<PathBuf> },
    /// Diff grade. Findings whose head-side line range doesn't intersect a
    /// changed line are dropped.
    Diff(DiffOptions),
}

#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Base ref. `None` = `merge-base origin/main HEAD` with HEAD~1 fallback.
    pub base: Option<String>,
    /// Head ref. `None` = working tree (with index applied).
    pub head: Option<String>,
    /// Force the working-tree head even if `head` is set.
    pub working_tree: bool,
}

/// Grade a repo. Returns a fully-built `Report` (with verdict computed).
pub fn grade(repo_root: &Path, mode: GradeMode) -> Result<Report, EngineError> {
    let config = Config::from_repo_root(repo_root)?;
    let exclude = config.paths.matcher().map_err(EngineError::Config)?;
    let ruleset = RuleSet::load(repo_root, &config)?;
    let ctx = EvalContext { repo_root };

    let findings = match mode {
        GradeMode::Files { paths } => {
            let targets = if paths.is_empty() {
                vec![repo_root.to_path_buf()]
            } else {
                paths
            };
            let files = collect_source_files(repo_root, &targets, &exclude)?;
            ruleset.grade_files(&files, &ctx)
        }
        GradeMode::Diff(opts) => {
            let diff = compute_diff(repo_root, &opts)?;
            let files = source_files_from_diff(repo_root, &diff, &exclude);
            let raw = ruleset.grade_files(&files, &ctx);
            filter_to_diff(raw, &diff)
        }
    };

    let thresholds: VerdictThresholds = (&config.verdict).into();
    let verdict = thresholds.evaluate(&findings);
    Ok(Report::build(findings, verdict))
}

fn compute_diff(repo_root: &Path, opts: &DiffOptions) -> Result<DiffSet, EngineError> {
    let base_spec = match &opts.base {
        Some(s) => BaseSpec::Ref(s.clone()),
        None => BaseSpec::Auto,
    };
    let head_spec = if opts.working_tree {
        HeadSpec::WorkingTree
    } else {
        match &opts.head {
            Some(s) => HeadSpec::Ref(s.clone()),
            None => HeadSpec::WorkingTree,
        }
    };
    Ok(compute(repo_root, &base_spec, &head_spec)?)
}

fn source_files_from_diff(repo_root: &Path, diff: &DiffSet, exclude: &GlobSet) -> Vec<SourceFile> {
    let mut out = Vec::new();
    for f in &diff.files {
        if exclude.is_match(&f.path) {
            continue;
        }
        let Some(contents) = f.head_contents.as_ref() else {
            continue;
        };
        out.push(SourceFile::new(repo_root.join(&f.path), contents.clone()));
    }
    out
}

fn filter_to_diff(findings: Vec<Finding>, diff: &DiffSet) -> Vec<Finding> {
    let mut by_path: std::collections::HashMap<PathBuf, &BTreeSet<u32>> =
        std::collections::HashMap::new();
    for f in &diff.files {
        by_path.insert(f.path.clone(), &f.changed_lines);
    }
    findings
        .into_iter()
        .filter(|f| {
            let Some(changed) = by_path.get(&f.path) else {
                return false;
            };
            match (f.line, f.end_line) {
                (None, _) => !changed.is_empty(),
                (Some(start), end) => {
                    let end = end.unwrap_or(start);
                    (start..=end).any(|ln| changed.contains(&ln))
                }
            }
        })
        .collect()
}

fn collect_source_files(
    root: &Path,
    targets: &[PathBuf],
    exclude: &GlobSet,
) -> Result<Vec<SourceFile>, EngineError> {
    let mut out = Vec::new();
    for target in targets {
        for dent in WalkBuilder::new(target).standard_filters(true).build() {
            if let Some(file) = load_walked_file(root, dent, exclude) {
                out.push(file);
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Turn one walker entry into a `SourceFile`, or `None` if it should be
/// skipped (walk error, non-file, excluded path, unreadable contents).
fn load_walked_file(
    root: &Path,
    dent: Result<ignore::DirEntry, ignore::Error>,
    exclude: &GlobSet,
) -> Option<SourceFile> {
    let dent = dent
        .inspect_err(|err| tracing::warn!(?err, "walk error"))
        .ok()?;
    if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
        return None;
    }
    let path = dent.into_path();
    let rel = path.strip_prefix(root).unwrap_or(&path);
    if exclude.is_match(rel) {
        return None;
    }
    let contents = std::fs::read_to_string(&path)
        .inspect_err(|err| tracing::debug!(?err, ?path, "skipping unreadable"))
        .ok()?;
    let abs = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    Some(SourceFile::new(abs, contents))
}

/// A flat, serialization-friendly view of a `Rule`. Used by `list_rules` and
/// `explain_rule` so callers (CLI, MCP) don't have to reach into the trait
/// object.
#[derive(Debug, Clone, Serialize)]
pub struct RuleSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub body: String,
    pub severity: Severity,
    pub category: Category,
    pub scope: Scope,
    pub languages: Vec<String>,
    pub source: RuleSource,
    pub tags: Vec<String>,
}

pub fn list_rules(repo_root: &Path) -> Result<Vec<RuleSummary>, EngineError> {
    let config = Config::from_repo_root(repo_root)?;
    let ruleset = RuleSet::load(repo_root, &config)?;
    let mut out: Vec<RuleSummary> = ruleset
        .evaluators()
        .iter()
        .map(|ev| {
            let r = ev.rule();
            RuleSummary {
                id: r.id.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                body: r.body.clone(),
                severity: r.severity,
                category: r.category.clone(),
                scope: r.scope,
                languages: r.languages.clone(),
                source: r.source,
                tags: r.tags.clone(),
            }
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn explain_rule(repo_root: &Path, id: &str) -> Result<Option<RuleSummary>, EngineError> {
    Ok(list_rules(repo_root)?.into_iter().find(|r| r.id == id))
}

pub fn load_config(repo_root: &Path) -> Result<Config, EngineError> {
    Ok(Config::from_repo_root(repo_root)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, contents).unwrap();
    }

    #[test]
    fn grade_files_returns_findings() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            ".sextant/config.toml",
            "[size]\nfile_length_warn = 10\nfile_length_error = 20\n",
        );
        write(root, "long.rs", &"x\n".repeat(25));

        let report = grade(
            root,
            GradeMode::Files {
                paths: vec![root.to_path_buf()],
            },
        )
        .unwrap();
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule_id == "builtin.size.file-length"));
    }

    #[test]
    fn list_rules_returns_builtins() {
        let dir = tempfile::tempdir().unwrap();
        let rules = list_rules(dir.path()).unwrap();
        let ids: Vec<_> = rules.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"builtin.size.file-length"));
        assert!(ids.contains(&"builtin.size.fn-length"));
        assert!(ids.contains(&"builtin.size.param-count"));
    }

    #[test]
    fn explain_rule_returns_body() {
        let dir = tempfile::tempdir().unwrap();
        let r = explain_rule(dir.path(), "builtin.size.fn-length")
            .unwrap()
            .expect("rule found");
        assert!(r.body.contains("Function length"));
    }

    #[test]
    fn explain_unknown_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(explain_rule(dir.path(), "nope").unwrap().is_none());
    }

    #[test]
    fn load_config_reads_repo_local_overrides() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            ".sextant/config.toml",
            "[size]\nfile_length_warn = 7\n",
        );
        let cfg = load_config(dir.path()).unwrap();
        assert_eq!(cfg.size.file_length_warn, 7);
    }
}
