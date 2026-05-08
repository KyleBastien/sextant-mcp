//! Baseline regression: diff a head report against a baseline report so
//! the verdict can be computed against *new* findings rather than the
//! whole report.
//!
//! Findings are matched by `Finding::identity()`, which is line-agnostic
//! — code edits shift line numbers without changing what's being
//! complained about. False positives (a finding whose message text was
//! tweaked across the diff) are accepted as the cost of the simpler
//! match function.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Finding, SeverityCounts};

/// Difference between two reports of the same repo, anchored at a base
/// SHA. `new_findings` is what regression-mode verdicts evaluate against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineDelta {
    pub base_sha: Option<String>,
    pub new_findings: Vec<Finding>,
    pub fixed_findings: Vec<Finding>,
    pub unchanged: u32,
    pub new_counts: SeverityCounts,
    pub fixed_counts: SeverityCounts,
}

impl BaselineDelta {
    pub fn compute(head: &[Finding], baseline: &[Finding], base_sha: Option<String>) -> Self {
        let baseline_index: HashMap<String, &Finding> =
            baseline.iter().map(|f| (f.identity(), f)).collect();
        let head_index: HashMap<String, &Finding> =
            head.iter().map(|f| (f.identity(), f)).collect();

        let mut new_findings = Vec::new();
        let mut unchanged = 0u32;
        for f in head {
            if baseline_index.contains_key(&f.identity()) {
                unchanged += 1;
            } else {
                new_findings.push(f.clone());
            }
        }
        let mut fixed_findings = Vec::new();
        for f in baseline {
            if !head_index.contains_key(&f.identity()) {
                fixed_findings.push(f.clone());
            }
        }
        Self {
            new_counts: SeverityCounts::from_findings(&new_findings),
            fixed_counts: SeverityCounts::from_findings(&fixed_findings),
            new_findings,
            fixed_findings,
            unchanged,
            base_sha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;

    fn finding(rule: &str, path: &str, msg: &str, sev: Severity) -> Finding {
        Finding::new(rule, sev, path, msg)
    }

    #[test]
    fn empty_inputs_produce_empty_delta() {
        let d = BaselineDelta::compute(&[], &[], None);
        assert!(d.new_findings.is_empty());
        assert!(d.fixed_findings.is_empty());
        assert_eq!(d.unchanged, 0);
    }

    #[test]
    fn identical_reports_have_no_new_or_fixed() {
        let f = finding("r", "a.rs", "m", Severity::Warn);
        let d = BaselineDelta::compute(std::slice::from_ref(&f), std::slice::from_ref(&f), None);
        assert!(d.new_findings.is_empty());
        assert!(d.fixed_findings.is_empty());
        assert_eq!(d.unchanged, 1);
    }

    #[test]
    fn line_shifts_are_tolerated() {
        let baseline = finding("r", "a.rs", "m", Severity::Warn);
        let head = finding("r", "a.rs", "m", Severity::Warn).at_line(99);
        let d = BaselineDelta::compute(&[head], &[baseline], None);
        assert!(d.new_findings.is_empty());
        assert_eq!(d.unchanged, 1);
    }

    #[test]
    fn introduced_finding_lands_in_new() {
        let baseline = finding("r", "a.rs", "old", Severity::Warn);
        let head = finding("r", "a.rs", "new", Severity::Warn);
        let d = BaselineDelta::compute(&[head], &[baseline], None);
        assert_eq!(d.new_findings.len(), 1);
        assert_eq!(d.fixed_findings.len(), 1);
        assert_eq!(d.new_counts.warn, 1);
        assert_eq!(d.fixed_counts.warn, 1);
    }

    #[test]
    fn captures_the_base_sha() {
        let d = BaselineDelta::compute(&[], &[], Some("deadbeef".into()));
        assert_eq!(d.base_sha.as_deref(), Some("deadbeef"));
    }
}
