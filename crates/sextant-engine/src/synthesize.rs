//! Optional LLM-driven patch synthesis pass.
//!
//! Runs after the regular grading pass and only mutates findings whose
//! `patch` is still `None`. Findings get grouped by `(rule_id, path)` so
//! the judge sees a coherent context per call. The pass is fail-soft:
//! any judge error, parse error, or shape mismatch leaves the finding
//! untouched and logs a warning. Disabled by default; opt in with
//! `[autofix] llm_synthesis = true`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use sextant_config::{AutofixConfig, JudgeConfig};
use sextant_core::{Finding, Rule, SourceFile};
use sextant_judge::{Judge, JudgeRequest};

const SYSTEM_PROMPT: &str =
    "You are proposing minimal, mechanical fixes for code-review findings. \
     For each input finding, return the same finding with its `patch` field \
     populated as a unified diff against the file. If you cannot produce a \
     clear, minimal fix, omit the patch — never invent vague refactors. \
     Preserve the input order: response findings[i] corresponds to input \
     finding i.";

/// Inputs to the synthesis pass. Grouping these onto one struct keeps
/// `run`/`synthesize_for_group` under the param-count threshold and
/// makes it harder to mis-thread an argument when the surface grows.
pub(crate) struct SynthesisInputs<'a> {
    pub files: &'a [SourceFile],
    pub rules: &'a HashMap<String, Rule>,
    pub judge: Option<&'a Arc<Judge>>,
    pub autofix: &'a AutofixConfig,
    pub judge_cfg: &'a JudgeConfig,
}

/// Attach LLM-synthesized patches to findings whose `patch` is `None`.
/// `inputs.files` is the same slice the grading pass walked, used to
/// recover each finding's file contents without re-reading from disk.
pub(crate) fn run(findings: &mut [Finding], inputs: SynthesisInputs<'_>) {
    if !inputs.autofix.enabled || !inputs.autofix.llm_synthesis {
        return;
    }
    let Some(judge) = inputs.judge else {
        return;
    };

    let candidate_indices =
        patch_candidate_indices(findings, inputs.autofix.max_synthesis_findings);
    if candidate_indices.is_empty() {
        return;
    }

    let groups = group_by_rule_and_path(findings, &candidate_indices);
    let file_lookup: HashMap<&PathBuf, &SourceFile> =
        inputs.files.iter().map(|f| (&f.path, f)).collect();

    for ((rule_id, path), idxs) in groups {
        let Some(rule) = inputs.rules.get(&rule_id) else {
            continue;
        };
        let Some(file) = lookup_source_for_path(&file_lookup, inputs.files, &path) else {
            tracing::debug!(rule = %rule_id, ?path, "synthesis: no source file in scope");
            continue;
        };
        let group = SynthesisGroup {
            rule,
            file,
            judge,
            judge_cfg: inputs.judge_cfg,
            indices: &idxs,
        };
        synthesize_for_group(findings, group);
    }
}

/// Indices of findings that don't yet have a patch, capped to the
/// configured budget so a noisy grade can't run away on the API bill.
fn patch_candidate_indices(findings: &[Finding], budget: u32) -> Vec<usize> {
    findings
        .iter()
        .enumerate()
        .filter(|(_, f)| f.patch.is_none())
        .map(|(i, _)| i)
        .take(budget as usize)
        .collect()
}

fn group_by_rule_and_path(
    findings: &[Finding],
    candidate_indices: &[usize],
) -> HashMap<(String, PathBuf), Vec<usize>> {
    let candidates: HashSet<usize> = candidate_indices.iter().copied().collect();
    let mut out: HashMap<(String, PathBuf), Vec<usize>> = HashMap::new();
    for (i, f) in findings.iter().enumerate() {
        if !candidates.contains(&i) {
            continue;
        }
        out.entry((f.rule_id.clone(), f.path.clone()))
            .or_default()
            .push(i);
    }
    out
}

/// Findings carry `path` relative to the repo root, while `SourceFile`
/// holds an absolute path. Try the direct lookup first, then fall back to
/// matching by suffix so the engine works regardless of where the
/// finding's path was anchored.
fn lookup_source_for_path<'a>(
    direct: &HashMap<&PathBuf, &'a SourceFile>,
    files: &'a [SourceFile],
    path: &PathBuf,
) -> Option<&'a SourceFile> {
    if let Some(found) = direct.get(path) {
        return Some(*found);
    }
    files.iter().find(|f| f.path.ends_with(path))
}

/// One judge call's worth of inputs: the rule, the file under review,
/// the judge handle and config, and the indices of the findings we want
/// patches for (positions in the parent `findings` slice).
struct SynthesisGroup<'a> {
    rule: &'a Rule,
    file: &'a SourceFile,
    judge: &'a Judge,
    judge_cfg: &'a JudgeConfig,
    indices: &'a [usize],
}

fn synthesize_for_group(findings: &mut [Finding], group: SynthesisGroup<'_>) {
    let inputs: Vec<&Finding> = group.indices.iter().map(|&i| &findings[i]).collect();
    let prompt = build_prompt(group.rule, group.file, &inputs);
    let req = JudgeRequest {
        system_prompt: Some(SYSTEM_PROMPT),
        user_prompt: &prompt,
        model: &group.judge_cfg.model,
        max_tokens: group.judge_cfg.max_tokens,
        temperature: group.judge_cfg.temperature,
    };
    let result = match group.judge.judge_blocking_synthesis(req) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(rule = %group.rule.id, ?err, "synthesis call failed");
            return;
        }
    };

    let fallback = result.patch.clone();
    for (slot, jf) in group.indices.iter().zip(result.findings.iter()) {
        if let Some(patch) = jf.patch.clone().or_else(|| fallback.clone()) {
            findings[*slot].patch = Some(patch);
        }
    }
    // If the response had fewer findings than the input, anything left
    // over still gets the whole-result patch as a last-resort fallback.
    for slot in group.indices.iter().skip(result.findings.len()) {
        if findings[*slot].patch.is_none() {
            if let Some(p) = fallback.clone() {
                findings[*slot].patch = Some(p);
            }
        }
    }
}

fn build_prompt(rule: &Rule, file: &SourceFile, findings: &[&Finding]) -> String {
    let mut s = String::new();
    s.push_str("Rule:\n");
    s.push_str(&format!("  id: {}\n", rule.id));
    s.push_str(&format!("  name: {}\n", rule.name));
    if !rule.body.is_empty() {
        s.push_str("\nRule body:\n");
        s.push_str(rule.body.trim());
        s.push('\n');
    }
    s.push_str(&format!("\nFile: {}\n", file.path.display()));
    s.push_str("```\n");
    s.push_str(&file.contents);
    if !file.contents.ends_with('\n') {
        s.push('\n');
    }
    s.push_str("```\n\nFindings (respond in the same order):\n");
    for (i, f) in findings.iter().enumerate() {
        let span = match (f.line, f.end_line) {
            (Some(s), Some(e)) if s != e => format!("lines {s}-{e}"),
            (Some(s), _) => format!("line {s}"),
            (None, _) => "no line".to_string(),
        };
        s.push_str(&format!("  [{i}] ({span}) {msg}\n", msg = f.message));
    }
    s.push_str(
        "\nReturn one finding per input. Each `patch` MUST be a unified diff \
         starting with `--- a/<path>` and `+++ b/<path>` using the file path above.\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use sextant_core::{Category, RuleSource, Scope, Severity};
    use sextant_judge::{FakeJudge, JudgeFinding, JudgeResult, JudgeSeverity};

    fn rule() -> Rule {
        Rule {
            id: "r.x".into(),
            name: "x".into(),
            description: "x".into(),
            body: "Fix it.".into(),
            severity: Severity::Warn,
            category: Category::Style,
            scope: Scope::File,
            languages: vec![],
            enabled: true,
            tags: vec![],
            source: RuleSource::Builtin,
        }
    }

    fn finding(path: &str, line: u32, msg: &str) -> Finding {
        Finding::new("r.x", Severity::Warn, path, msg).at_line(line)
    }

    fn judge_returning(result: JudgeResult) -> Arc<Judge> {
        let dir = tempfile::tempdir().unwrap().keep();
        let provider = Arc::new(FakeJudge::always("fake", result));
        Arc::new(Judge::new(provider, dir).unwrap())
    }

    fn rules_map() -> HashMap<String, Rule> {
        let mut m = HashMap::new();
        m.insert("r.x".into(), rule());
        m
    }

    fn judge_cfg() -> JudgeConfig {
        JudgeConfig {
            model: "m".into(),
            max_tokens: 64,
            ..JudgeConfig::default()
        }
    }

    fn enabled_autofix() -> AutofixConfig {
        AutofixConfig {
            enabled: true,
            llm_synthesis: true,
            max_synthesis_findings: 25,
        }
    }

    /// Local helper: keep the test bodies focused on the assertions
    /// rather than the call shape. `judge_cfg`/`rules` are constant
    /// across the suite so the helper bakes them in.
    fn run_with(
        findings: &mut [Finding],
        files: &[SourceFile],
        judge: Option<&Arc<Judge>>,
        autofix: &AutofixConfig,
    ) {
        let rules = rules_map();
        let cfg = judge_cfg();
        run(
            findings,
            SynthesisInputs {
                files,
                rules: &rules,
                judge,
                autofix,
                judge_cfg: &cfg,
            },
        );
    }

    #[test]
    fn no_op_when_synthesis_disabled() {
        let mut findings = vec![finding("a.rs", 1, "msg")];
        let files = vec![SourceFile::new("a.rs", "x\n")];
        let cfg = AutofixConfig::default();
        let judge = judge_returning(JudgeResult {
            findings: vec![],
            patch: None,
        });
        run_with(&mut findings, &files, Some(&judge), &cfg);
        assert!(findings[0].patch.is_none());
    }

    #[test]
    fn no_op_when_judge_missing() {
        let mut findings = vec![finding("a.rs", 1, "msg")];
        let files = vec![SourceFile::new("a.rs", "x\n")];
        run_with(&mut findings, &files, None, &enabled_autofix());
        assert!(findings[0].patch.is_none());
    }

    #[test]
    fn attaches_patch_from_judge_response() {
        let mut findings = vec![finding("a.rs", 1, "msg")];
        let files = vec![SourceFile::new("a.rs", "x\n")];
        let judge = judge_returning(JudgeResult {
            findings: vec![JudgeFinding {
                severity: JudgeSeverity::Warn,
                message: "msg".into(),
                line: Some(1),
                end_line: None,
                patch: Some("--- a/a.rs\n+++ b/a.rs\n".into()),
            }],
            patch: None,
        });
        run_with(&mut findings, &files, Some(&judge), &enabled_autofix());
        assert!(findings[0].patch.is_some());
    }

    #[test]
    fn skips_findings_that_already_have_a_patch() {
        let mut findings = vec![finding("a.rs", 1, "msg").with_patch("ORIGINAL")];
        let files = vec![SourceFile::new("a.rs", "x\n")];
        let judge = judge_returning(JudgeResult {
            findings: vec![JudgeFinding {
                severity: JudgeSeverity::Warn,
                message: "msg".into(),
                line: None,
                end_line: None,
                patch: Some("FROM-LLM".into()),
            }],
            patch: None,
        });
        run_with(&mut findings, &files, Some(&judge), &enabled_autofix());
        assert_eq!(findings[0].patch.as_deref(), Some("ORIGINAL"));
    }

    #[test]
    fn whole_result_patch_is_used_as_fallback() {
        let mut findings = vec![finding("a.rs", 1, "first"), finding("a.rs", 2, "second")];
        let files = vec![SourceFile::new("a.rs", "x\ny\n")];
        let judge = judge_returning(JudgeResult {
            findings: vec![],
            patch: Some("WHOLE".into()),
        });
        run_with(&mut findings, &files, Some(&judge), &enabled_autofix());
        assert_eq!(findings[0].patch.as_deref(), Some("WHOLE"));
        assert_eq!(findings[1].patch.as_deref(), Some("WHOLE"));
    }

    #[test]
    fn budget_caps_number_of_synthesis_calls() {
        let mut findings = vec![
            finding("a.rs", 1, "one"),
            finding("a.rs", 2, "two"),
            finding("a.rs", 3, "three"),
        ];
        let files = vec![SourceFile::new("a.rs", "x\ny\nz\n")];
        let judge = judge_returning(JudgeResult {
            findings: vec![],
            patch: Some("CAPPED".into()),
        });
        let cfg = AutofixConfig {
            enabled: true,
            llm_synthesis: true,
            max_synthesis_findings: 2,
        };
        run_with(&mut findings, &files, Some(&judge), &cfg);
        assert_eq!(findings[0].patch.as_deref(), Some("CAPPED"));
        assert_eq!(findings[1].patch.as_deref(), Some("CAPPED"));
        assert!(findings[2].patch.is_none());
    }
}
