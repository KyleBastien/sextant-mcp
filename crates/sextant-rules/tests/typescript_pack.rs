//! Per-rule integration tests for the TypeScript pack at
//! `<workspace_root>/packs/typescript/`. Each rule is loaded from its
//! markdown frontmatter, built into an `AstRule`, and exercised against
//! synthetic TS/TSX snippets — no .ts files are committed to the repo.

use std::path::PathBuf;

use sextant_core::{EvalContext, Evaluator, RuleSource, SourceFile};
use sextant_rules::{parse_rule_md, AstRule, AstRuleSpec, EvaluatorSpec};

fn pack_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("packs")
        .join("typescript")
}

fn load_rule(filename: &str) -> AstRule {
    let path = pack_root().join("rules").join(filename);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let parsed = parse_rule_md(
        &text,
        RuleSource::Vendor("typescript".into()),
        Some(path.clone()),
    )
    .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
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

fn count_findings(rule_file: &str, file_name: &str, body: &str) -> usize {
    let rule = load_rule(rule_file);
    let file = SourceFile::new(file_name, body);
    let root = std::env::current_dir().unwrap();
    let ctx = EvalContext { repo_root: &root };
    rule.evaluate_file(&file, &ctx).len()
}

fn assert_count(rule_file: &str, file_name: &str, body: &str, expected: usize) {
    let actual = count_findings(rule_file, file_name, body);
    assert_eq!(
        actual, expected,
        "{rule_file} on {file_name}: expected {expected} findings, got {actual}\nbody:\n{body}",
    );
}

/// Declare a batch of `#[test]` cases for the TS pack. Each entry is
/// `name : "rule.md" => ("file" , "body" , expected);`. The macro keeps
/// the test cases as separately-discoverable functions while the table
/// itself stays compact (no duplicated `assert_count` boilerplate).
macro_rules! ts_pack_tests {
    ($($name:ident : $rule:expr => ($file:expr, $body:expr, $expected:expr));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                assert_count($rule, $file, $body, $expected);
            }
        )+
    };
}

ts_pack_tests! {
    no_any_fires_on_type_annotation: "no-any.md" =>
        ("a.ts", "const x: any = 1;\n", 1);
    no_any_does_not_fire_on_string_or_comment_containing_any: "no-any.md" =>
        ("a.ts",
         "const a = \"any\";\n// any in a comment\nconst b: number = 1;\n",
         0);
    no_any_fires_in_generic_position: "no-any.md" =>
        ("a.ts", "const xs: Array<any> = [];\n", 1);
    no_unknown_fires_in_normal_type_position: "no-unknown.md" =>
        ("a.ts", "const x: unknown = 1;\n", 1);
    no_unknown_allows_unknown_in_catch_clause: "no-unknown.md" =>
        ("a.ts",
         "try { foo(); } catch (e: unknown) { throw e; }\n",
         0);
    no_unknown_still_fires_outside_catch_when_catch_present_elsewhere: "no-unknown.md" =>
        ("a.ts",
         "const x: unknown = 1;\ntry {} catch (e: unknown) {}\n",
         1);
    no_object_type_fires: "no-object-type.md" =>
        ("a.ts", "const x: object = {};\n", 1);
    // Lowercase `object` only fires as a type, not as a value identifier.
    no_object_type_allows_object_literal: "no-object-type.md" =>
        ("a.ts", "const Object = 1;\n", 0);
    no_as_cast_fires_on_basic_cast: "no-as-cast.md" =>
        ("a.ts", "const s = x as string;\n", 1);
    no_as_cast_allows_as_const: "no-as-cast.md" =>
        ("a.ts",
         "const s = \"hello\" as const;\nconst arr = [1, 2] as const;\n",
         0);
    no_as_cast_fires_on_complex_cast: "no-as-cast.md" =>
        ("a.ts", "const xs = data as ReadonlyArray<string>;\n", 1);
    no_type_assertion_fires_in_typescript: "no-type-assertion.md" =>
        ("a.ts", "const x = <number>raw;\n", 1);
    // TSX file: angle brackets are JSX, not type assertions.
    no_type_assertion_does_not_run_on_tsx: "no-type-assertion.md" =>
        ("a.tsx", "const el = <div>hi</div>;\n", 0);
    no_non_null_assertion_fires_on_member_access: "no-non-null-assertion.md" =>
        ("a.ts", "const v = obj!.prop;\n", 1);
    no_non_null_assertion_does_not_fire_on_logical_not: "no-non-null-assertion.md" =>
        ("a.ts", "if (!ready) return;\nconst x = !flag;\n", 0);
    no_empty_object_type_fires_on_type_alias: "no-empty-object-type.md" =>
        ("a.ts", "type Bag = {};\n", 1);
    no_empty_object_type_fires_on_param_annotation: "no-empty-object-type.md" =>
        ("a.ts", "function f(opts: {}) {}\n", 1);
    no_empty_object_type_does_not_fire_on_populated_type: "no-empty-object-type.md" =>
        ("a.ts", "type Bag = { id: string };\n", 0);
    no_empty_object_type_does_not_fire_on_empty_interface_decl: "no-empty-object-type.md" =>
        ("a.ts", "interface Foo {}\n", 0);
    no_branded_types_fires_on_intersection_brand: "no-branded-types.md" =>
        ("a.ts",
         "type UserId = string & { readonly __brand: unique symbol };\n",
         1);
    no_branded_types_fires_on_unique_symbol_const: "no-branded-types.md" =>
        ("a.ts", "const FOO: unique symbol = Symbol(\"foo\");\n", 1);
    no_branded_types_does_not_fire_on_plain_types: "no-branded-types.md" =>
        ("a.ts", "type UserId = { kind: \"user\"; id: string };\n", 0);
}
