//! Third per-rule integration test file for the TypeScript pack, split
//! out from `typescript_pack_more.rs` so the macro-table boilerplate
//! doesn't trip `builtin.duplication.tokens`. Covers the five
//! loose-typing rules added alongside the autofix machinery
//! (no-never-annotation, no-jsdoc-types, no-ambient-module-shim,
//! no-empty-type-construction, no-implicit-any-field) plus their
//! patch-shape assertions for the two regex rules.

mod common;

use common::{load_rule as build_ast, parse_pack_rule};
use sextant_core::{EvalContext, Evaluator, Finding, SourceFile};
use sextant_rules::{EvaluatorSpec, RegexRule};

fn build_regex(filename: &str) -> RegexRule {
    let parsed = parse_pack_rule(filename);
    let (pattern, replacement) = match &parsed.evaluator {
        EvaluatorSpec::Regex {
            pattern,
            replacement,
        } => (pattern.clone(), replacement.clone()),
        other => panic!("expected regex evaluator for {filename}, got {other:?}"),
    };
    RegexRule::from_parsed(parsed, &pattern, replacement.as_deref())
        .unwrap_or_else(|e| panic!("building {filename}: {e}"))
}

fn count_ast(rule_file: &str, body: &str) -> usize {
    let rule = build_ast(rule_file);
    let file = SourceFile::new("a.ts", body);
    let root = std::env::current_dir().unwrap();
    rule.evaluate_file(&file, &EvalContext { repo_root: &root })
        .len()
}

fn regex_findings(rule_file: &str, body: &str) -> Vec<Finding> {
    let rule = build_regex(rule_file);
    let file = SourceFile::new("a.ts", body);
    let root = std::env::current_dir().unwrap();
    rule.evaluate_file(&file, &EvalContext { repo_root: &root })
}

fn assert_count(rule_file: &str, body: &str, expected: usize) {
    let actual = count_ast(rule_file, body);
    assert_eq!(
        actual, expected,
        "{rule_file}: expected {expected} findings, got {actual}\nbody:\n{body}",
    );
}

fn assert_single_line_patch(findings: &[Finding], expected_old: &str, expected_new: Option<&str>) {
    assert_eq!(findings.len(), 1, "expected 1 finding, got {findings:?}");
    let patch = findings[0].patch.as_deref().expect("patch attached");
    assert!(patch.contains("@@ -1,1 +1,1 @@"), "patch was: {patch}");
    let old_marker = format!("-{expected_old}");
    assert!(patch.contains(&old_marker), "patch was: {patch}");
    if let Some(new_text) = expected_new {
        let new_marker = format!("+{new_text}");
        assert!(patch.contains(&new_marker), "patch was: {patch}");
    }
}

macro_rules! extra_tests {
    ($($name:ident : $rule:expr => ($body:expr, $expected:expr));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                assert_count($rule, $body, $expected);
            }
        )+
    };
}

extra_tests! {
    never_annotation_on_return_type: "no-never-annotation.md" =>
        ("function fail(): never { throw new Error('x'); }\n", 1);
    never_annotation_on_const: "no-never-annotation.md" =>
        ("const x: never = (() => { throw 0; })();\n", 1);
    never_annotation_on_param: "no-never-annotation.md" =>
        ("function g(x: never) {}\n", 1);
    never_annotation_allows_conditional_type: "no-never-annotation.md" =>
        ("type NonNull<T> = T extends null | undefined ? never : T;\n", 0);
    never_annotation_skips_string_literal: "no-never-annotation.md" =>
        ("const s = \"never\";\n", 0);
    empty_construction_pick_never: "no-empty-type-construction.md" =>
        ("type E = Pick<User, never>;\n", 1);
    empty_construction_record_never: "no-empty-type-construction.md" =>
        ("type R = Record<never, string>;\n", 1);
    empty_construction_omit_keyof: "no-empty-type-construction.md" =>
        ("type O = Omit<User, keyof User>;\n", 1);
    empty_construction_allows_pick_with_keys: "no-empty-type-construction.md" =>
        ("type P = Pick<User, \"id\" | \"email\">;\n", 0);
    empty_construction_allows_record_with_string_keys: "no-empty-type-construction.md" =>
        ("type R = Record<string, number>;\n", 0);
    implicit_any_field_interface: "no-implicit-any-field.md" =>
        ("interface User { id; email: string }\n", 1);
    implicit_any_field_class_no_initializer: "no-implicit-any-field.md" =>
        ("class C { count; ready: boolean = false }\n", 1);
    implicit_any_field_type_literal: "no-implicit-any-field.md" =>
        ("type T = { id; name: string };\n", 1);
    implicit_any_field_allows_annotated_interface: "no-implicit-any-field.md" =>
        ("interface User { id: string; email: string }\n", 0);
    implicit_any_field_allows_annotated_class: "no-implicit-any-field.md" =>
        ("class C { count: number = 0 }\n", 0);
    implicit_any_field_allows_class_field_with_initializer: "no-implicit-any-field.md" =>
        ("class C { ready = false; start = () => {}; }\n", 0);
}

#[test]
fn jsdoc_types_strips_payload_and_attaches_patch() {
    let findings = regex_findings(
        "no-jsdoc-types.md",
        "/** @type {string} */\nconst x = \"hi\";\n",
    );
    assert_single_line_patch(&findings, "/** @type {string} */", Some("/** @type */"));
}

#[test]
fn jsdoc_types_fires_on_param_and_returns_tags() {
    let findings = regex_findings(
        "no-jsdoc-types.md",
        "/** @param {number} n */\n/** @returns {boolean} */\nfunction f(n) { return true; }\n",
    );
    assert_eq!(findings.len(), 2, "expected 2 findings, got {findings:?}");
}

#[test]
fn jsdoc_types_skips_plain_jsdoc_without_brace_form() {
    let findings = regex_findings(
        "no-jsdoc-types.md",
        "/** Plain doc.\n * @param n the count\n */\nfunction h(n: number): boolean { return true; }\n",
    );
    assert_eq!(findings.len(), 0, "expected 0 findings, got {findings:?}");
}

#[test]
fn ambient_shim_fires_and_attaches_deletion_patch() {
    let findings = regex_findings(
        "no-ambient-module-shim.md",
        "declare module \"untyped-pkg\" {}\n",
    );
    assert_single_line_patch(&findings, "declare module \"untyped-pkg\" {}", None);
}

#[test]
fn ambient_shim_matches_wildcard_form() {
    let findings = regex_findings("no-ambient-module-shim.md", "declare module \"*.svg\" {}\n");
    assert_eq!(findings.len(), 1, "expected 1 finding, got {findings:?}");
}

#[test]
fn ambient_shim_skips_populated_declaration() {
    let findings = regex_findings(
        "no-ambient-module-shim.md",
        "declare module \"my-lib\" { export function load(p: string): Buffer; }\n",
    );
    assert_eq!(findings.len(), 0, "expected 0 findings, got {findings:?}");
}
