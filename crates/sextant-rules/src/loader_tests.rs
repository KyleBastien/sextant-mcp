use super::*;

fn rule(text: &str, source: RuleSource) -> ParsedRule {
    parse_rule_md(text, source, None).unwrap()
}

fn builtin(id: &str) -> ParsedRule {
    rule(
        &format!(
            r#"---
id: {id}
name: "x"
description: "x"
severity: warn
category: style
evaluator: {{ type: regex, pattern: "x" }}
---
"#
        ),
        RuleSource::Builtin,
    )
}

#[test]
fn parses_minimal_rule() {
    let text = r#"---
id: test.foo
name: "Foo"
description: "test"
severity: warn
category: style
evaluator:
  type: regex
  pattern: 'TODO'
---

body
"#;
    let r = rule(text, RuleSource::Repo);
    assert_eq!(r.id, "test.foo");
    assert_eq!(r.severity, Severity::Warn);
    assert_eq!(r.scope, Scope::File);
    assert!(matches!(r.evaluator, EvaluatorSpec::Regex { .. }));
    assert!(r.body.starts_with("body"));
}

#[test]
fn missing_frontmatter_errors() {
    let err = parse_rule_md("no frontmatter here\n", RuleSource::Repo, None).unwrap_err();
    assert!(format!("{err}").contains("frontmatter"));
}

#[test]
fn builtins_load() {
    let rules = builtin_rules().unwrap();
    let ids: Vec<_> = rules.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"builtin.size.file-length"));
    assert!(ids.contains(&"builtin.size.fn-length"));
    assert!(ids.contains(&"builtin.size.param-count"));
}

#[test]
fn repo_rule_replaces_builtin() {
    let original = rule(
        r#"---
id: builtin.size.file-length
name: "Original"
description: "x"
severity: warn
category: size
evaluator: { type: builtin, name: file_length }
---
"#,
        RuleSource::Builtin,
    );
    let override_ = rule(
        r#"---
id: builtin.size.file-length
name: "Override"
description: "x"
severity: error
category: size
evaluator: { type: regex, pattern: "x" }
---
"#,
        RuleSource::Repo,
    );
    let merged = merge(vec![original], vec![override_]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "Override");
    assert_eq!(merged[0].severity, Severity::Error);
}

#[test]
fn overrides_list_disables_rule() {
    let a = builtin("a");
    let b = rule(
        r#"---
id: b
name: B
description: x
severity: warn
category: style
overrides: ["a"]
evaluator: { type: regex, pattern: "y" }
---
"#,
        RuleSource::Repo,
    );
    let merged = merge(vec![a], vec![b]);
    let ids: Vec<_> = merged.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["b"]);
}

#[test]
fn repo_rules_walks_dot_sextant_directory() {
    let dir = tempfile::tempdir().unwrap();
    let rules_dir = dir.path().join(".sextant").join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    std::fs::write(
        rules_dir.join("custom.md"),
        r#"---
id: repo.custom
name: "Custom"
description: "x"
severity: warn
category: style
evaluator: { type: regex, pattern: "TODO" }
---
"#,
    )
    .unwrap();
    let rules = repo_rules(dir.path()).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, "repo.custom");
    assert_eq!(rules[0].source, RuleSource::Repo);
}

#[test]
fn repo_rules_returns_empty_when_directory_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(repo_rules(dir.path()).unwrap().is_empty());
}

#[test]
fn disabled_rule_is_dropped() {
    let a = rule(
        r#"---
id: a
name: A
description: x
severity: warn
category: style
enabled: false
evaluator: { type: regex, pattern: "x" }
---
"#,
        RuleSource::Builtin,
    );
    assert!(merge(vec![a], vec![]).is_empty());
}
