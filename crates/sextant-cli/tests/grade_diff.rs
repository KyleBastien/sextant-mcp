use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::tempdir;

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("running git");
    assert!(status.success(), "git {args:?} failed");
}

fn init_repo(dir: &std::path::Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
}

fn long_fn(name: &str) -> String {
    let body: String = (1..=10).map(|i| format!("    {i};\n")).collect();
    format!("fn {name}() {{\n{body}}}\n")
}

fn write_config(root: &std::path::Path, body: &str) {
    let cfg_dir = root.join(".sextant");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(cfg_dir.join("config.toml"), body).unwrap();
}

fn run_diff_grade_json(root: &std::path::Path, base: &str) -> serde_json::Value {
    let out = Command::cargo_bin("sextant")
        .unwrap()
        .args([
            "grade",
            "--diff",
            "--base",
            base,
            "--format",
            "json",
            "--fail-on",
            "never",
        ])
        .current_dir(root)
        .output()
        .expect("running sextant");
    assert!(
        out.status.success(),
        "stderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout),
    );
    serde_json::from_slice(&out.stdout).expect("valid json")
}

/// Diff mode: initialize a repo with a clean baseline, then add a long
/// function in a follow-up commit. The grader should fire on the new
/// function but not on a pre-existing long function.
#[test]
fn diff_mode_only_grades_changed_lines() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    write_config(
        root,
        "[size]\nfile_length_warn = 10000\nfile_length_error = 20000\n\
         fn_length_warn = 4\nfn_length_error = 8\n\
         param_count_warn = 100\nparam_count_error = 200\n",
    );

    let baseline = long_fn("pre_existing");
    std::fs::write(root.join("lib.rs"), &baseline).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "baseline"]);

    // Add a NEW long function. The pre-existing one should be ignored.
    let with_new = format!("{baseline}\n{}", long_fn("newly_added"));
    std::fs::write(root.join("lib.rs"), with_new).unwrap();

    let report = run_diff_grade_json(root, "HEAD");
    let findings = report.get("findings").and_then(|f| f.as_array()).unwrap();
    let has = |name: &str| {
        findings.iter().any(|f| {
            f.get("message")
                .and_then(|m| m.as_str())
                .is_some_and(|s| s.contains(name))
        })
    };
    assert!(
        has("newly_added"),
        "expected `newly_added`; got: {findings:?}"
    );
    assert!(
        !has("pre_existing"),
        "did NOT expect `pre_existing`; got: {findings:?}"
    );
}

#[test]
fn diff_mode_with_no_changes_is_clean() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    std::fs::write(root.join("lib.rs"), "fn ok() {}\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "init"]);

    Command::cargo_bin("sextant")
        .unwrap()
        .args(["grade", "--diff", "--base", "HEAD", "--format", "human"])
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicates::str::contains("No findings"));
}
