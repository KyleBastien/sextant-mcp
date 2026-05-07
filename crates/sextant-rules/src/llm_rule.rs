//! LLM-evaluated rule.
//!
//! The rule's markdown body is the prompt template. We render it with the
//! file path and contents, ship it to the configured `Judge`, and turn
//! the structured response into `Finding`s. Errors degrade to a single
//! `info` finding so a flaky network never blocks a grade.

use std::sync::Arc;

use globset::{Glob, GlobSet, GlobSetBuilder};
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};
use sextant_judge::{Judge, JudgeRequest, JudgeSeverity};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;
use crate::regex_rule::RegexBuildError;

pub struct LlmRule {
    rule: Rule,
    judge: Arc<Judge>,
    model: String,
    max_tokens: u32,
    temperature: f32,
    exclude: GlobSet,
}

pub struct LlmRuleSpec {
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub exclude_paths: Vec<String>,
}

impl LlmRule {
    pub fn from_parsed(
        parsed: ParsedRule,
        judge: Arc<Judge>,
        spec: LlmRuleSpec,
    ) -> Result<Self, RegexBuildError> {
        let exclude = build_globset(&spec.exclude_paths)?;
        Ok(Self {
            rule: rule_from_parsed(parsed),
            judge,
            model: spec.model,
            max_tokens: spec.max_tokens,
            temperature: spec.temperature,
            exclude,
        })
    }

    fn render(&self, file: &SourceFile, rel_path: &std::path::Path) -> String {
        // Minimal {{var}} substitution. Order matters only to the extent
        // that `{{rule.id}}` and `{{path}}` could collide if a rule body
        // mentions one literally, which we accept.
        self.rule
            .body
            .replace("{{rule.id}}", &self.rule.id)
            .replace("{{path}}", &rel_path.display().to_string())
            .replace("{{code}}", &file.contents)
    }
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, RegexBuildError> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p).map_err(|source| RegexBuildError::Glob {
            pattern: p.clone(),
            source,
        })?;
        b.add(glob);
    }
    b.build().map_err(|source| RegexBuildError::Glob {
        pattern: "<set>".into(),
        source,
    })
}

impl Evaluator for LlmRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        let rel = file.relative_to(ctx.repo_root);
        if self.exclude.is_match(&rel) {
            return Vec::new();
        }
        let prompt = self.render(file, &rel);
        let req = JudgeRequest {
            system_prompt: None,
            user_prompt: &prompt,
            model: &self.model,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
        };
        match self.judge.judge_blocking(req) {
            Ok(result) => result
                .findings
                .into_iter()
                .map(|jf| {
                    let mut f = Finding::new(
                        &self.rule.id,
                        translate_severity(jf.severity),
                        rel.clone(),
                        jf.message,
                    );
                    if let (Some(start), Some(end)) = (jf.line, jf.end_line) {
                        f = f.spanning(start, end);
                    } else if let Some(line) = jf.line {
                        f = f.at_line(line);
                    }
                    f
                })
                .collect(),
            Err(err) => {
                tracing::warn!(rule=%self.rule.id, ?err, "judge call failed; degrading to info");
                vec![Finding::new(
                    &self.rule.id,
                    Severity::Info,
                    rel,
                    format!(
                        "Judge `{}` failed; rule degraded to info: {err}",
                        self.judge.provider_name()
                    ),
                )]
            }
        }
    }
}

fn translate_severity(s: JudgeSeverity) -> Severity {
    match s {
        JudgeSeverity::Info => Severity::Info,
        JudgeSeverity::Warn => Severity::Warn,
        JudgeSeverity::Error => Severity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::RuleSource;
    use sextant_judge::{FakeJudge, JudgeFinding, JudgeResult, JudgeSeverity};

    fn parsed(body: &str) -> ParsedRule {
        let text = format!(
            r#"---
id: repo.llm.demo
name: "LLM demo"
description: "x"
severity: warn
category: style
languages: [rust]
evaluator: {{ type: llm }}
---

{body}
"#
        );
        parse_rule_md(&text, RuleSource::Repo, None).unwrap()
    }

    fn judge_with(result: JudgeResult) -> Arc<Judge> {
        // The temp dir lives only as long as it takes to spin the
        // runtime; cache writes happen inside `judge_blocking` and we
        // don't assert on them in these unit tests.
        let dir = tempfile::tempdir().unwrap().keep();
        let provider = Arc::new(FakeJudge::always("fake", result));
        Arc::new(Judge::new(provider, dir).unwrap())
    }

    fn rule(judge: Arc<Judge>) -> LlmRule {
        LlmRule::from_parsed(
            parsed("Review:\n\n```\n{{code}}\n```\nFile: {{path}} (id: {{rule.id}})"),
            judge,
            LlmRuleSpec {
                model: "fake-1".into(),
                max_tokens: 256,
                temperature: 0.0,
                exclude_paths: vec!["**/skip/**".into()],
            },
        )
        .unwrap()
    }

    #[test]
    fn translates_judge_findings_into_findings() {
        let judge = judge_with(JudgeResult {
            findings: vec![JudgeFinding {
                severity: JudgeSeverity::Warn,
                message: "looks suspicious".into(),
                line: Some(3),
                end_line: Some(4),
            }],
        });
        let r = rule(judge);
        let file = SourceFile::new("a.rs", "line1\nline2\nline3\nline4\n");
        let root = std::env::current_dir().unwrap();
        let f = r.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
        assert_eq!(f[0].line, Some(3));
        assert_eq!(f[0].end_line, Some(4));
        assert!(f[0].message.contains("suspicious"));
    }

    #[test]
    fn skips_excluded_paths() {
        let judge = judge_with(JudgeResult { findings: vec![] });
        let r = rule(judge);
        let root = std::env::current_dir().unwrap();
        let file = SourceFile::new(root.join("skip").join("a.rs"), "fn x() {}\n");
        let f = r.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert!(f.is_empty());
    }

    #[test]
    fn renders_template_placeholders() {
        let r = rule(judge_with(JudgeResult { findings: vec![] }));
        let file = SourceFile::new("a.rs", "hello world\n");
        let rendered = r.render(&file, std::path::Path::new("a.rs"));
        assert!(rendered.contains("hello world"));
        assert!(rendered.contains("a.rs"));
        assert!(rendered.contains("repo.llm.demo"));
        assert!(!rendered.contains("{{"));
    }

    #[test]
    fn judge_error_degrades_to_info_finding() {
        // FakeJudge with no responses errors on the first call.
        let dir = tempfile::tempdir().unwrap().keep();
        let provider = Arc::new(FakeJudge::new("fake", vec![]));
        let judge = Arc::new(Judge::new(provider, dir).unwrap());
        let r = LlmRule::from_parsed(
            parsed("review {{code}}"),
            judge,
            LlmRuleSpec {
                model: "m".into(),
                max_tokens: 64,
                temperature: 0.0,
                exclude_paths: vec![],
            },
        )
        .unwrap();
        let file = SourceFile::new("a.rs", "fn x() {}\n");
        let root = std::env::current_dir().unwrap();
        let f = r.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
        assert!(f[0].message.contains("Judge"));
    }
}
