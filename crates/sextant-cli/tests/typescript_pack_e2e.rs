//! End-to-end test for the TypeScript vendor pack: install via `file:`
//! source, grade a TS file containing one violation per rule, then
//! exercise every documented bypass attempt to confirm none of them work.

use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use serde_json::Value;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf()
}

fn pack_path() -> PathBuf {
    workspace_root().join("packs").join("typescript")
}

fn write_minimal_repo(root: &Path) {
    let cfg_dir = root.join(".sextant");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        "[verdict]\nmax_errors = 0\nmax_warns = 0\nmax_info = 0\n",
    )
    .unwrap();
}

fn run_sextant(root: &Path, args: &[&str]) -> std::process::Output {
    Command::cargo_bin("sextant")
        .unwrap()
        .args(args)
        .current_dir(root)
        .output()
        .expect("running sextant")
}

fn install_pack(root: &Path) {
    let spec = format!("file:{}", pack_path().display());
    let out = run_sextant(root, &["rules", "add", &spec]);
    assert!(
        out.status.success(),
        "rules add failed:\n stdout: {}\n stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(root.join(".sextant/rules.lock").exists());
    assert!(root
        .join(".sextant/rules/vendor/typescript/pack.toml")
        .exists());
}

const BAD_TS: &str = "\
const a: any = 1;
const b: unknown = 2;
const c: object = {};
const d = e as string;
const f = <number>g;
const h = obj!.prop;
// @ts-ignore
const i = 1;
var j = 2;
const k: Function = () => {};
interface Empty {}
eval(\"x\");
const l: string = \"hi\";
";

const REQUIRED_RULES: &[&str] = &[
    "vendor.typescript.no-any",
    "vendor.typescript.no-unknown",
    "vendor.typescript.no-object-type",
    "vendor.typescript.no-as-cast",
    "vendor.typescript.no-type-assertion",
    "vendor.typescript.no-non-null-assertion",
    "vendor.typescript.no-ts-ignore",
    "vendor.typescript.no-var",
    "vendor.typescript.no-function-type",
    "vendor.typescript.no-empty-interface",
    "vendor.typescript.no-eval",
    "vendor.typescript.prefer-inferred-types",
];

fn grade_findings(root: &Path) -> Vec<Value> {
    let out = run_sextant(
        root,
        &[
            "grade",
            "--format",
            "json",
            "--fail-on",
            "never",
            "--no-llm",
        ],
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "expected JSON, got:\n{stdout}\nstderr:\n{}\nerror: {e}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    body.get("findings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

#[test]
fn install_then_grade_fires_every_pack_rule() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_minimal_repo(root);
    install_pack(root);
    std::fs::write(root.join("bad.ts"), BAD_TS).unwrap();

    let findings = grade_findings(root);
    let fired: std::collections::HashSet<String> = findings
        .iter()
        .filter_map(|f| {
            f.get("rule_id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .collect();
    for id in REQUIRED_RULES {
        assert!(
            fired.contains(*id),
            "expected finding for `{id}` but only got: {:?}",
            fired
        );
    }
}

#[test]
fn rules_list_shows_vendor_pack_rules() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_minimal_repo(root);
    install_pack(root);

    let out = run_sextant(root, &["rules", "list"]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("vendor.typescript.no-any"),
        "missing no-any in rules list:\n{text}"
    );
    assert!(
        text.contains("\tvendor:typescript\t"),
        "expected vendor source column; got:\n{text}"
    );
}

fn grade_json_args() -> [&'static str; 6] {
    [
        "grade",
        "--format",
        "json",
        "--fail-on",
        "never",
        "--no-llm",
    ]
}

fn install_then_break_no_any<F: FnOnce(&Path)>(mutate_no_any: F) -> std::process::Output {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_minimal_repo(root);
    install_pack(root);
    std::fs::write(root.join("ok.ts"), "const x = 1;\n").unwrap();
    let path = root.join(".sextant/rules/vendor/typescript/rules/no-any.md");
    mutate_no_any(&path);
    let out = run_sextant(root, &grade_json_args());
    drop(dir);
    out
}

#[test]
fn tampering_with_vendor_file_fails_grade() {
    let out = install_then_break_no_any(|path| {
        let mut text = std::fs::read_to_string(path).unwrap();
        text.push_str("\n# tampered\n");
        std::fs::write(path, text).unwrap();
    });
    assert!(
        !out.status.success(),
        "expected non-zero exit after tampering"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.to_lowercase().contains("hash") || stderr.contains("modified"),
        "stderr did not mention tamper detection:\n{stderr}"
    );
}

#[test]
fn deleting_vendor_file_fails_grade() {
    let out = install_then_break_no_any(|path| {
        std::fs::remove_file(path).unwrap();
    });
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.to_lowercase().contains("missing"),
        "stderr did not mention missing-file detection:\n{stderr}"
    );
}

#[test]
fn repo_overrides_targeting_vendor_rule_are_ignored() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_minimal_repo(root);
    install_pack(root);
    std::fs::write(root.join("bad.ts"), "const a: any = 1;\n").unwrap();

    let rules_dir = root.join(".sextant/rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    std::fs::write(
        rules_dir.join("disabler.md"),
        r#"---
id: repo.disabler
name: "Try to disable no-any"
description: "should have no effect"
severity: info
category: style
overrides: ["vendor.typescript.no-any"]
evaluator:
  type: regex
  pattern: "z"
---
"#,
    )
    .unwrap();

    let findings = grade_findings(root);
    let fired: Vec<_> = findings
        .iter()
        .filter_map(|f| f.get("rule_id").and_then(|v| v.as_str()))
        .filter(|id| *id == "vendor.typescript.no-any")
        .collect();
    assert!(
        !fired.is_empty(),
        "vendor rule was suppressed by repo overrides — should be impossible"
    );
}

#[test]
fn repo_rule_with_same_id_as_vendor_is_a_load_error() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write_minimal_repo(root);
    install_pack(root);

    let rules_dir = root.join(".sextant/rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    std::fs::write(
        rules_dir.join("shadow.md"),
        r#"---
id: vendor.typescript.no-any
name: "Shadow"
description: "should fail to load"
severity: warn
category: style
evaluator:
  type: regex
  pattern: "z"
---
"#,
    )
    .unwrap();

    let out = run_sextant(root, &["rules", "list"]);
    assert!(!out.status.success(), "expected load failure");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("shadows vendor pack"),
        "stderr did not surface shadow error:\n{stderr}"
    );
}
