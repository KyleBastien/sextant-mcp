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

fn vendor(id: &str, pack: &str) -> ParsedRule {
    rule(
        &format!(
            r#"---
id: {id}
name: V
description: x
severity: error
category: style
evaluator: {{ type: regex, pattern: "x" }}
---
"#
        ),
        RuleSource::Vendor(pack.into()),
    )
}

#[test]
fn merge_all_vendor_overrides_builtin() {
    let b = builtin("conflict.id");
    let v = vendor("conflict.id", "ts");
    let merged = merge_all(vec![b], vec![v], vec![]).unwrap();
    assert_eq!(merged.len(), 1);
    assert!(matches!(merged[0].source, RuleSource::Vendor(_)));
}

#[test]
fn merge_all_repo_shadowing_vendor_is_an_error() {
    let v = vendor("vendor.ts.no-any", "typescript");
    let r = rule(
        r#"---
id: vendor.ts.no-any
name: shadow
description: x
severity: warn
category: style
evaluator: { type: regex, pattern: "y" }
---
"#,
        RuleSource::Repo,
    );
    let err = merge_all(vec![], vec![v], vec![r]).unwrap_err();
    assert!(matches!(err, LoaderError::ShadowsVendor { .. }));
}

#[test]
fn merge_all_repo_overrides_cannot_disable_vendor_rules() {
    let v = vendor("vendor.ts.no-any", "typescript");
    let r = rule(
        r#"---
id: repo.disabler
name: disabler
description: x
severity: warn
category: style
overrides: ["vendor.ts.no-any"]
evaluator: { type: regex, pattern: "y" }
---
"#,
        RuleSource::Repo,
    );
    let merged = merge_all(vec![], vec![v], vec![r]).unwrap();
    assert!(merged.iter().any(|m| m.id == "vendor.ts.no-any"));
}

#[test]
fn merge_all_vendor_overrides_can_disable_other_rules() {
    let target = builtin("builtin.target");
    let v_pack = rule(
        r#"---
id: vendor.disabler
name: disabler
description: x
severity: error
category: style
overrides: ["builtin.target"]
evaluator: { type: regex, pattern: "y" }
---
"#,
        RuleSource::Vendor("ts".into()),
    );
    let merged = merge_all(vec![target], vec![v_pack], vec![]).unwrap();
    let ids: Vec<_> = merged.iter().map(|r| r.id.as_str()).collect();
    assert!(!ids.contains(&"builtin.target"));
    assert!(ids.contains(&"vendor.disabler"));
}

#[test]
fn merge_all_vendor_rule_with_enabled_false_still_loads() {
    // Vendor rules deliberately bypass the `enabled: false` field — the
    // pack author shouldn't ship a disabled rule, and the lock-integrity
    // check ensures the bytes match what they did ship. Repo-level rules
    // still respect `enabled: false`.
    let v = rule(
        r#"---
id: vendor.x
name: X
description: x
severity: error
category: style
enabled: false
evaluator: { type: regex, pattern: "x" }
---
"#,
        RuleSource::Vendor("ts".into()),
    );
    let merged = merge_all(vec![], vec![v], vec![]).unwrap();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].id, "vendor.x");
}

#[test]
fn vendor_rules_returns_empty_when_lock_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(vendor_rules(dir.path()).unwrap().is_empty());
}

fn install_test_pack(root: &std::path::Path, pack_name: &str, rule_md: &str) {
    use crate::lock::{hash_bytes, LockFile, LockedPack};
    use std::collections::BTreeMap;
    let pack_dir = root
        .join(".sextant")
        .join("rules")
        .join("vendor")
        .join(pack_name);
    std::fs::create_dir_all(pack_dir.join("rules")).unwrap();
    let pack_toml = format!("name = \"{pack_name}\"\n");
    std::fs::write(pack_dir.join("pack.toml"), &pack_toml).unwrap();
    std::fs::write(pack_dir.join("rules/demo.md"), rule_md).unwrap();
    let mut files = BTreeMap::new();
    files.insert("pack.toml".into(), hash_bytes(pack_toml.as_bytes()));
    files.insert("rules/demo.md".into(), hash_bytes(rule_md.as_bytes()));
    let mut lock = LockFile::empty();
    lock.upsert(LockedPack {
        name: pack_name.into(),
        source: format!("file:./{pack_name}"),
        reference: "v0".into(),
        revision: "deadbeef".into(),
        subdir: String::new(),
        fetched_at: String::new(),
        files,
    });
    lock.write(root).unwrap();
}

const DEMO_RULE_MD: &str = r#"---
id: vendor.ts.demo
name: Demo
description: x
severity: error
category: style
evaluator: { type: regex, pattern: "x" }
---
"#;

#[test]
fn vendor_rules_loads_pack_files_against_lock() {
    let dir = tempfile::tempdir().unwrap();
    install_test_pack(dir.path(), "ts", DEMO_RULE_MD);
    let rules = vendor_rules(dir.path()).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, "vendor.ts.demo");
    assert_eq!(rules[0].source, RuleSource::Vendor("ts".into()));
}

#[test]
fn vendor_rules_fails_when_pack_dir_tampered() {
    let dir = tempfile::tempdir().unwrap();
    install_test_pack(dir.path(), "ts", DEMO_RULE_MD);
    let demo = dir.path().join(".sextant/rules/vendor/ts/rules/demo.md");
    std::fs::write(&demo, "tampered").unwrap();
    let err = vendor_rules(dir.path()).unwrap_err();
    assert!(matches!(err, LoaderError::Lock(_)));
}
