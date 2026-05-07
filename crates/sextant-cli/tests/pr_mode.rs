use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::str;
use tempfile::tempdir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn make_pr_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    write(
        root,
        ".sextant/config.toml",
        "[size]\nfile_length_warn = 5\nfile_length_error = 10\n",
    );
    write(root, "ok.rs", "fn ok() {}\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);

    write(root, "long.rs", &"x\n".repeat(20));
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "introduce long file"]);
    dir
}

#[test]
fn pr_mode_emits_markdown_review_with_marker() {
    let dir = make_pr_repo();
    let root = dir.path();
    let out = root.join("review.md");

    Command::cargo_bin("sextant")
        .unwrap()
        .args([
            "grade",
            "--pr",
            "--base",
            "HEAD~1",
            "--format",
            "markdown",
            "--output",
            out.to_str().unwrap(),
            "--fail-on",
            "never",
        ])
        .current_dir(root)
        .assert()
        .success();

    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("# Sextant review"), "got:\n{body}");
    assert!(body.contains("<!-- sextant:review -->"), "got:\n{body}");
    assert!(body.contains("New issues"), "got:\n{body}");
    assert!(body.contains("builtin.size.file-length"), "got:\n{body}");
}

#[test]
fn pr_mode_report_json_writes_pr_report_shape() {
    let dir = make_pr_repo();
    let root = dir.path();
    let review = root.join("review.md");
    let report = root.join("report.json");

    Command::cargo_bin("sextant")
        .unwrap()
        .args([
            "grade",
            "--pr",
            "--base",
            "HEAD~1",
            "--format",
            "markdown",
            "--output",
            review.to_str().unwrap(),
            "--report-json",
            report.to_str().unwrap(),
            "--fail-on",
            "never",
        ])
        .current_dir(root)
        .assert()
        .success();

    let body = std::fs::read_to_string(&report).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v.get("delta").is_some(), "got: {body}");
    assert!(v.get("verdict").is_some(), "got: {body}");
    assert!(v["delta"]["new_counts"].get("error").is_some(), "got: {body}");
}

#[test]
fn pr_mode_baseline_cache_is_reused() {
    let dir = make_pr_repo();
    let root = dir.path();
    let cache = root.join(".cache");

    let run = || {
        Command::cargo_bin("sextant")
            .unwrap()
            .args([
                "grade",
                "--pr",
                "--base",
                "HEAD~1",
                "--baseline-cache",
                cache.to_str().unwrap(),
                "--format",
                "json",
                "--fail-on",
                "never",
            ])
            .current_dir(root)
            .assert()
    };

    run().success();
    let cached_files: Vec<_> = std::fs::read_dir(&cache).unwrap().collect();
    assert_eq!(cached_files.len(), 1, "expected one baseline cache entry");
    // Second invocation: cache should still have the same file.
    run().success();
    let cached_files2: Vec<_> = std::fs::read_dir(&cache).unwrap().collect();
    assert_eq!(cached_files2.len(), 1);
}

#[test]
fn sarif_format_is_valid_json_with_v210_envelope() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ".sextant/config.toml",
        "[size]\nfile_length_warn = 5\nfile_length_error = 10\n",
    );
    write(root, "long.rs", &"x\n".repeat(20));

    Command::cargo_bin("sextant")
        .unwrap()
        .args(["grade", "--format", "sarif", "--fail-on", "never"])
        .current_dir(root)
        .assert()
        .success()
        .stdout(str::contains(r#""version": "2.1.0""#))
        .stdout(str::contains(r#""ruleId": "builtin.size.file-length""#));
}
