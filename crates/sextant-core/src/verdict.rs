use serde::{Deserialize, Serialize};

use crate::{BaselineDelta, Finding, Severity};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    Approve,
    RequestChanges { reasons: Vec<String> },
}

/// How a verdict relates findings to thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerdictMode {
    /// Count every finding in the report. Used for whole-tree grades and
    /// for new (unbaselined) repos.
    #[default]
    Absolute,
    /// Count only findings that are *new* relative to a baseline. Used by
    /// `--pr`: a PR that doesn't introduce any new issues approves even
    /// when the repo has pre-existing ones.
    Regression,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VerdictThresholds {
    pub max_errors: u32,
    pub max_warns: u32,
    pub max_info: u32,
}

impl Default for VerdictThresholds {
    fn default() -> Self {
        Self {
            max_errors: 0,
            max_warns: u32::MAX,
            max_info: u32::MAX,
        }
    }
}

impl VerdictThresholds {
    pub fn evaluate(&self, findings: &[Finding]) -> Verdict {
        let mut errors = 0u32;
        let mut warns = 0u32;
        let mut infos = 0u32;
        for f in findings {
            match f.severity {
                Severity::Error => errors += 1,
                Severity::Warn => warns += 1,
                Severity::Info => infos += 1,
            }
        }
        let mut reasons = Vec::new();
        if errors > self.max_errors {
            reasons.push(format!(
                "{errors} error finding(s) exceeds limit of {}",
                self.max_errors
            ));
        }
        if warns > self.max_warns {
            reasons.push(format!(
                "{warns} warn finding(s) exceeds limit of {}",
                self.max_warns
            ));
        }
        if infos > self.max_info {
            reasons.push(format!(
                "{infos} info finding(s) exceeds limit of {}",
                self.max_info
            ));
        }
        if reasons.is_empty() {
            Verdict::Approve
        } else {
            Verdict::RequestChanges { reasons }
        }
    }

    /// Evaluate against new findings only. The threshold check is the
    /// same — `evaluate` is just called against `delta.new_findings` —
    /// but having a named entry point keeps PR-mode call sites legible.
    pub fn evaluate_regression(&self, delta: &BaselineDelta) -> Verdict {
        self.evaluate(&delta.new_findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn finding(sev: Severity) -> Finding {
        Finding::new("r", sev, PathBuf::from("a"), "msg")
    }

    #[test]
    fn approve_when_all_within_limits() {
        let t = VerdictThresholds {
            max_errors: 0,
            max_warns: 5,
            max_info: u32::MAX,
        };
        assert_eq!(t.evaluate(&[finding(Severity::Warn)]), Verdict::Approve);
        assert_eq!(t.evaluate(&[finding(Severity::Info)]), Verdict::Approve);
    }

    #[test]
    fn request_changes_on_error() {
        let t = VerdictThresholds {
            max_errors: 0,
            max_warns: u32::MAX,
            max_info: u32::MAX,
        };
        match t.evaluate(&[finding(Severity::Error)]) {
            Verdict::RequestChanges { reasons } => assert_eq!(reasons.len(), 1),
            _ => panic!("expected RequestChanges"),
        }
    }

    #[test]
    fn request_changes_when_info_exceeds_max_info() {
        let t = VerdictThresholds {
            max_errors: 0,
            max_warns: u32::MAX,
            max_info: 0,
        };
        match t.evaluate(&[finding(Severity::Info)]) {
            Verdict::RequestChanges { reasons } => {
                assert_eq!(reasons.len(), 1);
                assert!(reasons[0].contains("info"));
            }
            _ => panic!("expected RequestChanges for info over threshold"),
        }
    }

    #[test]
    fn evaluate_regression_only_counts_new_findings() {
        let t = VerdictThresholds {
            max_errors: 0,
            max_warns: u32::MAX,
            max_info: u32::MAX,
        };
        let unchanged = vec![finding(Severity::Error)];
        let new_finding = vec![finding(Severity::Error)];
        // Pretend the existing error is in baseline — delta should be empty.
        let delta = BaselineDelta::compute(&unchanged, &unchanged, None);
        assert_eq!(t.evaluate_regression(&delta), Verdict::Approve);
        // A new error in the head trips the threshold.
        let delta = BaselineDelta::compute(&new_finding, &[], None);
        assert!(matches!(
            t.evaluate_regression(&delta),
            Verdict::RequestChanges { .. }
        ));
    }
}
