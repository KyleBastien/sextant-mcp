use sextant_config::SizeRuleConfig;
use sextant_core::{Category, EvalContext, Evaluator, Finding, Rule, Scope, Severity, SourceFile};
use sextant_lang::{function_ranges, parse, Language};

pub const RULE_ID: &str = "builtin.size.fn-length";

pub struct FnLengthRule {
    rule: Rule,
    warn: u32,
    error: u32,
}

impl FnLengthRule {
    pub fn from_config(cfg: &SizeRuleConfig) -> Self {
        Self {
            rule: Rule {
                id: RULE_ID.to_string(),
                name: "Function length".to_string(),
                description: "Flags functions whose body spans more than the configured number \
                              of lines. Long functions are usually doing too many things; \
                              extract helpers."
                    .to_string(),
                severity: Severity::Warn,
                category: Category::Size,
                scope: Scope::File,
                languages: vec!["rust".into()],
                enabled: true,
                tags: vec!["size".into(), "complexity".into()],
            },
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
                        RULE_ID,
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
                        RULE_ID,
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

    #[test]
    fn flags_long_functions() {
        let cfg = SizeRuleConfig {
            fn_length_warn: 5,
            fn_length_error: 10,
            ..Default::default()
        };
        let rule = FnLengthRule::from_config(&cfg);
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
        let rule = FnLengthRule::from_config(&cfg);
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

    #[test]
    fn skips_non_rust_files() {
        let cfg = SizeRuleConfig::default();
        let rule = FnLengthRule::from_config(&cfg);
        let file = SourceFile::new("a.py", "def big():\n    pass\n");
        let root = std::env::current_dir().unwrap();
        // language filter handled at RuleSet level, but evaluator alone should be safe.
        let f = rule.evaluate_file(
            &file,
            &EvalContext {
                repo_root: root.as_path(),
            },
        );
        assert!(f.is_empty());
    }
}
