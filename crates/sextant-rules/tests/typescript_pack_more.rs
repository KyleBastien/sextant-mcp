//! Continuation of `tests/typescript_pack.rs`. Split across two files so
//! neither one trips the `tokens_dup` rule on the macro-table boilerplate.

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
    let (query, capture, message, not_under) = match &parsed.evaluator {
        EvaluatorSpec::Ast {
            query,
            capture,
            message,
            not_under,
        } => (
            query.clone(),
            capture.clone(),
            message.clone(),
            not_under.clone(),
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
        },
    )
    .unwrap_or_else(|e| panic!("building {filename}: {e}"))
}

fn check(rule_file: &str, file_name: &str, body: &str, expected: usize) {
    let rule = load_rule(rule_file);
    let file = SourceFile::new(file_name, body);
    let root = std::env::current_dir().unwrap();
    let actual = rule
        .evaluate_file(&file, &EvalContext { repo_root: &root })
        .len();
    assert_eq!(
        actual, expected,
        "{rule_file} on {file_name}: expected {expected} findings, got {actual}\nbody:\n{body}",
    );
}

macro_rules! more_tests {
    ($($name:ident : $rule:expr => ($file:expr, $body:expr, $expected:expr));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                check($rule, $file, $body, $expected);
            }
        )+
    };
}

more_tests! {
    no_ts_ignore_fires_on_each_directive: "no-ts-ignore.md" =>
        ("a.ts",
         "// @ts-ignore\nconst a = 1;\n// @ts-expect-error\nconst b = 2;\n// @ts-nocheck\nconst c = 3;\n",
         3);
    no_ts_ignore_does_not_fire_on_normal_comments: "no-ts-ignore.md" =>
        ("a.ts", "// just a comment\n/* block comment */\nconst x = 1;\n", 0);
    no_var_fires: "no-var.md" =>
        ("a.ts", "var x = 1;\n", 1);
    no_var_does_not_fire_on_const_or_let: "no-var.md" =>
        ("a.ts", "const a = 1;\nlet b = 2;\n", 0);
    no_function_type_fires_in_type_position: "no-function-type.md" =>
        ("a.ts", "const cb: Function = () => {};\n", 1);
    // `Function` as an identifier in value position: not a type_identifier.
    no_function_type_does_not_fire_on_value_named_function: "no-function-type.md" =>
        ("a.ts", "function Function() {}\n", 0);
    no_empty_interface_fires: "no-empty-interface.md" =>
        ("a.ts", "interface Foo {}\n", 1);
    no_empty_interface_does_not_fire_on_populated_interface: "no-empty-interface.md" =>
        ("a.ts", "interface Bar { id: string }\n", 0);
    no_eval_fires: "no-eval.md" =>
        ("a.ts", "eval(\"1+1\");\n", 1);
    no_eval_does_not_fire_on_method_named_eval: "no-eval.md" =>
        ("a.ts", "obj.eval(\"x\");\n", 0);
    prefer_inferred_types_fires_on_redundant_string_annotation: "prefer-inferred-types.md" =>
        ("a.ts", "const greeting: string = \"hello\";\n", 1);
    prefer_inferred_types_fires_on_each_primitive_kind: "prefer-inferred-types.md" =>
        ("a.ts",
         "const a: string = \"hi\";\nlet b: number = 1;\nconst c: boolean = true;\nconst d: boolean = false;\n",
         4);
    prefer_inferred_types_does_not_fire_on_non_literal_initializer: "prefer-inferred-types.md" =>
        ("a.ts", "const x: string = makeIt();\n", 0);
    prefer_inferred_types_does_not_fire_on_non_predefined_type: "prefer-inferred-types.md" =>
        ("a.ts", "const x: SpecialType = \"hi\";\n", 0);
    prefer_inferred_types_does_not_fire_when_no_annotation: "prefer-inferred-types.md" =>
        ("a.ts", "const x = \"hello\";\nconst n = 5;\n", 0);
}
