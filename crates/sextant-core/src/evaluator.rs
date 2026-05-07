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
/// emit zero or more findings. Repo-scoped rules will receive a different
/// trait variant in M5; for M1 we only need file-level evaluation.
pub trait Evaluator: Send + Sync {
    fn rule(&self) -> &Rule;
    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding>;
}
