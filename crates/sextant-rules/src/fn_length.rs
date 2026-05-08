use sextant_config::SizeRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};
use sextant_lang::{function_ranges, parse, Language};

use crate::file_length::rule_from_parsed;
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
        let Some(hint) = file.language_hint() else {
            return Vec::new();
        };
        let Some(lang) = Language::from_hint(hint) else {
            return Vec::new();
        };
        let parsed = match parse(file.contents.clone(), lang) {
            Ok(p) => p,
            Err(err) => {
                tracing::debug!(?err, path=?file.path, "parse failed");
                return Vec::new();
            }
        };
        let fns = match function_ranges(&parsed) {
            Ok(f) => f,
            Err(err) => {
                tracing::debug!(?err, path=?file.path, "function_ranges failed");
                return Vec::new();
            }
        };

        let path = file.relative_to(ctx.repo_root);
        let mut out = Vec::new();
        for f in fns {
            let len = f.line_count();
            if len >= self.error {
                out.push(
                    Finding::new(
                        &self.rule.id,
                        Severity::Error,
                        path.clone(),
                        format!(
                            "Function `{}` is {len} lines (threshold: {}). Extract helpers.",
                            f.name, self.error
                        ),
                    )
                    .spanning(f.start_line, f.end_line),
                );
            } else if len >= self.warn {
                out.push(
                    Finding::new(
                        &self.rule.id,
                        Severity::Warn,
                        path.clone(),
                        format!(
                            "Function `{}` is {len} lines (threshold: {}). Consider extracting.",
                            f.name, self.warn
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
