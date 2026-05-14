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

fn evaluate_at(root: &std::path::Path, path: &std::path::Path, src: &str) -> Vec<Finding> {
    let rule = PubFnUntestedRule::from_parsed(parsed_for_test());
    let file = SourceFile::new(path, src);
    rule.evaluate_file(&file, &EvalContext { repo_root: root })
}

#[test]
fn flags_pub_fn_with_no_test() {
    let f = evaluate("a.rs", "pub fn lonely() {}\n");
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, Severity::Info);
    assert!(f[0].message.contains("lonely"));
}

#[test]
fn rust_finding_carries_peer_file_create_patch() {
    let f = evaluate("a.rs", "pub fn lonely() {}\n");
    let patch = f[0].patch.as_deref().expect("expected stub patch");
    assert!(patch.contains("--- /dev/null"));
    assert!(patch.contains("+++ b/a_tests.rs"));
    assert!(patch.contains("use super::*;"));
    assert!(patch.contains("fn lonely_smoke"));
}

#[test]
fn js_finding_carries_peer_file_create_patch() {
    let f = evaluate("a.ts", "export function lonely() {}\n");
    let patch = f[0].patch.as_deref().expect("expected stub patch");
    assert!(patch.contains("--- /dev/null"));
    assert!(patch.contains("+++ b/a.test.ts"));
    assert!(patch.contains("from 'vitest'"));
    assert!(patch.contains("import { lonely } from './a'"));
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

#[test]
fn rust_quiet_when_sibling_tests_file_mentions_fn() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let src_path = root.join("foo.rs");
    std::fs::write(&src_path, "pub fn foo() {}\n").unwrap();
    std::fs::write(
        root.join("foo_tests.rs"),
        "use super::*;\n#[test]\nfn t() { foo(); }\n",
    )
    .unwrap();
    assert!(evaluate_at(root, &src_path, "pub fn foo() {}\n").is_empty());
}

#[test]
fn rust_quiet_when_crate_tests_dir_mentions_fn() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    let src_path = root.join("src/foo.rs");
    std::fs::write(&src_path, "pub fn foo() {}\n").unwrap();
    std::fs::write(root.join("tests/foo.rs"), "#[test] fn t() { foo(); }\n").unwrap();
    assert!(evaluate_at(root, &src_path, "pub fn foo() {}\n").is_empty());
}

#[test]
fn rust_finding_omits_patch_when_peer_file_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let src_path = root.join("foo.rs");
    std::fs::write(&src_path, "pub fn foo() {}\n").unwrap();
    std::fs::write(root.join("foo_tests.rs"), "// unrelated\n").unwrap();
    let f = evaluate_at(root, &src_path, "pub fn foo() {}\n");
    assert_eq!(f.len(), 1);
    assert!(
        f[0].patch.is_none(),
        "patch should be omitted: {:?}",
        f[0].patch
    );
}

/// Assert the rule stays quiet when an exported TS `foo` is mentioned by
/// a peer test file at `peer_rel` with the given contents. The peer
/// path's parent directory is created if needed.
fn assert_ts_quiet_with_peer(peer_rel: &str, peer_contents: &str) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let peer_path = root.join(peer_rel);
    if let Some(parent) = peer_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let src_path = root.join("foo.ts");
    std::fs::write(&src_path, "export function foo() {}\n").unwrap();
    std::fs::write(&peer_path, peer_contents).unwrap();
    assert!(evaluate_at(root, &src_path, "export function foo() {}\n").is_empty());
}

#[test]
fn ts_quiet_when_sibling_test_file_mentions_fn() {
    assert_ts_quiet_with_peer(
        "foo.test.ts",
        "import { foo } from './foo';\ndescribe('foo', () => { it('w', () => { foo(); }); });\n",
    );
}

#[test]
fn ts_quiet_when_sibling_spec_file_mentions_fn() {
    assert_ts_quiet_with_peer(
        "foo.spec.ts",
        "test('foo', () => { expect(foo).toBeDefined(); });\n",
    );
}

#[test]
fn ts_quiet_when_underscore_tests_dir_mentions_fn() {
    assert_ts_quiet_with_peer(
        "__tests__/foo.test.ts",
        "describe('foo', () => { it('w', () => { foo(); }); });\n",
    );
}
