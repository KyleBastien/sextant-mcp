//! SARIF v2.1.0 emitter — what GitHub Code Scanning ingests.

use serde_json::{json, Value};
use sextant_core::{Finding, Report, Severity};

/// Minimal SARIF document covering the GitHub Code Scanning
/// requirements: a tool description with the rule registry and a list
/// of results with `ruleId`, `level`, `message`, and a `locations`
/// array.
pub fn sarif(report: &Report) -> Result<String, serde_json::Error> {
    let rules: Vec<Value> = collect_rule_ids(&report.findings)
        .into_iter()
        .map(|id| json!({ "id": id, "name": id }))
        .collect();
    let results: Vec<Value> = report
        .findings
        .iter()
        .map(|f| {
            let mut result = json!({
                "ruleId": f.rule_id,
                "level": sarif_level(f.severity),
                "message": { "text": f.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.path.to_string_lossy() },
                        "region": region(f),
                    }
                }]
            });
            if let Some(patch) = &f.patch {
                result.as_object_mut().unwrap().insert(
                    "fixes".into(),
                    json!([{
                        "description": { "text": "Sextant proposed patch" },
                        "artifactChanges": [{
                            "artifactLocation": { "uri": f.path.to_string_lossy() },
                            // SARIF wants per-replacement edits; we don't
                            // carry those, so embed the unified diff as a
                            // single replacement description. Tools that
                            // ingest SARIF and apply fixes will fall back
                            // to displaying the diff.
                            "replacements": [{
                                "deletedRegion": region(f),
                                "insertedContent": { "text": patch.clone() }
                            }]
                        }]
                    }]),
                );
            }
            result
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
    use sextant_core::Verdict;

    #[test]
    fn sarif_emits_v210_with_rule_ids_and_results() {
        let f1 = Finding::new("r.a", Severity::Error, "src/a.rs", "boom").at_line(2);
        let f2 = Finding::new("r.b", Severity::Warn, "src/b.rs", "huh");
        let r = Report::build(vec![f1, f2], Verdict::Approve);
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
    fn sarif_emits_fixes_when_finding_has_patch() {
        let f = Finding::new("r.a", Severity::Warn, "src/a.rs", "boom")
            .at_line(2)
            .with_patch("--- a/src/a.rs\n+++ b/src/a.rs\n@@ -2,1 +2,1 @@\n-x\n+y\n");
        let r = Report::build(vec![f], Verdict::Approve);
        let s = sarif(&r).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        let fixes = &v["runs"][0]["results"][0]["fixes"];
        assert!(fixes.is_array(), "missing fixes array");
        assert_eq!(
            fixes[0]["artifactChanges"][0]["artifactLocation"]["uri"],
            "src/a.rs"
        );
        let inserted =
            &fixes[0]["artifactChanges"][0]["replacements"][0]["insertedContent"]["text"];
        assert!(inserted.as_str().unwrap().contains("+y"));
    }

    #[test]
    fn sarif_level_maps_severities() {
        assert_eq!(sarif_level(Severity::Info), "note");
        assert_eq!(sarif_level(Severity::Warn), "warning");
        assert_eq!(sarif_level(Severity::Error), "error");
    }
}
