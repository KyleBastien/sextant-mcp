//! Shared test scaffolding for the markdown and review-JSON formatters.
//! Both wire formats consume `PrReport`, so the same fixture builder
//! works for both — keeping it in one place stops the two test modules
//! from duplicating the boilerplate.

use sextant_core::{BaselineDelta, Finding, Report, Severity, SeverityCounts, Verdict};
use sextant_engine::PrReport;

pub(super) fn finding(
    rule: &str,
    sev: Severity,
    path: &str,
    line: Option<u32>,
    msg: &str,
) -> Finding {
    let mut f = Finding::new(rule, sev, path, msg);
    if let Some(l) = line {
        f = f.at_line(l);
    }
    f
}

pub(super) fn pr_report(
    new: Vec<Finding>,
    fixed: Vec<Finding>,
    verdict: Verdict,
    unchanged: u32,
) -> PrReport {
    PrReport {
        head: Report::build(new.clone(), Verdict::Approve),
        baseline: Report::build(fixed.clone(), Verdict::Approve),
        delta: BaselineDelta {
            base_sha: Some("abcdef1234567890".into()),
            new_findings: new.clone(),
            fixed_findings: fixed.clone(),
            unchanged,
            new_counts: SeverityCounts::from_findings(&new),
            fixed_counts: SeverityCounts::from_findings(&fixed),
        },
        verdict,
    }
}
