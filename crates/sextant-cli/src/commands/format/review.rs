//! GitHub PR Review API payload (`POST /repos/.../pulls/<n>/reviews`).
//!
//! Findings with a line number become inline review comments anchored
//! on the head side of the diff (`side: "RIGHT"`). Findings without a
//! line are folded into the top-level body. The output is exactly the
//! shape the API expects, so the action can `gh api ... --input -`
//! directly.
//!
//! Caveat: GitHub rejects inline comments on lines that aren't part of
//! the PR's diff. In `--pr` mode our findings are already restricted
//! to changed lines, so this should mostly hold; if it doesn't, the
//! `gh api` call will return 422 and the workflow falls back to the
//! issue-comment review.

use serde_json::json;
use sextant_core::{Finding, Verdict};
use sextant_engine::PrReport;

use super::{verdict_line, REVIEW_MARKER};

pub fn review_json_pr(pr: &PrReport) -> Result<String, serde_json::Error> {
    let mut comments = Vec::new();
    let mut bodyless: Vec<&Finding> = Vec::new();
    for f in &pr.delta.new_findings {
        match f.line {
            Some(line) => comments.push(json!({
                "path": f.path.to_string_lossy(),
                "line": line,
                "side": "RIGHT",
                "body": format!("**{}** · `{}` — {}", f.severity.as_str(), f.rule_id, f.message),
            })),
            None => bodyless.push(f),
        }
    }
    let event = match pr.verdict {
        Verdict::Approve => "COMMENT",
        Verdict::RequestChanges { .. } => "REQUEST_CHANGES",
    };
    serde_json::to_string_pretty(&json!({
        "event": event,
        "body": render_body(pr, &bodyless),
        "comments": comments,
    }))
}

fn render_body(pr: &PrReport, bodyless: &[&Finding]) -> String {
    let mut s = String::new();
    s.push_str(&verdict_line(&pr.verdict, &pr.delta));
    s.push_str("\n\n");
    s.push_str(&format!(
        "**{}** new errors · **{}** new warnings · **{}** new info · {} unchanged.\n",
        pr.delta.new_counts.error,
        pr.delta.new_counts.warn,
        pr.delta.new_counts.info,
        pr.delta.unchanged
    ));
    if !bodyless.is_empty() {
        s.push_str("\n#### File-level findings (no line anchor)\n\n");
        for f in bodyless {
            s.push_str(&format!(
                "- **{}** · `{}` · `{}` — {}\n",
                f.severity.as_str(),
                f.rule_id,
                f.path.display(),
                f.message,
            ));
        }
    }
    if !pr.delta.fixed_findings.is_empty() {
        s.push_str(&format!(
            "\n_Fixed {} pre-existing finding(s) — nice._\n",
            pr.delta.fixed_findings.len()
        ));
    }
    s.push('\n');
    s.push_str(REVIEW_MARKER);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::format::test_fixtures::{finding, pr_report};
    use serde_json::Value;
    use sextant_core::Severity;

    #[test]
    fn review_json_pr_emits_inline_comments_and_marker() {
        let pr = pr_report(
            vec![
                finding("r.bad", Severity::Error, "src/a.rs", Some(7), "boom"),
                finding("r.file", Severity::Warn, "src/b.rs", None, "no-line"),
            ],
            vec![],
            Verdict::RequestChanges {
                reasons: vec!["1 errors".into()],
            },
            0,
        );
        let v: Value = serde_json::from_str(&review_json_pr(&pr).unwrap()).unwrap();
        assert_eq!(v["event"], "REQUEST_CHANGES");
        let comments = v["comments"].as_array().unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0]["path"], "src/a.rs");
        assert_eq!(comments[0]["line"], 7);
        assert_eq!(comments[0]["side"], "RIGHT");
        let body = v["body"].as_str().unwrap();
        assert!(body.contains("REQUEST_CHANGES"));
        assert!(body.contains("r.file"));
        assert!(body.contains(REVIEW_MARKER));
    }

    #[test]
    fn review_json_pr_uses_comment_event_for_approve() {
        let pr = pr_report(vec![], vec![], Verdict::Approve, 0);
        let v: Value = serde_json::from_str(&review_json_pr(&pr).unwrap()).unwrap();
        assert_eq!(v["event"], "COMMENT");
        assert!(v["comments"].as_array().unwrap().is_empty());
    }
}
