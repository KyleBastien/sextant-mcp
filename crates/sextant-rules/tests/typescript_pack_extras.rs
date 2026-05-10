//! Third per-rule integration test file for the TypeScript pack, split
//! out from `typescript_pack_more.rs` so the macro-table boilerplate
//! doesn't trip `builtin.duplication.tokens`. Covers the five
//! loose-typing rules added alongside the autofix machinery
//! (no-never-annotation, no-jsdoc-types, no-ambient-module-shim,
//! no-empty-type-construction, no-implicit-any-field) plus their
//! patch-shape assertions for the two regex rules.

use std::path::PathBuf;

use sextant_core::{EvalContext, Evaluator, Finding, RuleSource, SourceFile};
use sextant_rules::{parse_rule_md, AstRule, AstRuleSpec, EvaluatorSpec, ParsedRule, RegexRule};

fn pack_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("packs")
        .join("typescript")
}

fn parse_pack_rule(filename: &str) -> ParsedRule {
    let path = pack_root().join("rules").join(filename);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    parse_rule_md(
        &text,
        RuleSource::Vendor("typescript".into()),
        Some(path.clone()),
    )
    .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

fn build_ast(filename: &str) -> AstRule {
    let parsed = parse_pack_rule(filename);
    let (query, capture, message, not_under, exclude_paths) = match &parsed.evaluator {
        EvaluatorSpec::Ast {
            query,
            capture,
            message,
            not_under,
            exclude_paths,
        } => (
            query.clone(),
            capture.clone(),
            message.clone(),
            not_under.clone(),
            exclude_paths.clone(),
        ),
        other => panic!("expected ast evaluator, got {other:?}"),
    };
    AstRule::from_parsed(
        parsed,
        AstRuleSpec {
            query: &query,
            capture: capture.as_deref(),
            message: message.as_deref(),
            not_under: &not_under,
            exclude_paths: &exclude_paths,
        },
    )
    .unwrap_or_else(|e| panic!("building {filename}: {e}"))
}

fn build_regex(filename: &str) -> RegexRule {
    let parsed = parse_pack_rule(filename);
    let (pattern, replacement, exclude_paths) = match &parsed.evaluator {
        EvaluatorSpec::Regex {
            pattern,
            exclude_paths,
            replacement,
        } => (pattern.clone(), replacement.clone(), exclude_paths.clone()),
        other => panic!("expected regex evaluator for {filename}, got {other:?}"),
    };
    RegexRule::from_parsed(parsed, &pattern, &exclude_paths, replacement.as_deref())
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
