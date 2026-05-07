use sextant_config::SizeRuleConfig;
use sextant_core::{Category, EvalContext, Evaluator, Finding, Rule, Scope, Severity, SourceFile};
use sextant_lang::{function_ranges, parse, Language};

pub const RULE_ID: &str = "builtin.size.param-count";

pub struct ParamCountRule {
    rule: Rule,
    warn: u32,
    error: u32,
}

impl ParamCountRule {
    pub fn from_config(cfg: &SizeRuleConfig) -> Self {
        Self {
            rule: Rule {
                id: RULE_ID.to_string(),
                name: "Parameter count".to_string(),
                description: "Flags functions that take more than the configured number of \
                              parameters. Long parameter lists usually indicate a missing \
                              type or a function doing too much."
                    .to_string(),
                severity: Severity::Warn,
                category: Category::Size,
                scope: Scope::File,
                languages: vec!["rust".into()],
                enabled: true,
                tags: vec!["size".into(), "api".into()],
            },
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
                        RULE_ID,
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
                        RULE_ID,
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

    #[test]
    fn flags_at_threshold() {
        let cfg = SizeRuleConfig {
            param_count_warn: 3,
            param_count_error: 5,
            ..Default::default()
        };
        let rule = ParamCountRule::from_config(&cfg);
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
        let rule = ParamCountRule::from_config(&cfg);
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
