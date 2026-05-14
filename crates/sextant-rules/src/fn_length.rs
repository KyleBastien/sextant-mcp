use sextant_config::SizeRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};

use crate::file_length::rule_from_parsed;
use crate::function_walker::for_each_function;
use crate::loader::ParsedRule;

pub struct FnLengthRule {
    rule: Rule,
    warn: u32,
    error: u32,
}

impl FnLengthRule {
    pub fn from_parsed(parsed: ParsedRule, cfg: &SizeRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            warn: cfg.fn_length_warn,
            error: cfg.fn_length_error,
        }
    }
}

impl Evaluator for FnLengthRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        for_each_function(file, ctx, |f, path| {
            let len = f.line_count();
            let (severity, threshold, suffix) = if len >= self.error {
                (Severity::Error, self.error, "Extract helpers.")
            } else if len >= self.warn {
                (Severity::Warn, self.warn, "Consider extracting.")
            } else {
                return None;
            };
            Some(
                Finding::new(
                    &self.rule.id,
                    severity,
                    path.to_path_buf(),
                    format!(
                        "Function `{}` is {len} lines (threshold: {threshold}). {suffix}",
                        f.name
                    ),
                )
                .spanning(f.start_line, f.end_line),
            )
        })
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
id: builtin.size.fn-length
name: "Function length"
description: "test"
severity: warn
category: size
languages: [rust]
evaluator: { type: builtin, name: fn_length }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    #[test]
    fn flags_long_functions() {
        let cfg = SizeRuleConfig {
            fn_length_warn: 5,
            fn_length_error: 10,
            ..Default::default()
        };
        let rule = FnLengthRule::from_parsed(parsed_for_test(), &cfg);
        let body = "    1;\n".repeat(12);
        let src = format!("fn big() {{\n{body}}}\n");
        let file = SourceFile::new("a.rs", src);
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(
            &file,
            &EvalContext {
                repo_root: root.as_path(),
            },
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Error);
        assert!(f[0].message.contains("`big`"));
    }

    #[test]
    fn ignores_short_functions() {
        let cfg = SizeRuleConfig {
            fn_length_warn: 10,
            fn_length_error: 20,
            ..Default::default()
        };
        let rule = FnLengthRule::from_parsed(parsed_for_test(), &cfg);
        let file = SourceFile::new("a.rs", "fn small() { let x = 1; }\n");
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(
            &file,
            &EvalContext {
                repo_root: root.as_path(),
            },
        );
        assert!(f.is_empty());
    }
}
