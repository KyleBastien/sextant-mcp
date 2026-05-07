use sextant_config::SizeRuleConfig;
use sextant_core::{Category, EvalContext, Evaluator, Finding, Rule, Scope, Severity, SourceFile};

pub const RULE_ID: &str = "builtin.size.file-length";

pub struct FileLengthRule {
    rule: Rule,
    warn: u32,
    error: u32,
}

impl FileLengthRule {
    pub fn from_config(cfg: &SizeRuleConfig) -> Self {
        Self {
            rule: Rule {
                id: RULE_ID.to_string(),
                name: "File length".to_string(),
                description:
                    "Flags files that exceed the configured line-count thresholds. Long files \
                     are usually doing too many things; split them up."
                        .to_string(),
                severity: Severity::Warn,
                category: Category::Size,
                scope: Scope::File,
                languages: Vec::new(),
                enabled: true,
                tags: vec!["size".into()],
            },
            warn: cfg.file_length_warn,
            error: cfg.file_length_error,
        }
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
                RULE_ID,
                Severity::Error,
                path,
                format!(
                    "File has {lines} lines (threshold: {}). Split this file into smaller modules.",
                    self.error
                ),
            )]
        } else if lines >= self.warn {
            vec![Finding::new(
                RULE_ID,
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
    use std::path::Path;

    fn ctx<'a>(root: &'a Path) -> EvalContext<'a> {
        EvalContext { repo_root: root }
    }

    #[test]
    fn under_threshold_is_clean() {
        let cfg = SizeRuleConfig {
            file_length_warn: 100,
            file_length_error: 200,
        };
        let rule = FileLengthRule::from_config(&cfg);
        let file = SourceFile::new("a.rs", "x\n".repeat(50));
        let root = std::env::current_dir().unwrap();
        let findings = rule.evaluate_file(&file, &ctx(&root));
        assert!(findings.is_empty());
    }

    #[test]
    fn warn_at_warn_threshold() {
        let cfg = SizeRuleConfig {
            file_length_warn: 10,
            file_length_error: 20,
        };
        let rule = FileLengthRule::from_config(&cfg);
        let file = SourceFile::new("a.rs", "x\n".repeat(15));
        let root = std::env::current_dir().unwrap();
        let findings = rule.evaluate_file(&file, &ctx(&root));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warn);
    }

    #[test]
    fn error_at_error_threshold() {
        let cfg = SizeRuleConfig {
            file_length_warn: 10,
            file_length_error: 20,
        };
        let rule = FileLengthRule::from_config(&cfg);
        let file = SourceFile::new("a.rs", "x\n".repeat(25));
        let root = std::env::current_dir().unwrap();
        let findings = rule.evaluate_file(&file, &ctx(&root));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
    }
}
