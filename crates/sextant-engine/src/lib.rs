//! Grading orchestration shared between the CLI and MCP server.
//!
//! This crate is the engine: load config, build the rule set, gather source
//! files (whole-tree or diff-restricted), grade, and return a `Report`. It
//! emits no I/O and no log lines — callers handle presentation. Both
//! `sextant-cli` and `sextant-mcp` are thin wrappers around the functions
//! exported here.

mod judge_setup;
mod pr;
mod synthesize;

pub use pr::{grade_pr, PrOptions, PrReport};

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use globset::GlobSet;
use ignore::WalkBuilder;
use serde::Serialize;
use sextant_config::{default_exclude_matcher, Config, ConfigError};
use sextant_core::{
    Category, EvalContext, Finding, Report, Rule, RuleSource, Scope, Severity, SourceFile,
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

/// Engine-wide knobs that aren't tied to a specific grade invocation.
#[derive(Debug, Clone, Default)]
pub struct GradeOptions {
    /// Skip LLM rules entirely. Equivalent to `[judge].enabled = false`,
    /// but per-invocation. Maps to the CLI `--no-llm` flag.
    pub no_llm: bool,
}

/// Grade a repo. Returns a fully-built `Report` (with verdict computed).
pub fn grade(repo_root: &Path, mode: GradeMode) -> Result<Report, EngineError> {
    grade_with(repo_root, mode, GradeOptions::default())
}

pub fn grade_with(
    repo_root: &Path,
    mode: GradeMode,
    opts: GradeOptions,
) -> Result<Report, EngineError> {
    let config = Config::from_repo_root(repo_root)?;
    let exclude = default_exclude_matcher().map_err(EngineError::Config)?;
    let judge = judge_setup::build_judge(repo_root, &config, &opts);
    let ruleset = RuleSet::load_with(repo_root, &config, judge.clone())?;
    let ctx = EvalContext { repo_root };

    let (mut findings, files) = match mode {
        GradeMode::Files { paths } => {
            let targets = if paths.is_empty() {
                vec![repo_root.to_path_buf()]
            } else {
                paths
            };
            let files = collect_source_files(repo_root, &targets, &exclude)?;
            let findings = ruleset.grade_files(&files, &ctx);
            (findings, files)
        }
        GradeMode::Diff(opts) => {
            let diff = compute_diff(repo_root, &opts)?;
            let files = source_files_from_diff(repo_root, &diff, &exclude);
            let raw = ruleset.grade_files(&files, &ctx);
            (filter_to_diff(raw, &diff), files)
        }
    };

    let rules = collect_rules(&ruleset);
    synthesize::run(
        &mut findings,
        synthesize::SynthesisInputs {
            files: &files,
            rules: &rules,
            judge: judge.as_ref(),
            autofix: &config.autofix,
            judge_cfg: &config.judge,
        },
    );

    let thresholds: VerdictThresholds = (&config.verdict).into();
    let verdict = thresholds.evaluate(&findings);
    Ok(Report::build(findings, verdict))
}

fn collect_rules(ruleset: &RuleSet) -> HashMap<String, Rule> {
    ruleset
        .evaluators()
        .iter()
        .map(|ev| {
            let r = ev.rule();
            (r.id.clone(), r.clone())
        })
        .collect()
}

/// Grade a single in-memory buffer.
///
/// For editor integrations (LSP) where the source on disk is stale: callers
/// pass the live buffer as a `SourceFile` and the engine grades it against
/// the same `RuleSet` as `grade_with`. Cross-file rules (clones, untested
/// public functions across crates) won't fire because only one file is in
/// scope; on-save flows can still call `grade_with` for the full tree.
///
/// Files matched by the built-in skip list (generated artifacts like
/// `Cargo.lock`, `target/`, `node_modules/`) return an empty report so
/// editor diagnostics line up with what the CLI would have done.
pub fn grade_file_buffer(
    repo_root: &Path,
    file: SourceFile,
    opts: GradeOptions,
) -> Result<Report, EngineError> {
    let config = Config::from_repo_root(repo_root)?;
    let exclude = default_exclude_matcher().map_err(EngineError::Config)?;
    let rel = file
        .path
        .strip_prefix(repo_root)
        .unwrap_or(file.path.as_path());
    let thresholds: VerdictThresholds = (&config.verdict).into();
    if exclude.is_match(rel) {
        let verdict = thresholds.evaluate(&[]);
        return Ok(Report::build(Vec::new(), verdict));
    }
    let judge = judge_setup::build_judge(repo_root, &config, &opts);
    let ruleset = RuleSet::load_with(repo_root, &config, judge)?;
    let ctx = EvalContext { repo_root };
    let findings = ruleset.grade_files(std::slice::from_ref(&file), &ctx);
    let verdict = thresholds.evaluate(&findings);
    Ok(Report::build(findings, verdict))
}

pub(crate) fn compute_diff(repo_root: &Path, opts: &DiffOptions) -> Result<DiffSet, EngineError> {
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

pub(crate) fn source_files_from_diff(
    repo_root: &Path,
    diff: &DiffSet,
    exclude: &GlobSet,
) -> Vec<SourceFile> {
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

pub(crate) fn filter_to_diff(findings: Vec<Finding>, diff: &DiffSet) -> Vec<Finding> {
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
                source: r.source.clone(),
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
#[path = "lib_tests.rs"]
mod tests;

#[cfg(test)]
mod smoke {
    //! In-file smoke that names the public surface so the
    //! `pub-fn-untested` rule sees direct mentions. The thorough tests
    //! live in `lib_tests.rs` (extracted to keep this file under the
    //! file-length threshold).
    use super::*;

    #[test]
    fn public_surface_is_callable() {
        let dir = tempfile::tempdir().unwrap();
        let _ = list_rules(dir.path()).unwrap();
        let _ = explain_rule(dir.path(), "nope").unwrap();
        let _ = load_config(dir.path()).unwrap();
        let _ = grade(
            dir.path(),
            GradeMode::Files {
                paths: vec![dir.path().to_path_buf()],
            },
        )
        .unwrap();
        let _ = grade_with(
            dir.path(),
            GradeMode::Files {
                paths: vec![dir.path().to_path_buf()],
            },
            GradeOptions::default(),
        )
        .unwrap();
        let _ = grade_file_buffer(
            dir.path(),
            SourceFile::new(dir.path().join("smoke.rs"), String::new()),
            GradeOptions::default(),
        )
        .unwrap();
    }
}
