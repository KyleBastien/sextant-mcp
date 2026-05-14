use crate::{Finding, Rule, SourceFile};

/// Read-only context passed to every evaluator. Currently only carries the
/// repository root so paths in findings can be made relative for display;
/// will grow to include parsed trees, baseline reports, the LLM judge handle,
/// etc. as later milestones land.
#[derive(Debug, Clone)]
pub struct EvalContext<'a> {
    pub repo_root: &'a std::path::Path,
}

/// A rule implementation. Evaluators receive a single `SourceFile` and
/// emit zero or more findings.
pub trait Evaluator: Send + Sync {
    fn rule(&self) -> &Rule;
    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding>;
}

/// Repo-scoped rule: sees the whole corpus at once. Dispatched after the
/// per-file pass so cross-file checks (clones across files, public API
/// without a test anywhere in the tree) can correlate across boundaries.
/// A rule may implement both [`Evaluator`] and [`CorpusEvaluator`] when
/// it has work to do at both levels.
pub trait CorpusEvaluator: Send + Sync {
    fn rule(&self) -> &Rule;
    fn evaluate_corpus(&self, files: &[SourceFile], ctx: &EvalContext<'_>) -> Vec<Finding>;
}
