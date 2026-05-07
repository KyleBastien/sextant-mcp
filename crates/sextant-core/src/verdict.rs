use serde::{Deserialize, Serialize};

use crate::{Finding, Severity};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    Approve,
    RequestChanges { reasons: Vec<String> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VerdictThresholds {
    pub max_errors: u32,
    pub max_warns: u32,
}

impl Default for VerdictThresholds {
    fn default() -> Self {
        Self {
            max_errors: 0,
            max_warns: u32::MAX,
        }
    }
}

impl VerdictThresholds {
    pub fn evaluate(&self, findings: &[Finding]) -> Verdict {
        let mut errors = 0u32;
        let mut warns = 0u32;
        for f in findings {
            match f.severity {
                Severity::Error => errors += 1,
                Severity::Warn => warns += 1,
                Severity::Info => {}
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
        if reasons.is_empty() {
            Verdict::Approve
        } else {
            Verdict::RequestChanges { reasons }
        }
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
        };
        assert_eq!(t.evaluate(&[finding(Severity::Warn)]), Verdict::Approve);
        assert_eq!(t.evaluate(&[finding(Severity::Info)]), Verdict::Approve);
    }

    #[test]
    fn request_changes_on_error() {
        let t = VerdictThresholds {
            max_errors: 0,
            max_warns: u32::MAX,
        };
        match t.evaluate(&[finding(Severity::Error)]) {
            Verdict::RequestChanges { reasons } => assert_eq!(reasons.len(), 1),
            _ => panic!("expected RequestChanges"),
        }
    }
}
