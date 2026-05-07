//! Token-duplication evaluator built on `sextant_lang::find_clones`.
//!
//! Each clone pair produces TWO findings — one anchored at each occurrence,
//! each pointing at the other. This is deliberate: in `grade_diff` the
//! finding whose anchor is inside a changed line set is the one that
//! survives the diff filter, so a developer only sees the side of the
//! clone they're touching.

use sextant_config::DuplicationRuleConfig;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};
use sextant_lang::{find_clones, parse, ClonePair, Language};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;

pub struct DuplicationRule {
    rule: Rule,
    min_tokens: usize,
}

impl DuplicationRule {
    pub fn from_parsed(parsed: ParsedRule, cfg: &DuplicationRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            min_tokens: cfg.min_tokens as usize,
        }
    }
}

impl Evaluator for DuplicationRule {
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
        let clones = find_clones(&parsed, self.min_tokens);
        let path = file.relative_to(ctx.repo_root);
        let mut out = Vec::with_capacity(clones.len() * 2);
        for c in clones {
            push_pair(&self.rule, &path, &c, &mut out);
        }
        out
    }
}

fn push_pair(rule: &Rule, path: &std::path::Path, c: &ClonePair, out: &mut Vec<Finding>) {
    let token_count = c.token_count;
    let msg_a = format!(
        "Duplicate of lines {}-{} ({} tokens). Extract a helper.",
        c.b.start_line, c.b.end_line, token_count
    );
    let msg_b = format!(
        "Duplicate of lines {}-{} ({} tokens). Extract a helper.",
        c.a.start_line, c.a.end_line, token_count
    );
    out.push(
        Finding::new(&rule.id, rule.severity, path.to_path_buf(), msg_a)
            .spanning(c.a.start_line, c.a.end_line),
    );
    out.push(
        Finding::new(&rule.id, rule.severity, path.to_path_buf(), msg_b)
            .spanning(c.b.start_line, c.b.end_line),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::RuleSource;

    fn parsed_for_test() -> ParsedRule {
        parse_rule_md(
            r#"---
id: builtin.duplication.tokens
name: "Token duplication"
description: "x"
severity: warn
category: duplication
languages: [rust, python]
evaluator: { type: builtin, name: tokens_dup }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    #[test]
    fn flags_two_findings_per_clone() {
        let cfg = DuplicationRuleConfig { min_tokens: 20 };
        let rule = DuplicationRule::from_parsed(parsed_for_test(), &cfg);
        let src = r#"
fn one() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}

fn two() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}
"#;
        let file = SourceFile::new("a.rs", src);
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(
            &file,
            &EvalContext {
                repo_root: root.as_path(),
            },
        );
        assert_eq!(f.len(), 2, "{f:?}");
        // Each finding points at the other's lines in its message.
        assert!(f[0].message.contains("lines"));
        assert!(f[1].message.contains("lines"));
    }

    #[test]
    fn quiet_when_no_duplication() {
        let cfg = DuplicationRuleConfig::default();
        let rule = DuplicationRule::from_parsed(parsed_for_test(), &cfg);
        let file = SourceFile::new("a.rs", "fn ok() { let x = 1; }\n");
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
    fn skips_unsupported_languages() {
        let cfg = DuplicationRuleConfig { min_tokens: 5 };
        let rule = DuplicationRule::from_parsed(parsed_for_test(), &cfg);
        let file = SourceFile::new("a.txt", "anything\nat all\n");
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
