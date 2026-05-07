use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::tempdir;

/// End-to-end: build a tiny repo with three files of varying lengths,
/// run `sextant grade --format json`, and snapshot the report shape.
#[test]
fn grade_emits_expected_findings() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // .sextant/config.toml — tight thresholds so the fixture stays small.
    let cfg_dir = root.join(".sextant");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::write(
        cfg_dir.join("config.toml"),
        "[size]\nfile_length_warn = 10\nfile_length_error = 20\n",
    )
    .unwrap();

    std::fs::write(root.join("clean.rs"), "fn ok() {}\n").unwrap();
    std::fs::write(root.join("warn.rs"), "x\n".repeat(15)).unwrap();
    std::fs::write(root.join("error.rs"), "x\n".repeat(25)).unwrap();

    let output = Command::cargo_bin("sextant")
        .unwrap()
        .args(["grade", "--format", "json", "--fail-on", "never"])
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
    insta::assert_json_snapshot!("grade_files_report", parsed);
}

#[test]
fn grade_clean_repo_returns_zero() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("ok.rs"), "fn ok() {}\n").unwrap();

    Command::cargo_bin("sextant")
        .unwrap()
        .args(["grade", "--format", "human"])
        .current_dir(root)
        .assert()
        .success();
}

#[test]
fn excluded_paths_produce_no_findings() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Default config — Cargo.lock is in the default exclude list.
    // Use a file long enough to trip the default file-length error threshold
    // if it weren't excluded.
    std::fs::write(root.join("Cargo.lock"), "x\n".repeat(900)).unwrap();
    std::fs::write(root.join("ok.rs"), "fn ok() {}\n").unwrap();

    Command::cargo_bin("sextant")
        .unwrap()
        .args(["grade", "--format", "human"])
        .current_dir(root)
        .assert()
        .success()
        .stdout(predicates::str::contains("No findings"));
}

#[test]
fn rules_list_prints_builtin() {
    let dir = tempdir().unwrap();
    let output = Command::cargo_bin("sextant")
        .unwrap()
        .args(["rules", "list"])
        .current_dir(dir.path())
        .output()
        .expect("running sextant");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("builtin.size.file-length"),
        "expected builtin.size.file-length in output, got:\n{stdout}"
    );
}
