//! Output formatters: human, JSON, markdown, SARIF, GitHub review JSON.
//!
//! Each submodule owns one wire format. Shared helpers (the review
//! marker comment, common verdict-line rendering) live here.

mod markdown;
mod review;
mod sarif;
#[cfg(test)]
mod test_fixtures;

pub use markdown::markdown_pr;
pub use review::review_json_pr;
pub use sarif::sarif;

use sextant_core::{BaselineDelta, Finding, Report, SeverityCounts, Verdict};
use sextant_engine::PrReport;

/// Marker comment embedded in every Sextant-authored PR review. The
/// action's `post-review.sh` greps for it to find a prior comment to
/// update instead of stacking duplicates.
pub(crate) const REVIEW_MARKER: &str = "<!-- sextant:review -->";

/// Human formatter with optional patch rendering. When `show_patches` is
/// true, every finding that carries a `patch` is followed by an indented
/// diff block; when false, the patched lines get a `(patch available, --show-patches)`
/// hint instead so the report stays scannable by default.
pub fn human_with(report: &Report, show_patches: bool) -> String {
    let mut out = String::new();
    if report.findings.is_empty() {
        out.push_str("No findings.\n");
    } else {
        for f in &report.findings {
            render_finding(&mut out, f, show_patches);
        }
    }
    out.push('\n');
    out.push_str(&report.summary);
    out.push('\n');
    out
}

fn render_finding(out: &mut String, f: &Finding, show_patches: bool) {
    let line = f.line.map(|l| format!(":{l}")).unwrap_or_default();
    out.push_str(&format!(
        "{:<5} {}{}\t{}\t{}\n",
        f.severity.as_str(),
        f.path.display(),
        line,
        f.rule_id,
        f.message
    ));
    render_patch(out, f.patch.as_deref(), show_patches);
}

fn render_patch(out: &mut String, patch: Option<&str>, show_patches: bool) {
    match (patch, show_patches) {
        (Some(diff), true) => indent_diff(out, diff),
        (Some(_), false) => {
            out.push_str("      (patch available, pass --show-patches to render)\n");
        }
        (None, _) => {}
    }
}

fn indent_diff(out: &mut String, diff: &str) {
    for diff_line in diff.lines() {
        out.push_str("    ");
        out.push_str(diff_line);
        out.push('\n');
    }
}

pub fn json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

pub fn json_pr(pr: &PrReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(pr)
}

/// One-line "**Verdict:** APPROVE / REQUEST_CHANGES — reasons" header
/// shared by both the markdown and review-JSON formatters.
pub(crate) fn verdict_line(verdict: &Verdict, delta: &BaselineDelta) -> String {
    match verdict {
        Verdict::Approve if delta.new_findings.is_empty() => format!(
            "**Verdict:** :white_check_mark: APPROVE — no new issues ({} unchanged).",
            delta.unchanged
        ),
        Verdict::Approve => format!(
            "**Verdict:** :white_check_mark: APPROVE — {} under thresholds ({} unchanged).",
            severity_breakdown(&delta.new_counts),
            delta.unchanged
        ),
        Verdict::RequestChanges { reasons } => {
            let mut s = "**Verdict:** :no_entry_sign: REQUEST_CHANGES".to_string();
            if !reasons.is_empty() {
                s.push_str(" — ");
                s.push_str(&reasons.join("; "));
            }
            s
        }
    }
}

fn severity_breakdown(c: &SeverityCounts) -> String {
    let mut parts = Vec::new();
    if c.error > 0 {
        parts.push(format!("{} new error{}", c.error, plural(c.error)));
    }
    if c.warn > 0 {
        parts.push(format!("{} new warning{}", c.warn, plural(c.warn)));
    }
    if c.info > 0 {
        parts.push(format!("{} new info", c.info));
    }
    parts.join(", ")
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sextant_core::{Finding, Severity};

    #[test]
    fn human_render_lists_findings_with_summary() {
        let f = Finding::new("r.x", Severity::Warn, "src/a.rs", "boom").at_line(2);
        let r = Report::build(vec![f], Verdict::Approve);
        let s = human_with(&r, false);
        assert!(s.contains("warn"));
        assert!(s.contains("src/a.rs:2"));
        assert!(s.contains("r.x"));
        assert!(s.contains("APPROVE"));
    }

    #[test]
    fn human_with_patches_renders_indented_diff_block() {
        let f = Finding::new("r.x", Severity::Warn, "src/a.rs", "boom")
            .at_line(2)
            .with_patch("--- a/src/a.rs\n+++ b/src/a.rs\n@@ -2,1 +2,1 @@\n-old\n+new\n");
        let r = Report::build(vec![f], Verdict::Approve);
        let on = human_with(&r, true);
        assert!(on.contains("    --- a/src/a.rs"));
        assert!(on.contains("    +new"));
        let off = human_with(&r, false);
        assert!(off.contains("(patch available"));
        assert!(!off.contains("    +new"));
    }

    #[test]
    fn json_round_trips_a_report() {
        let r = Report::build(vec![], Verdict::Approve);
        let v: serde_json::Value = serde_json::from_str(&json(&r).unwrap()).unwrap();
        assert!(v.get("findings").is_some());
        assert!(v.get("verdict").is_some());
    }

    #[test]
    fn json_pr_carries_head_baseline_and_delta() {
        let head = Report::build(vec![], Verdict::Approve);
        let baseline = Report::build(vec![], Verdict::Approve);
        let delta = BaselineDelta::compute(&[], &[], None);
        let pr = PrReport {
            head,
            baseline,
            delta,
            verdict: Verdict::Approve,
        };
        let v: serde_json::Value = serde_json::from_str(&json_pr(&pr).unwrap()).unwrap();
        assert!(v.get("head").is_some());
        assert!(v.get("baseline").is_some());
        assert!(v.get("delta").is_some());
        assert!(v.get("verdict").is_some());
    }

    #[test]
    fn verdict_line_marks_approve_and_request_changes() {
        let d = BaselineDelta::compute(&[], &[], None);
        assert!(verdict_line(&Verdict::Approve, &d).contains("APPROVE"));
        assert!(verdict_line(
            &Verdict::RequestChanges {
                reasons: vec!["x".into()],
            },
            &d
        )
        .contains("REQUEST_CHANGES"));
    }

    #[test]
    fn approve_with_subthreshold_findings_does_not_claim_no_new_issues() {
        let infos: Vec<Finding> = (0..4)
            .map(|i| Finding::new("r", Severity::Info, format!("a{i}.rs"), "m"))
            .collect();
        let delta = BaselineDelta::compute(&infos, &[], None);
        let line = verdict_line(&Verdict::Approve, &delta);
        assert!(line.contains("APPROVE"));
        assert!(
            !line.contains("no new issues"),
            "headline lied about new issues: {line}"
        );
        assert!(line.contains("4 new info"));
        assert!(line.contains("under thresholds"));
    }

    #[test]
    fn severity_breakdown_pluralizes_and_orders_high_to_low() {
        let c = SeverityCounts {
            info: 4,
            warn: 2,
            error: 1,
        };
        assert_eq!(
            severity_breakdown(&c),
            "1 new error, 2 new warnings, 4 new info"
        );
    }
}
