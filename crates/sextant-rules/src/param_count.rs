use sextant_config::SizeRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};
use sextant_lang::{function_ranges, parse, Language};

use crate::file_length::rule_from_parsed;
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
        let Some(hint) = file.language_hint() else {
            return Vec::new();
        };
        let Some(lang) = Language::from_hint(hint) else {
            return Vec::new();
        };
        let parsed = match parse(file.contents.clone(), lang) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let fns = match function_ranges(&parsed) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };

        let path = file.relative_to(ctx.repo_root);
        let mut out = Vec::new();
        for f in fns {
            if f.param_count >= self.error {
                out.push(
                    Finding::new(
                        &self.rule.id,
                        Severity::Error,
                        path.clone(),
                        format!(
                            "Function `{}` takes {} parameters (threshold: {}). Group related parameters into a struct.",
                            f.name, f.param_count, self.error
                        ),
                    )
                    .spanning(f.start_line, f.end_line),
                );
            } else if f.param_count >= self.warn {
                out.push(
                    Finding::new(
                        &self.rule.id,
                        Severity::Warn,
                        path.clone(),
                        format!(
                            "Function `{}` takes {} parameters (threshold: {}). Consider grouping.",
                            f.name, f.param_count, self.warn
                        ),
                    )
                    .spanning(f.start_line, f.end_line),
                );
            }
        }
        out
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
