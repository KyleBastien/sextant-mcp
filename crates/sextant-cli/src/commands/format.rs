//! Output formatters: human, JSON, markdown, SARIF.
//!
//! Markdown is the format the GitHub Action posts as a PR review comment.
//! SARIF is the format GitHub Code Scanning ingests; emitting it lets a
//! CI run double-publish into the security tab.

use serde_json::{json, Value};
use sextant_core::{BaselineDelta, Finding, Report, Severity, Verdict};
use sextant_engine::PrReport;

/// Marker comment that lets a re-run update the same PR review instead
/// of duplicating it. The action's `post-review.sh` script grep for it.
const REVIEW_MARKER: &str = "<!-- sextant:review -->";

pub fn human(report: &Report) -> String {
    let mut out = String::new();
    if report.findings.is_empty() {
        out.push_str("No findings.\n");
    } else {
        for f in &report.findings {
            let line = f.line.map(|l| format!(":{l}")).unwrap_or_default();
            out.push_str(&format!(
                "{:<5} {}{}\t{}\t{}\n",
                f.severity.as_str(),
                f.path.display(),
                line,
                f.rule_id,
                f.message
            ));
        }
    }
    out.push('\n');
    out.push_str(&report.summary);
    out.push('\n');
    out
}

pub fn json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

pub fn json_pr(pr: &PrReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(pr)
}

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

fn verdict_line(verdict: &Verdict, delta: &BaselineDelta) -> String {
    match verdict {
        Verdict::Approve => format!(
            "**Verdict:** :white_check_mark: APPROVE — no new issues ({} unchanged).",
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
    s
}

fn fixed_table(findings: &[Finding]) -> String {
    let mut s = String::from("| Rule | Location |\n|---|---|\n");
    for f in findings {
        s.push_str(&format!("| `{}` | `{}` |\n", f.rule_id, location(f)));
    }
    s
}

fn location(f: &Finding) -> String {
    match f.line {
        Some(line) => format!("{}:{}", f.path.display(), line),
        None => f.path.display().to_string(),
    }
}

fn escape_pipe(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// SARIF v2.1.0 emitter. Minimal but covers the GitHub Code Scanning
/// requirements: a tool description with the rule registry and a list of
/// results with `ruleId`, `level`, `message`, and a `locations` array.
pub fn sarif(report: &Report) -> Result<String, serde_json::Error> {
    let rules: Vec<Value> = collect_rule_ids(&report.findings)
        .into_iter()
        .map(|id| json!({ "id": id, "name": id }))
        .collect();
    let results: Vec<Value> = report
        .findings
        .iter()
        .map(|f| {
            json!({
                "ruleId": f.rule_id,
                "level": sarif_level(f.severity),
                "message": { "text": f.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.path.to_string_lossy() },
                        "region": region(f),
                    }
                }]
            })
        })
        .collect();
    let doc = json!({
        "version": "2.1.0",
        "$schema": "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "sextant",
                    "informationUri": "https://github.com/kylebastien/sextant-mcp",
                    "rules": rules
                }
            },
            "results": results
        }]
    });
    serde_json::to_string_pretty(&doc)
}

fn region(f: &Finding) -> Value {
    let mut obj = serde_json::Map::new();
    if let Some(line) = f.line {
        obj.insert("startLine".into(), json!(line));
    }
    if let Some(end) = f.end_line {
        obj.insert("endLine".into(), json!(end));
    }
    Value::Object(obj)
}

fn sarif_level(s: Severity) -> &'static str {
    match s {
        Severity::Info => "note",
        Severity::Warn => "warning",
        Severity::Error => "error",
    }
}

fn collect_rule_ids(findings: &[Finding]) -> Vec<String> {
    let mut ids: Vec<String> = findings.iter().map(|f| f.rule_id.clone()).collect();
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use sextant_core::SeverityCounts;

    fn finding(rule: &str, sev: Severity, path: &str, line: Option<u32>, msg: &str) -> Finding {
        let mut f = Finding::new(rule, sev, path, msg);
        if let Some(l) = line {
            f = f.at_line(l);
        }
        f
    }

    fn pr_report(new: Vec<Finding>, fixed: Vec<Finding>, verdict: Verdict) -> PrReport {
        let head = Report::build(new.clone(), Verdict::Approve);
        let baseline = Report::build(fixed.clone(), Verdict::Approve);
        PrReport {
            head,
            baseline,
            delta: BaselineDelta {
                base_sha: Some("abcdef1234567890".into()),
                new_findings: new.clone(),
                fixed_findings: fixed.clone(),
                unchanged: 3,
                new_counts: SeverityCounts::from_findings(&new),
                fixed_counts: SeverityCounts::from_findings(&fixed),
            },
            verdict,
        }
    }

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
    fn markdown_pr_clean_run_says_so() {
        let pr = pr_report(vec![], vec![], Verdict::Approve);
        let md = markdown_pr(&pr);
        assert!(md.contains("APPROVE"));
        assert!(md.contains("No code-quality changes detected"));
    }

    #[test]
    fn sarif_emits_v210_with_rule_ids_and_results() {
        let r = Report::build(
            vec![
                finding("r.a", Severity::Error, "src/a.rs", Some(2), "boom"),
                finding("r.b", Severity::Warn, "src/b.rs", None, "huh"),
            ],
            Verdict::Approve,
        );
        let s = sarif(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["version"], "2.1.0");
        let results = v["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["ruleId"], "r.a");
        assert_eq!(results[0]["level"], "error");
        let rules = v["runs"][0]["tool"]["driver"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn escape_pipe_keeps_messages_table_safe() {
        assert_eq!(escape_pipe("a | b"), "a \\| b");
        assert_eq!(escape_pipe("multi\nline"), "multi line");
    }

    #[test]
    fn human_render_lists_findings_with_summary() {
        let r = Report::build(
            vec![finding("r.x", Severity::Warn, "src/a.rs", Some(2), "boom")],
            Verdict::Approve,
        );
        let s = human(&r);
        assert!(s.contains("warn"));
        assert!(s.contains("src/a.rs:2"));
        assert!(s.contains("r.x"));
        assert!(s.contains("APPROVE"));
    }

    #[test]
    fn json_round_trips_a_report() {
        let r = Report::build(vec![], Verdict::Approve);
        let s = json(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("findings").is_some());
        assert!(v.get("verdict").is_some());
    }

    #[test]
    fn json_pr_round_trips_a_pr_report() {
        let pr = pr_report(vec![], vec![], Verdict::Approve);
        let s = json_pr(&pr).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("delta").is_some());
        assert!(v.get("head").is_some());
        assert!(v.get("baseline").is_some());
    }
}
