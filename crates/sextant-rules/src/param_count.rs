use sextant_config::SizeRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};

use crate::file_length::rule_from_parsed;
use crate::function_walker::for_each_function;
use crate::loader::ParsedRule;

pub struct ParamCountRule {
    rule: Rule,
    warn: u32,
    error: u32,
}

impl ParamCountRule {
    pub fn from_parsed(parsed: ParsedRule, cfg: &SizeRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            warn: cfg.param_count_warn,
            error: cfg.param_count_error,
        }
    }
}

impl Evaluator for ParamCountRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        for_each_function(file, ctx, |f, path| {
            let (severity, threshold, suffix) = if f.param_count >= self.error {
                (
                    Severity::Error,
                    self.error,
                    "Group related parameters into a struct.",
                )
            } else if f.param_count >= self.warn {
                (Severity::Warn, self.warn, "Consider grouping.")
            } else {
                return None;
            };
            Some(
                Finding::new(
                    &self.rule.id,
                    severity,
                    path.to_path_buf(),
                    format!(
                        "Function `{}` takes {} parameters (threshold: {threshold}). {suffix}",
                        f.name, f.param_count
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
id: builtin.size.param-count
name: "Parameter count"
description: "test"
severity: warn
category: size
languages: [rust]
evaluator: { type: builtin, name: param_count }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    #[test]
    fn flags_at_threshold() {
        let cfg = SizeRuleConfig {
            param_count_warn: 3,
            param_count_error: 5,
            ..Default::default()
        };
        let rule = ParamCountRule::from_parsed(parsed_for_test(), &cfg);
        let file = SourceFile::new(
            "a.rs",
            "fn many(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}\n",
        );
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(
            &file,
            &EvalContext {
                repo_root: root.as_path(),
            },
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Error);
    }

    #[test]
    fn clean_when_under() {
        let cfg = SizeRuleConfig::default();
        let rule = ParamCountRule::from_parsed(parsed_for_test(), &cfg);
        let file = SourceFile::new("a.rs", "fn ok(x: i32) {}\n");
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
