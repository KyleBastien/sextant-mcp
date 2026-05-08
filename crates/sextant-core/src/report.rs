use serde::{Deserialize, Serialize};

use crate::{Finding, Severity, Verdict};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub info: u32,
    pub warn: u32,
    pub error: u32,
}

impl SeverityCounts {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut c = Self::default();
        for f in findings {
            match f.severity {
                Severity::Info => c.info += 1,
                Severity::Warn => c.warn += 1,
                Severity::Error => c.error += 1,
            }
        }
        c
    }

    pub fn total(&self) -> u32 {
        self.info + self.warn + self.error
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub counts: SeverityCounts,
    pub verdict: Verdict,
    pub summary: String,
}

impl Report {
    /// Build a report and compute its summary line. Findings are sorted by
    /// (severity desc, path, line) so the JSON output is deterministic —
    /// snapshot tests rely on this.
    pub fn build(mut findings: Vec<Finding>, verdict: Verdict) -> Self {
        findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.line.cmp(&b.line))
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });
        let counts = SeverityCounts::from_findings(&findings);
        let summary = summarize(&findings, &counts, &verdict);
        Self {
            findings,
            counts,
            verdict,
            summary,
        }
    }
}

fn summarize(findings: &[Finding], counts: &SeverityCounts, verdict: &Verdict) -> String {
    let verdict_str = match verdict {
        Verdict::Approve => "APPROVE",
        Verdict::RequestChanges { .. } => "REQUEST_CHANGES",
    };
    let mut s = format!(
        "{} errors, {} warnings, {} info; verdict: {}",
        counts.error, counts.warn, counts.info, verdict_str
    );
    let top: Vec<String> = findings
        .iter()
        .take(3)
        .map(|f| format!("{} ({})", f.rule_id, f.severity.as_str()))
        .collect();
    if !top.is_empty() {
        s.push_str("; top: ");
        s.push_str(&top.join(", "));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(sev: Severity) -> Finding {
        Finding::new("r", sev, "a.rs", "m")
    }

    #[test]
    fn from_findings_tallies_each_severity() {
        let fs = vec![
            finding(Severity::Info),
            finding(Severity::Warn),
            finding(Severity::Warn),
            finding(Severity::Error),
        ];
        let c = SeverityCounts::from_findings(&fs);
        assert_eq!(c.info, 1);
        assert_eq!(c.warn, 2);
        assert_eq!(c.error, 1);
        assert_eq!(c.total(), 4);
    }

    #[test]
    fn build_sorts_findings_severity_desc() {
        let fs = vec![
            finding(Severity::Info),
            finding(Severity::Error),
            finding(Severity::Warn),
        ];
        let r = Report::build(fs, Verdict::Approve);
        assert_eq!(r.findings[0].severity, Severity::Error);
        assert_eq!(r.findings[1].severity, Severity::Warn);
        assert_eq!(r.findings[2].severity, Severity::Info);
    }

    #[test]
    fn build_summary_mentions_verdict_and_counts() {
        let fs = vec![finding(Severity::Error), finding(Severity::Warn)];
        let r = Report::build(fs, Verdict::Approve);
        assert!(r.summary.contains("1 errors"));
        assert!(r.summary.contains("1 warnings"));
        assert!(r.summary.contains("APPROVE"));
    }

    #[test]
    fn build_summary_marks_request_changes() {
        let r = Report::build(
            vec![finding(Severity::Error)],
            Verdict::RequestChanges {
                reasons: vec!["x".into()],
            },
        );
        assert!(r.summary.contains("REQUEST_CHANGES"));
    }
}
