use super::test_helpers::*;
use super::*;

const DUP_BODY: &str = "fn f() { let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; }\n";

fn setup_dup_repo(threshold: u32) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        ".sextant/config.toml",
        &format!("[verdict]\nmax_errors = 0\n[duplication]\ncross_file_min_tokens = {threshold}\n"),
    );
    write(root, "a.rs", DUP_BODY);
    write(root, "b.rs", DUP_BODY);
    dir
}

fn dup_findings(report: &sextant_core::Report) -> Vec<&sextant_core::Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.rule_id == "builtin.duplication.tokens")
        .collect()
}

#[test]
fn grade_files_surfaces_cross_file_duplication() {
    let dir = setup_dup_repo(15);
    let root = dir.path();
    let report = grade(
        root,
        GradeMode::Files {
            paths: vec![root.to_path_buf()],
        },
    )
    .unwrap();
    let dup = dup_findings(&report);
    assert_eq!(dup.len(), 2, "expected dual-anchor findings: {dup:?}");
    let names: std::collections::HashSet<_> =
        dup.iter().map(|f| f.path.file_name().unwrap()).collect();
    assert_eq!(names.len(), 2, "anchors must be distinct: {names:?}");
}

#[test]
fn cross_file_threshold_default_is_loaded_from_config() {
    let dir = setup_dup_repo(9999);
    let root = dir.path();
    let report = grade(
        root,
        GradeMode::Files {
            paths: vec![root.to_path_buf()],
        },
    )
    .unwrap();
    assert!(
        dup_findings(&report).is_empty(),
        "high threshold must suppress: {:?}",
        report.findings
    );
}

#[test]
fn grade_diff_keeps_only_changed_side_of_cross_file_pair() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    write(
        root,
        ".sextant/config.toml",
        "[verdict]\nmax_errors = 0\n[duplication]\ncross_file_min_tokens = 15\n",
    );
    write(root, "a.rs", DUP_BODY);
    write(root, "b.rs", DUP_BODY);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);

    // Rewrite a.rs with token-equal content (different identifiers/literals
    // are filtered out by the token-kind hash) so the cross-file dup still
    // matches b.rs, but the changed-lines set marks every code line.
    let touched = "fn g() { let x = 9; let y = 8; let z = 7; let w = 6; let v = 5; }\n";
    write(root, "a.rs", touched);

    let report = grade(
        root,
        GradeMode::Diff(DiffOptions {
            base: Some("HEAD".into()),
            head: None,
            working_tree: true,
        }),
    )
    .unwrap();
    let dup = dup_findings(&report);
    assert_eq!(dup.len(), 1, "only touched side should survive: {dup:?}");
    assert!(
        dup[0].path.ends_with("a.rs"),
        "anchor must be at changed file: {dup:?}"
    );
}

#[test]
fn grade_pr_reports_newly_introduced_cross_file_clone() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);
    write(
        root,
        ".sextant/config.toml",
        "[verdict]\nmax_errors = 0\n[duplication]\ncross_file_min_tokens = 15\n",
    );
    write(root, "a.rs", DUP_BODY);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "base"]);

    write(root, "b.rs", DUP_BODY);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "add duplicate"]);

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

    assert!(
        pr.delta
            .new_findings
            .iter()
            .any(|f| f.rule_id == "builtin.duplication.tokens"),
        "newly added clone should appear as new: {:?}",
        pr.delta.new_findings,
    );
}
