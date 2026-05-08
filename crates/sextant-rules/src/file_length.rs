use sextant_config::SizeRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};

use crate::loader::ParsedRule;

pub struct FileLengthRule {
    rule: Rule,
    warn: u32,
    error: u32,
}

impl FileLengthRule {
    pub fn from_parsed(parsed: ParsedRule, cfg: &SizeRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            warn: cfg.file_length_warn,
            error: cfg.file_length_error,
        }
    }
}

pub(crate) fn rule_from_parsed(parsed: ParsedRule) -> Rule {
    Rule {
        id: parsed.id,
        name: parsed.name,
        description: parsed.description,
        body: parsed.body,
        severity: parsed.severity,
        category: parsed.category,
        scope: parsed.scope,
        languages: parsed.languages,
        enabled: parsed.enabled,
        tags: parsed.tags,
        source: parsed.source,
    }
}

impl Evaluator for FileLengthRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        let lines = file.line_count() as u32;
        let path = file.relative_to(ctx.repo_root);
        if lines >= self.error {
            vec![Finding::new(
                &self.rule.id,
                Severity::Error,
                path,
                format!(
                    "File has {lines} lines (threshold: {}). Split this file into smaller modules.",
                    self.error
                ),
            )]
        } else if lines >= self.warn {
            vec![Finding::new(
                &self.rule.id,
                Severity::Warn,
                path,
                format!(
                    "File has {lines} lines (threshold: {}). Consider splitting before it grows further.",
                    self.warn
                ),
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::RuleSource;

    fn parsed_for_test() -> ParsedRule {
        parse_rule_md(
            r#"---
id: builtin.size.file-length
name: "File length"
description: "test"
severity: warn
category: size
evaluator: { type: builtin, name: file_length }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    fn evaluate(warn: u32, error: u32, lines: usize) -> Vec<Finding> {
        let cfg = SizeRuleConfig {
            file_length_warn: warn,
            file_length_error: error,
            ..Default::default()
        };
        let rule = FileLengthRule::from_parsed(parsed_for_test(), &cfg);
        let file = SourceFile::new("a.rs", "x\n".repeat(lines));
        let root = std::env::current_dir().unwrap();
        rule.evaluate_file(&file, &EvalContext { repo_root: &root })
    }

    #[test]
    fn under_threshold_is_clean() {
        assert!(evaluate(100, 200, 50).is_empty());
    }

    #[test]
    fn warn_at_warn_threshold() {
        let f = evaluate(10, 20, 15);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
    }

    #[test]
    fn error_at_error_threshold() {
        let f = evaluate(10, 20, 25);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Error);
    }
}
