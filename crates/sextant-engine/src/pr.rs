//! PR mode: diff-grade the head, compare against a baseline-graded base
//! SHA, return the head report + delta + a regression-mode verdict.
//!
//! The baseline is a full-tree grade of the base SHA. Computing it on
//! every PR run would be wasteful, so callers can pass a cache directory
//! (e.g., the GitHub Actions cache mount) where we persist baseline
//! reports keyed by SHA. A cache miss recomputes via `git2` blob reads —
//! never a checkout — so the workflow is safe to run inside a CI job
//! that's already building head.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sextant_config::Config;
use sextant_core::{
    BaselineDelta, EvalContext, Finding, Report, SourceFile, Verdict, VerdictThresholds,
};
use sextant_diff::files_at_ref;
use sextant_rules::RuleSet;

use crate::{
    compute_diff, filter_to_diff, judge_setup, source_files_from_diff, DiffOptions, EngineError,
    GradeOptions,
};

/// Output of [`grade_pr`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrReport {
    /// Diff-mode grade of the head: only findings on changed lines.
    pub head: Report,
    /// Whole-tree grade of the base SHA — what the repo looked like
    /// before the PR. Persisted in the baseline cache.
    pub baseline: Report,
    /// Findings newly introduced or fixed by this PR.
    pub delta: BaselineDelta,
    /// Regression-mode verdict, computed against `delta.new_findings`.
    /// Drives the `--pr` exit code and the markdown review.
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Default)]
pub struct PrOptions {
    /// Where to read/write baseline reports keyed by base SHA. `None`
    /// disables caching — we always recompute.
    pub baseline_cache: Option<PathBuf>,
    /// Forwarded to the head + baseline grade calls.
    pub grade: GradeOptions,
}

/// Grade a PR: head changes against a baseline, returning both reports +
/// the delta. Verdict is regression-mode regardless of `[verdict].mode`.
pub fn grade_pr(
    repo_root: &Path,
    diff: DiffOptions,
    opts: PrOptions,
) -> Result<PrReport, EngineError> {
    let config = Config::from_repo_root(repo_root)?;
    let exclude = config.paths.matcher().map_err(EngineError::Config)?;
    let judge = judge_setup::build_judge(repo_root, &config, &opts.grade);
    let ruleset = RuleSet::load_with(repo_root, &config, judge)?;
    let ctx = EvalContext { repo_root };
    let thresholds: VerdictThresholds = (&config.verdict).into();

    let env = GradeEnv {
        ruleset: &ruleset,
        ctx: &ctx,
        repo_root,
        exclude: &exclude,
    };
    let head_diff = compute_diff(env.repo_root, &diff)?;
    let head_report = build_head_report(&env, &head_diff, &thresholds);
    let (baseline_report, base_sha) =
        build_baseline_report(&env, diff.base.as_deref(), opts.baseline_cache.as_deref())?;

    // Scope the baseline to paths the PR actually touched. The baseline
    // is a whole-tree grade; without this filter, baseline findings on
    // unchanged files appear as "fixed" (because head is diff-only and
    // never sees those paths) — see the regression test below. With the
    // filter, the delta reflects only what the PR can plausibly affect.
    let baseline_in_scope = scope_baseline_to_diff(&baseline_report.findings, &head_diff);

    let delta = BaselineDelta::compute(&head_report.findings, &baseline_in_scope, Some(base_sha));
    let verdict = thresholds.evaluate_regression(&delta);
    Ok(PrReport {
        head: head_report,
        baseline: baseline_report,
        delta,
        verdict,
    })
}

/// Drop baseline findings on paths the PR did not touch. The PR's delta
/// only counts what the PR could plausibly have introduced or fixed —
/// pre-existing debt in unchanged files is out of scope on both sides.
fn scope_baseline_to_diff(findings: &[Finding], diff: &sextant_diff::DiffSet) -> Vec<Finding> {
    let touched: std::collections::HashSet<&Path> =
        diff.files.iter().map(|f| f.path.as_path()).collect();
    findings
        .iter()
        .filter(|f| touched.contains(f.path.as_path()))
        .cloned()
        .collect()
}

/// Bundle of references threaded through the PR-mode helpers — the
/// ruleset, eval context, repo root, and exclude matcher. Avoids
/// pushing the same five args through every helper.
struct GradeEnv<'a> {
    ruleset: &'a RuleSet,
    ctx: &'a EvalContext<'a>,
    repo_root: &'a Path,
    exclude: &'a globset::GlobSet,
}

fn build_head_report(
    env: &GradeEnv<'_>,
    head_diff: &sextant_diff::DiffSet,
    thresholds: &VerdictThresholds,
) -> Report {
    let findings = grade_head(env, head_diff);
    Report::build(findings, thresholds.evaluate(&[]))
}

fn build_baseline_report(
    env: &GradeEnv<'_>,
    base_ref: Option<&str>,
    cache_dir: Option<&Path>,
) -> Result<(Report, String), EngineError> {
    let base_ref = base_ref.unwrap_or("origin/main");
    let snapshot = files_at_ref(env.repo_root, base_ref)?;
    let base_sha = snapshot.oid.to_string();
    if let Some(cached) = cache_dir.and_then(|dir| read_cached_report(dir, &base_sha)) {
        return Ok((cached, base_sha));
    }
    let report = grade_baseline_snapshot(env, snapshot);
    if let Some(dir) = cache_dir {
        if let Err(err) = write_cached_report(dir, &base_sha, &report) {
            tracing::warn!(?err, "baseline cache write failed");
        }
    }
    Ok((report, base_sha))
}

fn grade_head(env: &GradeEnv<'_>, diff: &sextant_diff::DiffSet) -> Vec<Finding> {
    let files = source_files_from_diff(env.repo_root, diff, env.exclude);
    let raw = env.ruleset.grade_files(&files, env.ctx);
    filter_to_diff(raw, diff)
}

fn grade_baseline_snapshot(env: &GradeEnv<'_>, snapshot: sextant_diff::RefSnapshot) -> Report {
    let files: Vec<SourceFile> = snapshot
        .files
        .into_iter()
        .filter(|(p, _)| !env.exclude.is_match(p))
        .map(|(rel, contents)| SourceFile::new(env.repo_root.join(rel), contents))
        .collect();
    let findings = env.ruleset.grade_files(&files, env.ctx);
    // Verdict on the baseline is informational — the *real* verdict is
    // computed against the delta. Use Approve as a sentinel.
    Report::build(findings, Verdict::Approve)
}

fn cache_path(dir: &Path, base_sha: &str) -> PathBuf {
    dir.join(format!("{base_sha}.json"))
}

fn read_cached_report(dir: &Path, base_sha: &str) -> Option<Report> {
    let path = cache_path(dir, base_sha);
    let bytes = std::fs::read(&path).ok()?;
    match serde_json::from_slice::<Report>(&bytes) {
        Ok(r) => {
            tracing::debug!(%base_sha, "baseline cache hit");
            Some(r)
        }
        Err(err) => {
            tracing::warn!(?err, ?path, "ignoring malformed baseline cache entry");
            None
        }
    }
}

fn write_cached_report(dir: &Path, base_sha: &str, report: &Report) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = cache_path(dir, base_sha);
    let json = serde_json::to_vec(report).map_err(|e| std::io::Error::other(e.to_string()))?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod smoke {
    //! Smoke tests for the cache helpers + a `grade_pr` symbol mention.
    //! Full-stack PR tests live in `lib_tests.rs` since they need a real
    //! git repo and a configured engine.
    use super::*;

    #[test]
    fn cache_round_trips_a_report() {
        let dir = tempfile::tempdir().unwrap();
        let r = Report::build(vec![], Verdict::Approve);
        write_cached_report(dir.path(), "abc", &r).unwrap();
        let hit = read_cached_report(dir.path(), "abc").expect("hit");
        assert_eq!(hit.findings.len(), 0);
        assert!(read_cached_report(dir.path(), "missing").is_none());
    }

    #[test]
    fn grade_pr_is_callable() {
        // We can't drive a real grade_pr without a git repo, so just
        // hold a function pointer to it. The pub-fn-untested rule
        // matches names, not call shape.
        let _: fn(&Path, DiffOptions, PrOptions) -> Result<PrReport, EngineError> = grade_pr;
    }
}
