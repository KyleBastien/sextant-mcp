use super::*;
use crate::loader::parse_rule_md;
use sextant_core::RuleSource;
use std::path::Path;

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

fn build_rule(min_tokens: u32, cross_file_min_tokens: u32) -> DuplicationRule {
    DuplicationRule::from_parsed(
        parsed_for_test(),
        &DuplicationRuleConfig {
            min_tokens,
            cross_file_min_tokens,
        },
    )
}

fn ctx_at_cwd() -> (std::path::PathBuf, EvalContext<'static>) {
    let root = std::env::current_dir().unwrap();
    let leaked: &'static Path = Box::leak(root.clone().into_boxed_path());
    (root, EvalContext { repo_root: leaked })
}

const DUP_BODY: &str = r#"
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

#[test]
fn flags_two_findings_per_clone() {
    let rule = build_rule(20, 9999);
    let file = SourceFile::new("a.rs", DUP_BODY);
    let (_root, ctx) = ctx_at_cwd();
    let f = rule.evaluate_file(&file, &ctx);
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(f[0].message.contains("lines"));
    assert!(f[1].message.contains("lines"));
}

#[test]
fn quiet_when_no_duplication() {
    let rule = build_rule(100, 9999);
    let file = SourceFile::new("a.rs", "fn ok() { let x = 1; }\n");
    let (_root, ctx) = ctx_at_cwd();
    assert!(rule.evaluate_file(&file, &ctx).is_empty());
}

#[test]
fn skips_unsupported_languages() {
    let rule = build_rule(5, 9999);
    let file = SourceFile::new("a.txt", "anything\nat all\n");
    let (_root, ctx) = ctx_at_cwd();
    assert!(rule.evaluate_file(&file, &ctx).is_empty());
}

fn rust_file(name: &str, src: &str) -> SourceFile {
    SourceFile::new(name, src)
}

#[test]
fn corpus_flags_two_findings_per_cross_file_clone() {
    let rule = build_rule(9999, 15);
    let body = "fn f() { let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; }\n";
    let files = vec![rust_file("a.rs", body), rust_file("b.rs", body)];
    let (_root, ctx) = ctx_at_cwd();
    let f = rule.evaluate_corpus(&files, &ctx);
    assert_eq!(f.len(), 2, "{f:?}");
    let paths: Vec<_> = f.iter().map(|x| x.path.clone()).collect();
    assert!(paths.contains(&std::path::PathBuf::from("a.rs")));
    assert!(paths.contains(&std::path::PathBuf::from("b.rs")));
    let msg_for_a = f
        .iter()
        .find(|x| x.path.as_path() == std::path::Path::new("a.rs"))
        .unwrap();
    assert!(
        msg_for_a.message.contains("b.rs"),
        "message must name other file: {}",
        msg_for_a.message
    );
}

#[test]
fn corpus_quiet_when_no_cross_file_duplication() {
    let rule = build_rule(9999, 15);
    let files = vec![
        rust_file("a.rs", "fn alpha() { let x = 1; }\n"),
        rust_file("b.rs", "struct Foo { bar: u32, baz: u64 }\n"),
    ];
    let (_root, ctx) = ctx_at_cwd();
    assert!(rule.evaluate_corpus(&files, &ctx).is_empty());
}

#[test]
fn corpus_respects_separate_threshold() {
    let rule = build_rule(20, 9999);
    let body = "fn f() { let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; }\n";
    let files = vec![rust_file("a.rs", body), rust_file("b.rs", body)];
    let (_root, ctx) = ctx_at_cwd();
    assert!(
        rule.evaluate_corpus(&files, &ctx).is_empty(),
        "cross_file_min_tokens at 9999 must suppress matches even when min_tokens is low"
    );
}

#[test]
fn corpus_skips_unsupported_languages() {
    let rule = build_rule(9999, 5);
    let files = vec![
        SourceFile::new("a.txt", "anything\nat all\n"),
        SourceFile::new("b.txt", "anything\nat all\n"),
    ];
    let (_root, ctx) = ctx_at_cwd();
    assert!(rule.evaluate_corpus(&files, &ctx).is_empty());
}

#[test]
fn corpus_does_not_double_count_in_file_clones() {
    let rule = build_rule(9999, 10);
    let body = r#"
fn f() {
    let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; let f = 6;
    let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; let f = 6;
}
"#;
    let files = vec![rust_file("only.rs", body)];
    let (_root, ctx) = ctx_at_cwd();
    assert!(
        rule.evaluate_corpus(&files, &ctx).is_empty(),
        "in-file pass owns same-file duplication"
    );
}
