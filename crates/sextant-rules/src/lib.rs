//! Built-in rule evaluators for Sextant.
//!
//! M2 adds two AST-based rules backed by `sextant-lang`: `fn-length` and
//! `param-count`. Later milestones will add complexity, duplication, and
//! tests rules; rule discovery via `rust-embed` and frontmatter parsing
//! arrives in M3.

mod file_length;
mod fn_length;
mod param_count;

use std::sync::Arc;

use sextant_config::Config;
use sextant_core::{EvalContext, Evaluator, Finding, SourceFile};

pub use file_length::FileLengthRule;
pub use fn_length::FnLengthRule;
pub use param_count::ParamCountRule;

/// A bundle of evaluators discovered from built-ins + (later) the repo's
/// `.sextant/rules/` directory.
pub struct RuleSet {
    evaluators: Vec<Arc<dyn Evaluator>>,
}

impl RuleSet {
    pub fn builtin(config: &Config) -> Self {
        let evaluators: Vec<Arc<dyn Evaluator>> = vec![
            Arc::new(FileLengthRule::from_config(&config.size)),
            Arc::new(FnLengthRule::from_config(&config.size)),
            Arc::new(ParamCountRule::from_config(&config.size)),
        ];
        Self { evaluators }
    }

    pub fn evaluators(&self) -> &[Arc<dyn Evaluator>] {
        &self.evaluators
    }

    pub fn grade_files(&self, files: &[SourceFile], ctx: &EvalContext<'_>) -> Vec<Finding> {
        let mut out = Vec::new();
        for file in files {
            for ev in &self.evaluators {
                if !rule_applies_to_file(ev.as_ref(), file) {
                    continue;
                }
                out.extend(ev.evaluate_file(file, ctx));
            }
        }
        out
    }
}

fn rule_applies_to_file(ev: &dyn Evaluator, file: &SourceFile) -> bool {
    let rule = ev.rule();
    if !rule.enabled {
        return false;
    }
    if rule.languages.is_empty() {
        return true;
    }
    match file.language_hint() {
        Some(lang) => rule.languages.iter().any(|l| l == lang),
        None => false,
    }
}
