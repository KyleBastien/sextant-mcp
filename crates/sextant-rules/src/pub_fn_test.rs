//! Flag public-API definitions whose name doesn't appear in any in-file
//! test body. For Rust this is `pub fn` vs `#[test]` bodies; for
//! JavaScript/TypeScript it's `export`-ed declarations vs `describe` /
//! `it` / `test` callback bodies.
//!
//! Severity is `info` — this is a signal that helps the agent decide
//! where to focus, not a verdict-breaker. Cross-file detection (tests in
//! `tests/`, a separate integration crate, or a sibling `*.test.ts`
//! file) is intentionally out of scope until the engine grows a
//! repo-scope evaluator API.

use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};
use sextant_lang::{parse, test_haystack_mentions, test_witness, Language};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;
use crate::patch::append_diff;

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
        let Some(hint) = file.language_hint() else {
            return Vec::new();
        };
        let Some(lang) = Language::from_hint(hint) else {
            return Vec::new();
        };
        if !matches!(
            lang,
            Language::Rust | Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return Vec::new();
        }
        let parsed = match parse(file.contents.clone(), lang) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let witness = test_witness(&parsed);
        if witness.pub_fns.is_empty() {
            return Vec::new();
        }
        let path = file.relative_to(ctx.repo_root);
        let mut out = Vec::new();
        for pf in &witness.pub_fns {
            if test_haystack_mentions(&witness.test_haystack, &pf.name) {
                continue;
            }
            let msg = message_for(lang, &pf.name);
            let mut finding = Finding::new(&self.rule.id, self.rule.severity, path.clone(), msg)
                .spanning(pf.start_line, pf.end_line);
            if let Some(stub) = stub_test_block(lang, &pf.name) {
                finding = finding.with_patch(append_diff(&path, &file.contents, &stub));
            }
            out.push(finding);
        }
        out
    }
}

/// Skeleton test block to append at the end of a file. Kept minimal — the
/// generated test compiles and runs, but `todo!()` (or its equivalent)
/// forces the author to fill in real coverage rather than rubber-stamp
/// the rule. Returns `None` for languages where the stub would be more
/// disruptive than helpful.
fn stub_test_block(lang: Language, name: &str) -> Option<String> {
    match lang {
        Language::Rust => Some(format!(
            "\n#[cfg(test)]\nmod {name}_tests {{\n    use super::*;\n\n    #[test]\n    fn {name}_smoke() {{\n        // TODO: exercise `{name}` and assert its behaviour.\n        let _ = {name};\n    }}\n}}\n"
        )),
        Language::JavaScript | Language::TypeScript | Language::Tsx => Some(format!(
            "\nif (import.meta.vitest) {{\n    const {{ describe, it, expect }} = import.meta.vitest;\n    describe('{name}', () => {{\n        it('is exercised', () => {{\n            // TODO: exercise `{name}` and assert its behaviour.\n            expect({name}).toBeDefined();\n        }});\n    }});\n}}\n"
        )),
        _ => None,
    }
}

fn message_for(lang: Language, name: &str) -> String {
    match lang {
        Language::Rust => format!(
            "Public function `{name}` is not referenced by any `#[test]` in this file. \
             Add a unit test or reduce visibility to `pub(crate)`."
        ),
        Language::JavaScript | Language::TypeScript | Language::Tsx => format!(
            "Exported `{name}` is not referenced by any `describe`/`it`/`test` block \
             in this file. Add an in-file test or drop the `export`."
        ),
        _ => format!("Public `{name}` is not referenced by any test in this file."),
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
languages: [rust, javascript, typescript, tsx]
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
    fn rust_finding_carries_stub_test_patch() {
        let f = evaluate("a.rs", "pub fn lonely() {}\n");
        let patch = f[0].patch.as_deref().expect("expected stub patch");
        assert!(patch.contains("--- a/a.rs"));
        assert!(patch.contains("mod lonely_tests"));
        assert!(patch.contains("#[test]"));
    }

    #[test]
    fn js_finding_carries_vitest_stub_patch() {
        let f = evaluate("a.ts", "export function lonely() {}\n");
        let patch = f[0].patch.as_deref().expect("expected stub patch");
        assert!(patch.contains("import.meta.vitest"));
        assert!(patch.contains("describe('lonely'"));
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
    fn ignores_unsupported_languages() {
        assert!(evaluate("a.py", "def f(): pass\n").is_empty());
        assert!(evaluate("a.go", "package x\nfunc F() {}\n").is_empty());
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

    #[test]
    fn js_flags_exported_fn_with_no_test() {
        let f = evaluate("a.js", "export function lonely() {}\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].severity, Severity::Info);
        assert!(f[0].message.contains("lonely"));
        assert!(f[0].message.contains("describe"));
    }

    #[test]
    fn js_quiet_when_describe_mentions_fn() {
        let src = "export function add(a, b) { return a + b; }\n\
                   describe('add', () => {\n\
                     it('sums', () => { expect(add(1, 2)).toBe(3); });\n\
                   });\n";
        assert!(evaluate("a.js", src).is_empty());
    }

    #[test]
    fn js_ignores_non_exported_top_level_fn() {
        assert!(evaluate("a.js", "function privateFn() {}\n").is_empty());
    }

    #[test]
    fn ts_flags_exported_arrow_with_no_test() {
        let f = evaluate(
            "a.ts",
            "export const square = (x: number): number => x * x;\n",
        );
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("square"));
    }

    #[test]
    fn ts_quiet_when_test_mentions_fn() {
        let src = "export const square = (x: number): number => x * x;\n\
                   test('square', () => { expect(square(3)).toBe(9); });\n";
        assert!(evaluate("a.ts", src).is_empty());
    }

    #[test]
    fn tsx_flags_untested_exported_component() {
        let src = "export const Hello = ({ name }: { name: string }) => <div>Hi {name}</div>;\n";
        let f = evaluate("a.tsx", src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("Hello"));
    }
}
