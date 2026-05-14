//! Continuation of `tests/typescript_pack.rs`. Split across two files so
//! the per-binary test counts stay manageable.

mod common;

use common::load_rule;
use sextant_core::{EvalContext, Evaluator, SourceFile};

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
