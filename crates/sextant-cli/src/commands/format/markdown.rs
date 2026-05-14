//! Markdown PR review — what the action posts as an issue comment.

use sextant_core::Finding;
use sextant_engine::PrReport;

use super::{verdict_line, REVIEW_MARKER};

/// Markdown review for a `--pr` run. Uses the delta — fixed on top
/// (good news first), new findings as the action items. The marker
/// comment at the bottom anchors re-runs to the same review thread.
pub fn markdown_pr(pr: &PrReport) -> String {
    let mut s = String::new();
    s.push_str("# Sextant review\n\n");
    s.push_str(&verdict_line(&pr.verdict, &pr.delta));
    s.push_str("\n\n");

    if !pr.delta.new_findings.is_empty() {
        s.push_str("## New issues introduced by this PR\n\n");
        s.push_str(&findings_table(&pr.delta.new_findings));
        s.push('\n');
    }
    if !pr.delta.fixed_findings.is_empty() {
        s.push_str("## Fixed by this PR\n\n");
        s.push_str(&fixed_table(&pr.delta.fixed_findings));
        s.push('\n');
    }
    if pr.delta.new_findings.is_empty() && pr.delta.fixed_findings.is_empty() {
        s.push_str("_No code-quality changes detected._\n\n");
    }

    s.push_str(&format!(
        "Counts: **{}** new errors, **{}** new warnings, **{}** new info; \
         {} unchanged from baseline.\n\n",
        pr.delta.new_counts.error,
        pr.delta.new_counts.warn,
        pr.delta.new_counts.info,
        pr.delta.unchanged
    ));

    if let Some(sha) = &pr.delta.base_sha {
        s.push_str(&format!("Baseline: `{}`\n\n", &sha[..sha.len().min(12)]));
    }
    s.push_str(REVIEW_MARKER);
    s.push('\n');
    s
}

fn findings_table(findings: &[Finding]) -> String {
    let mut s = String::from("| Severity | Rule | Location | Message |\n|---|---|---|---|\n");
    for f in findings {
        s.push_str(&format!(
            "| {} | `{}` | `{}` | {} |\n",
            f.severity.as_str(),
            f.rule_id,
            location(f),
            escape_pipe(&f.message),
        ));
    }
    let any_patches = findings.iter().any(|f| f.patch.is_some());
    if any_patches {
        s.push_str("\n### Proposed fixes\n\n");
        for f in findings {
            let Some(patch) = &f.patch else { continue };
            s.push_str(&format!(
                "<details><summary>{} <code>{}</code> at <code>{}</code></summary>\n\n```diff\n{}\n```\n\n</details>\n\n",
                f.severity.as_str(),
                f.rule_id,
                location(f),
                patch.trim_end(),
            ));
        }
    }
    s
}

fn fixed_table(findings: &[Finding]) -> String {
    let mut s = String::from("| Rule | Location |\n|---|---|\n");
    for f in findings {
        s.push_str(&format!("| `{}` | `{}` |\n", f.rule_id, location(f)));
    }
    s
}

pub(super) fn location(f: &Finding) -> String {
    match f.line {
        Some(line) => format!("{}:{}", f.path.display(), line),
        None => f.path.display().to_string(),
    }
}

fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::format::test_fixtures::{finding, pr_report};
    use sextant_core::{Severity, Verdict};

    #[test]
    fn markdown_pr_lists_new_and_fixed() {
        let pr = pr_report(
            vec![finding(
                "r.bad",
                Severity::Error,
                "src/a.rs",
                Some(7),
                "boom",
            )],
            vec![finding("r.old", Severity::Warn, "src/b.rs", None, "stale")],
            Verdict::RequestChanges {
                reasons: vec!["1 errors > 0".into()],
            },
            3,
        );
        let md = markdown_pr(&pr);
        assert!(md.contains("REQUEST_CHANGES"));
        assert!(md.contains("New issues"));
        assert!(md.contains("Fixed by this PR"));
        assert!(md.contains("r.bad"));
        assert!(md.contains("r.old"));
        assert!(md.contains("Baseline: `abcdef123456`"));
        assert!(md.contains(REVIEW_MARKER));
    }

    #[test]
    fn markdown_pr_renders_proposed_fix_diff_blocks() {
        let mut f = finding("r.fix", Severity::Warn, "src/a.rs", Some(2), "boom");
        f = f.with_patch("--- a/src/a.rs\n+++ b/src/a.rs\n@@ -2,1 +2,1 @@\n-x\n+y\n");
        let pr = pr_report(vec![f], vec![], Verdict::Approve, 3);
        let md = markdown_pr(&pr);
        assert!(md.contains("Proposed fixes"));
        assert!(md.contains("```diff"));
        assert!(md.contains("+y"));
    }

    #[test]
    fn markdown_pr_clean_run_says_so() {
        let pr = pr_report(vec![], vec![], Verdict::Approve, 3);
        let md = markdown_pr(&pr);
        assert!(md.contains("APPROVE"));
        assert!(md.contains("No code-quality changes detected"));
    }

    #[test]
    fn escape_pipe_keeps_messages_table_safe() {
        assert_eq!(escape_pipe("a | b"), "a \\| b");
        assert_eq!(escape_pipe("multi\nline"), "multi line");
    }
}
