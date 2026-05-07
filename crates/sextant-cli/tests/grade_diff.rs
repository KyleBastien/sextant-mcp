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

/// Diff mode: initialize a repo with a clean baseline, then add a long
/// function in a follow-up commit. The grader should fire on the new
/// function but not on a pre-existing long function.
#[test]
fn diff_mode_only_grades_changed_lines() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_repo(root);

    // .sextant/config.toml — tight thresholds.
    let cfg_dir = root.join(".sextant");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        "[size]\nfile_length_warn = 10000\nfile_length_error = 20000\n\
         fn_length_warn = 4\nfn_length_error = 8\n\
         param_count_warn = 100\nparam_count_error = 200\n",
    )
    .unwrap();

    // Baseline: a long function already exists.
    let baseline = "fn pre_existing() {\n    1;\n    2;\n    3;\n    4;\n    5;\n    6;\n    7;\n    8;\n    9;\n    10;\n}\n";
    std::fs::write(root.join("lib.rs"), baseline).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "baseline"]);

    // Now add a NEW long function (≥8 lines). The pre-existing one should
    // be ignored because diff mode only counts touched lines.
    let with_new = format!("{baseline}\nfn newly_added() {{\n    1;\n    2;\n    3;\n    4;\n    5;\n    6;\n    7;\n    8;\n    9;\n    10;\n}}\n");
    std::fs::write(root.join("lib.rs"), with_new).unwrap();

    let output = Command::cargo_bin("sextant")
        .unwrap()
        .args([
            "grade",
            "--diff",
            "--base",
            "HEAD",
            "--format",
            "json",
            "--fail-on",
            "never",
        ])
        .current_dir(root)
        .output()
        .expect("running sextant");

    assert!(
        output.status.success(),
        "stderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    let findings = parsed.get("findings").and_then(|f| f.as_array()).unwrap();
    assert!(
        findings.iter().any(|f| f
            .get("message")
            .and_then(|m| m.as_str())
            .map(|s| s.contains("newly_added"))
            .unwrap_or(false)),
        "expected `newly_added` to be flagged; findings: {findings:?}",
    );
    assert!(
        !findings.iter().any(|f| f
            .get("message")
            .and_then(|m| m.as_str())
            .map(|s| s.contains("pre_existing"))
            .unwrap_or(false)),
        "did NOT expect `pre_existing` to be flagged in diff mode; findings: {findings:?}",
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
