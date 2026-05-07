//! Flag `pub fn` definitions whose name doesn't appear in any
//! `#[test]`-annotated function body in the same file.
//!
//! Severity is `info` — this is a signal that helps the agent decide
//! where to focus, not a verdict-breaker. Cross-file detection (tests in
//! `tests/` or a separate integration crate) is intentionally out of
//! scope until the engine grows a repo-scope evaluator API.

use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};
use sextant_lang::{parse, rust_test_witness, test_haystack_mentions, Language};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;

pub struct PubFnUntestedRule {
    rule: Rule,
}

impl PubFnUntestedRule {
    pub fn from_parsed(parsed: ParsedRule) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
        }
    }
}

impl Evaluator for PubFnUntestedRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        if file.language_hint() != Some("rust") {
            return Vec::new();
        }
        let parsed = match parse(file.contents.clone(), Language::Rust) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let witness = rust_test_witness(&parsed);
        if witness.pub_fns.is_empty() {
            return Vec::new();
        }
        let path = file.relative_to(ctx.repo_root);
        let mut out = Vec::new();
        for pf in &witness.pub_fns {
            if test_haystack_mentions(&witness.test_haystack, &pf.name) {
                continue;
            }
            let msg = format!(
                "Public function `{}` is not referenced by any `#[test]` in this file. \
                 Add a unit test or reduce visibility to `pub(crate)`.",
                pf.name
            );
            out.push(
                Finding::new(&self.rule.id, self.rule.severity, path.clone(), msg)
                    .spanning(pf.start_line, pf.end_line),
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::{RuleSource, Severity};

    fn parsed_for_test() -> ParsedRule {
        parse_rule_md(
            r#"---
id: builtin.tests.pub-fn-untested
name: "Public function without test"
description: "x"
severity: info
category: tests
scope: file
languages: [rust]
evaluator: { type: builtin, name: pub_fn_untested }
---
"#,
            RuleSource::Builtin,
            None,
        )
        .unwrap()
    }

    fn evaluate(path: &str, src: &str) -> Vec<Finding> {
        let rule = PubFnUntestedRule::from_parsed(parsed_for_test());
        let file = SourceFile::new(path, src);
        let root = std::env::current_dir().unwrap();
        rule.evaluate_file(&file, &EvalContext { repo_root: &root })
    }

    #[test]
    fn flags_pub_fn_with_no_test() {
        let f = evaluate("a.rs", "pub fn lonely() {}\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].severity, Severity::Info);
        assert!(f[0].message.contains("lonely"));
    }

    #[test]
    fn quiet_when_test_mentions_fn() {
        let src = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_works() { assert_eq!(add(1, 2), 3); }
}
"#;
        assert!(evaluate("a.rs", src).is_empty());
    }

    #[test]
    fn ignores_non_rust_files() {
        assert!(evaluate("a.py", "def f(): pass\n").is_empty());
    }

    #[test]
    fn ignores_pub_crate() {
        assert!(evaluate("a.rs", "pub(crate) fn internal() {}\n").is_empty());
    }

    #[test]
    fn ignores_pub_fns_in_cfg_test_mod() {
        let src = r#"
#[cfg(test)]
mod tests {
    pub fn helper() {}
    #[test]
    fn t() { helper(); }
}
"#;
        assert!(evaluate("a.rs", src).is_empty());
    }

    #[test]
    fn flags_each_untested_fn() {
        let src = r#"
pub fn one() {}
pub fn two() {}
pub fn three() {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t() { two(); }
}
"#;
        let f = evaluate("a.rs", src);
        let names: Vec<_> = f
            .iter()
            .filter_map(|x| x.message.split('`').nth(1))
            .collect();
        assert_eq!(names, vec!["one", "three"]);
    }
}
