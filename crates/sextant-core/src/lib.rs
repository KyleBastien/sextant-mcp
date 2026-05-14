//! Core types for the Sextant grading engine.
//!
//! This crate intentionally has no I/O dependencies. It defines the data
//! model (`Rule`, `Finding`, `Report`, `Verdict`) and the `Evaluator` trait
//! that rule implementations satisfy. Wire formats (JSON for MCP, markdown
//! for PR comments) are derived from these types in higher-level crates.

mod baseline;
mod evaluator;
mod finding;
mod report;
mod rule;
mod source;
mod verdict;

pub use baseline::BaselineDelta;
pub use evaluator::{CorpusEvaluator, EvalContext, Evaluator};
pub use finding::{Finding, Severity};
pub use report::{Report, SeverityCounts};
pub use rule::{Category, Rule, RuleSource, Scope};
pub use source::SourceFile;
pub use verdict::{Verdict, VerdictMode, VerdictThresholds};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("rule {0} failed: {1}")]
    RuleFailed(String, String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;
