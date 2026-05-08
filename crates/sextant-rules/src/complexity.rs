//! Cyclomatic and max-nesting rules built on `sextant_lang::function_complexity`.
//!
//! Both rules share the same evaluation skeleton — parse, walk, compare each
//! function's metric against thresholds — and only differ in which metric
//! they read. They're a single `ComplexityRule` parameterized by `Metric`.

use sextant_config::ComplexityRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, Severity, SourceFile};
use sextant_lang::{function_complexity, parse, FunctionComplexity, Language};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;

#[derive(Debug, Clone, Copy)]
pub enum Metric {
    Cyclomatic,
    Nesting,
}

impl Metric {
    fn read(self, fc: &FunctionComplexity) -> u32 {
        match self {
            Metric::Cyclomatic => fc.cyclomatic,
            Metric::Nesting => fc.max_nesting,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Metric::Cyclomatic => "cyclomatic complexity",
            Metric::Nesting => "max nesting depth",
        }
    }
}

pub struct ComplexityRule {
    rule: Rule,
    warn: u32,
    error: u32,
    metric: Metric,
}

impl ComplexityRule {
    pub fn cyclomatic(parsed: ParsedRule, cfg: &ComplexityRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            warn: cfg.cyclomatic_warn,
            error: cfg.cyclomatic_error,
            metric: Metric::Cyclomatic,
        }
    }

    pub fn nesting(parsed: ParsedRule, cfg: &ComplexityRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            warn: cfg.nesting_warn,
            error: cfg.nesting_error,
            metric: Metric::Nesting,
        }
    }
}

impl Evaluator for ComplexityRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        evaluate(EvalArgs {
            rule: &self.rule,
            metric: self.metric,
            warn: self.warn,
            error: self.error,
            file,
            ctx,
        })
    }
}

struct EvalArgs<'a> {
    rule: &'a Rule,
    metric: Metric,
    warn: u32,
    error: u32,
    file: &'a SourceFile,
    ctx: &'a EvalContext<'a>,
}

fn evaluate(args: EvalArgs<'_>) -> Vec<Finding> {
    let Some(hint) = args.file.language_hint() else {
        return Vec::new();
    };
    let Some(lang) = Language::from_hint(hint) else {
        return Vec::new();
    };
    let parsed = match parse(args.file.contents.clone(), lang) {
        Ok(p) => p,
        Err(err) => {
            tracing::debug!(?err, path=?args.file.path, "parse failed");
            return Vec::new();
        }
    };
    let fns = match function_complexity(&parsed) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let path = args.file.relative_to(args.ctx.repo_root);
    let mut out = Vec::new();
    for fc in fns {
        let value = args.metric.read(&fc);
        let (sev, threshold) = if value >= args.error {
            (Severity::Error, args.error)
        } else if value >= args.warn {
            (Severity::Warn, args.warn)
        } else {
            continue;
        };
        let msg = format!(
            "Function `{}` has {} of {value} (threshold: {threshold}).",
            fc.name,
            args.metric.label(),
        );
        out.push(
            Finding::new(&args.rule.id, sev, path.clone(), msg)
                .spanning(fc.start_line, fc.end_line),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::RuleSource;

    fn cyclomatic_parsed() -> ParsedRule {
        parse_rule_md(
            r#"---
id: builtin.complexity.cyclomatic
name: "Cyclomatic complexity"
description: "x"
severity: warn
category: complexity
languages: [rust, python]
evaluator: { type: builtin, name: cyclomatic }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    fn nesting_parsed() -> ParsedRule {
        parse_rule_md(
            r#"---
id: builtin.complexity.nesting
name: "Nesting"
description: "x"
severity: warn
category: complexity
languages: [rust, python]
evaluator: { type: builtin, name: nesting }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    fn root_ctx() -> std::path::PathBuf {
        std::env::current_dir().unwrap()
    }

    #[test]
    fn cyclomatic_flags_branchy_function() {
        let cfg = ComplexityRuleConfig {
            cyclomatic_warn: 3,
            cyclomatic_error: 5,
            ..Default::default()
        };
        let rule = ComplexityRule::cyclomatic(cyclomatic_parsed(), &cfg);
        let src = "fn f(x: i32) -> i32 { if x > 0 { 1 } else if x < 0 { -1 } else { 0 } }\n";
        let file = SourceFile::new("a.rs", src);
        let root = root_ctx();
        let f = rule.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].severity, Severity::Warn);
    }

    #[test]
    fn cyclomatic_clean_when_simple() {
        let cfg = ComplexityRuleConfig::default();
        let rule = ComplexityRule::cyclomatic(cyclomatic_parsed(), &cfg);
        let file = SourceFile::new("a.rs", "fn ok() { let x = 1; }\n");
        let root = root_ctx();
        let f = rule.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert!(f.is_empty());
    }

    #[test]
    fn nesting_flags_deep_function() {
        let cfg = ComplexityRuleConfig {
            nesting_warn: 2,
            nesting_error: 3,
            ..Default::default()
        };
        let rule = ComplexityRule::nesting(nesting_parsed(), &cfg);
        let src = r#"
fn f() {
    if true {
        for _ in 0..1 {
            if true {
                while true { break; }
            }
        }
    }
}
"#;
        let file = SourceFile::new("a.rs", src);
        let root = root_ctx();
        let f = rule.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].severity, Severity::Error);
    }

    #[test]
    fn cyclomatic_works_for_python() {
        let cfg = ComplexityRuleConfig {
            cyclomatic_warn: 3,
            cyclomatic_error: 5,
            ..Default::default()
        };
        let rule = ComplexityRule::cyclomatic(cyclomatic_parsed(), &cfg);
        let src = "def f(x):\n    if x > 0:\n        return 1\n    elif x < 0:\n        return -1\n    else:\n        return 0\n";
        let file = SourceFile::new("a.py", src);
        let root = root_ctx();
        let f = rule.evaluate_file(&file, &EvalContext { repo_root: &root });
        assert_eq!(f.len(), 1, "{f:?}");
    }
}
