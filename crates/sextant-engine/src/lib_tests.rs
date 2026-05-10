use super::*;
use sextant_core::Verdict;

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, contents).unwrap();
}

fn git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("running git");
    assert!(
        status.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn init_repo(root: &Path) {
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    git(root, &["config", "commit.gpgsign", "false"]);
}

#[test]
fn grade_files_returns_findings() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ".sextant/config.toml",
        "[size]\nfile_length_warn = 10\nfile_length_error = 20\n",
    );
    write(root, "long.rs", &"x\n".repeat(25));

    let report = grade(
        root,
        GradeMode::Files {
            paths: vec![root.to_path_buf()],
        },
    )
    .unwrap();
    assert!(report
        .findings
        .iter()
        .any(|f| f.rule_id == "builtin.size.file-length"));
}

#[test]
fn grade_file_buffer_uses_overlay_text() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ".sextant/config.toml",
        "[size]\nfile_length_warn = 10\nfile_length_error = 20\n",
    );
    write(root, "long.rs", "x\n");

    let overlay = SourceFile::new(root.join("long.rs"), "x\n".repeat(25));
    let report = grade_file_buffer(root, overlay, GradeOptions::default()).unwrap();
    assert!(report
        .findings
        .iter()
        .any(|f| f.rule_id == "builtin.size.file-length"));
}

#[test]
fn grade_file_buffer_respects_path_excludes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ".sextant/config.toml",
        "[paths]\nexclude = [\"vendor/**\"]\n[size]\nfile_length_warn = 5\nfile_length_error = 10\n",
    );
    let overlay = SourceFile::new(root.join("vendor/long.rs"), "x\n".repeat(25));
    let report = grade_file_buffer(root, overlay, GradeOptions::default()).unwrap();
    assert!(report.findings.is_empty());
}

#[test]
fn list_rules_returns_builtins() {
    let dir = tempfile::tempdir().unwrap();
    let rules = list_rules(dir.path()).unwrap();
    let ids: Vec<_> = rules.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"builtin.size.file-length"));
    assert!(ids.contains(&"builtin.size.fn-length"));
    assert!(ids.contains(&"builtin.size.param-count"));
}

#[test]
fn explain_rule_returns_body() {
    let dir = tempfile::tempdir().unwrap();
    let r = explain_rule(dir.path(), "builtin.size.fn-length")
        .unwrap()
        .expect("rule found");
    assert!(r.body.contains("Function length"));
}

#[test]
fn explain_unknown_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    assert!(explain_rule(dir.path(), "nope").unwrap().is_none());
}

#[test]
fn load_config_reads_repo_local_overrides() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        ".sextant/config.toml",
        "[size]\nfile_length_warn = 7\n",
    );
    let cfg = load_config(dir.path()).unwrap();
    assert_eq!(cfg.size.file_length_warn, 7);
}

#[test]
fn grade_pr_returns_only_new_findings_in_delta() {
    let dir = tempfile::tempdir().unwrap();
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

    let pr = grade_pr(
        root,
        DiffOptions {
            base: Some("HEAD~1".into()),
            head: None,
            working_tree: false,
        },
        PrOptions::default(),
    )
    .expect("grade_pr");

    assert!(pr
        .delta
        .new_findings
        .iter()
        .any(|f| f.rule_id == "builtin.size.file-length"));
    assert!(matches!(
        pr.verdict,
        Verdict::Approve | Verdict::RequestChanges { .. }
    ));
    assert!(pr.delta.base_sha.is_some());
}

/// Regression: a PR that introduces a new repo rule must not see
/// pre-existing violations in unchanged files reported as "fixed by
/// this PR". The baseline grade is whole-tree (so a new rule fires on
/// every existing match) while head is diff-only (so the same files
/// produce zero findings if they're not touched). Without scoping the
/// baseline to diff paths, those zero-vs-nonzero comparisons
/// incorrectly mark the findings as fixed.
#[test]
fn grade_pr_does_not_count_untouched_files_as_fixed_when_a_rule_is_added() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    write(root, ".sextant/config.toml", "[verdict]\nmax_errors = 0\n");
    // Pre-existing file containing the marker — base never had a rule
    // for it, so the baseline (under HEAD's ruleset) will produce a
    // finding here that *looks* fixable but isn't actually touched.
    write(root, "untouched.rs", "fn x() { /* MARKER42 */ }\n");
    write(root, "ok.rs", "fn ok() {}\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);

    // PR introduces a new repo rule + an unrelated edit. The rule's own
    // pattern is unique (MARKER42), so the rule definition file doesn't
    // self-match. ok.rs is the only path with content the PR actually
    // changed.
    write(
        root,
        ".sextant/rules/marker.md",
        "---\nid: project.marker\nname: \"Marker\"\ndescription: \"banned token\"\nseverity: warn\ncategory: style\nevaluator: { type: regex, pattern: \"\\\\bMARKER42\\\\b\" }\n---\n",
    );
    write(root, "ok.rs", "fn ok() { /* unrelated */ }\n");
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "add marker rule and edit ok.rs"]);

    let pr = grade_pr(
        root,
        DiffOptions {
            base: Some("HEAD~1".into()),
            head: None,
            working_tree: false,
        },
        PrOptions::default(),
    )
    .expect("grade_pr");

    let fixed_paths: Vec<_> = pr
        .delta
        .fixed_findings
        .iter()
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect();
    assert!(
        !fixed_paths.iter().any(|p| p.contains("untouched.rs")),
        "untouched.rs should not appear as `fixed`; got: {fixed_paths:?}",
    );
    assert!(
        pr.delta.new_findings.is_empty(),
        "no new findings expected on the touched path; got: {:?}",
        pr.delta.new_findings,
    );
}

#[test]
fn grade_pr_baseline_cache_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    write(
        root,
        ".sextant/config.toml",
        "[size]\nfile_length_warn = 5\nfile_length_error = 10\n",
    );
    write(root, "long.rs", &"x\n".repeat(20));
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "init"]);
    write(root, "long.rs", &"y\n".repeat(20));
    git(root, &["commit", "-aq", "-m", "edit"]);

    let cache_dir = root.join(".cache");
    let opts = || PrOptions {
        baseline_cache: Some(cache_dir.clone()),
        grade: GradeOptions::default(),
    };
    let first = grade_pr(
        root,
        DiffOptions {
            base: Some("HEAD~1".into()),
            ..Default::default()
        },
        opts(),
    )
    .unwrap();
    let cached: Vec<_> = std::fs::read_dir(&cache_dir).unwrap().collect();
    assert_eq!(cached.len(), 1);

    let second = grade_pr(
        root,
        DiffOptions {
            base: Some("HEAD~1".into()),
            ..Default::default()
        },
        opts(),
    )
    .unwrap();
    assert_eq!(first.delta, second.delta);
}

#[test]
fn no_llm_skips_llm_rules_at_load_time() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ".sextant/config.toml",
        r#"
[judge]
enabled = true
provider = "anthropic"
api_key_env = "DEFINITELY_NOT_SET_SEXTANT_TEST"
"#,
    );
    write(
        root,
        ".sextant/rules/example.md",
        r#"---
id: repo.llm.example
name: "LLM example"
description: "x"
severity: warn
category: style
languages: [rust]
evaluator:
  type: llm
---

Review {{code}}.
"#,
    );
    write(root, "a.rs", "fn x() {}\n");

    let report = grade_with(
        root,
        GradeMode::Files {
            paths: vec![root.to_path_buf()],
        },
        GradeOptions { no_llm: true },
    )
    .unwrap();
    assert!(report
        .findings
        .iter()
        .all(|f| f.rule_id != "repo.llm.example"));
}
