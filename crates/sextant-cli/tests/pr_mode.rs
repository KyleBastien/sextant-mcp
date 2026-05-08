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

/// Run `sextant grade --pr` against the repo at `root` and return the
/// path to the rendered output. Extra args (e.g. `--report-json`) get
/// appended verbatim. Centralising the boilerplate here keeps the
/// test bodies focused on the actual assertions.
fn run_pr_grade(
    root: &Path,
    format: &str,
    output_name: &str,
    extra: &[&str],
) -> std::path::PathBuf {
    let out = root.join(output_name);
    let mut args: Vec<String> = vec![
        "grade".into(),
        "--pr".into(),
        "--base".into(),
        "HEAD~1".into(),
        "--format".into(),
        format.into(),
        "--output".into(),
        out.to_str().unwrap().into(),
        "--fail-on".into(),
        "never".into(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    Command::cargo_bin("sextant")
        .unwrap()
        .args(&args)
        .current_dir(root)
        .assert()
        .success();
    out
}

#[test]
fn pr_mode_emits_markdown_review_with_marker() {
    let dir = make_pr_repo();
    let out = run_pr_grade(dir.path(), "markdown", "review.md", &[]);
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.contains("# Sextant review"), "got:\n{body}");
    assert!(body.contains("<!-- sextant:review -->"), "got:\n{body}");
    assert!(body.contains("New issues"), "got:\n{body}");
    assert!(body.contains("builtin.size.file-length"), "got:\n{body}");
}

#[test]
fn pr_mode_report_json_writes_pr_report_shape() {
    let dir = make_pr_repo();
    let report = dir.path().join("report.json");
    run_pr_grade(
        dir.path(),
        "markdown",
        "review.md",
        &["--report-json", report.to_str().unwrap()],
    );
    let body = std::fs::read_to_string(&report).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v.get("delta").is_some(), "got: {body}");
    assert!(v.get("verdict").is_some(), "got: {body}");
    assert!(
        v["delta"]["new_counts"].get("error").is_some(),
        "got: {body}"
    );
}

#[test]
fn pr_mode_review_json_emits_github_review_payload() {
    let dir = make_pr_repo();
    let out = run_pr_grade(dir.path(), "review-json", "review.json", &[]);
    let body = std::fs::read_to_string(&out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(
        v["event"] == "COMMENT" || v["event"] == "REQUEST_CHANGES",
        "got event: {}",
        v["event"]
    );
    assert!(v["comments"].is_array(), "got: {body}");
    assert!(
        v["body"]
            .as_str()
            .unwrap()
            .contains("<!-- sextant:review -->"),
        "got body:\n{}",
        v["body"]
    );
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
