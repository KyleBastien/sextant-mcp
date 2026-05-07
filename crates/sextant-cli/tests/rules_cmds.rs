use std::process::Command;

use assert_cmd::prelude::*;
use predicates::str;
use tempfile::tempdir;

#[test]
fn rules_list_shows_builtin_and_source() {
    let dir = tempdir().unwrap();
    let out = Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "list"])
        .current_dir(dir.path())
        .output()
        .expect("running sextant");
    assert!(out.status.success());
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("builtin.size.file-length"), "got:\n{s}");
    assert!(s.contains("builtin.size.fn-length"), "got:\n{s}");
    assert!(s.contains("builtin.size.param-count"), "got:\n{s}");
    assert!(
        s.contains("\tbuiltin\t"),
        "expected source column; got:\n{s}"
    );
}

#[test]
fn rules_explain_prints_markdown_body() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "explain", "builtin.size.fn-length"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(str::contains("# Function length"))
        .stdout(str::contains("`[size]`"));
}

#[test]
fn rules_explain_unknown_id_errors() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "explain", "no.such.rule"])
        .current_dir(dir.path())
        .assert()
        .code(2)
        .stderr(str::contains("no rule with id"));
}

#[test]
fn rules_check_validates_rule_md() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("my-rule.md");
    std::fs::write(
        &path,
        r#"---
id: project.no-todo
name: "No TODO comments"
description: "Don't ship TODOs."
severity: warn
category: style
evaluator:
  type: regex
  pattern: "TODO"
---

Detail body.
"#,
    )
    .unwrap();

    Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "check", path.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(str::contains("OK: project.no-todo"))
        .stdout(str::contains("evaluator=regex"));
}

#[test]
fn rules_check_rejects_invalid_frontmatter() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.md");
    std::fs::write(
        &path,
        r#"---
id: a
# missing required fields
---
"#,
    )
    .unwrap();

    Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "check", path.to_str().unwrap()])
        .current_dir(dir.path())
        .assert()
        .code(2)
        .stderr(str::contains("frontmatter"));
}

/// End-to-end: a repo-local regex rule fires on its target pattern and
/// shows up in `rules list`. This is the core M3 promise: write a rule
/// without writing Rust.
#[test]
fn repo_regex_rule_fires_via_grade() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let rules_dir = root.join(".sextant").join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    std::fs::write(
        rules_dir.join("no-todo.md"),
        r#"---
id: project.no-todo
name: "No TODO comments"
description: "Avoid shipping TODO markers."
severity: error
category: style
evaluator:
  type: regex
  pattern: "TODO"
---
"#,
    )
    .unwrap();

    std::fs::write(root.join("ok.rs"), "fn ok() {}\n").unwrap();
    std::fs::write(root.join("bad.rs"), "// TODO: real fix\nfn bad() {}\n").unwrap();

    let out = Command::cargo_bin("sextant")
        .unwrap()
        .args(["grade", "--format", "json", "--fail-on", "never"])
        .current_dir(root)
        .output()
        .expect("running sextant");
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = body.get("findings").and_then(|f| f.as_array()).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.get("rule_id").and_then(|v| v.as_str()) == Some("project.no-todo")),
        "expected project.no-todo finding, got:\n{body:#?}"
    );

    let listed = Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "list"])
        .current_dir(root)
        .output()
        .unwrap();
    let s = String::from_utf8(listed.stdout).unwrap();
    assert!(s.contains("project.no-todo"), "got:\n{s}");
    assert!(
        s.contains("\trepo\t"),
        "expected repo source column; got:\n{s}"
    );
}
