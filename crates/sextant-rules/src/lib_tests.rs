use super::*;
use sextant_core::EvalContext;

fn loaded_default_set(dir: &std::path::Path) -> RuleSet {
    RuleSet::load(dir, &Config::default()).unwrap()
}

fn file_rule_ids(set: &RuleSet) -> Vec<String> {
    set.evaluators()
        .iter()
        .map(|e| e.rule().id.clone())
        .collect()
}

fn corpus_rule_ids(set: &RuleSet) -> Vec<String> {
    set.corpus_evaluators()
        .iter()
        .map(|e| e.rule().id.clone())
        .collect()
}

fn dup_set_at(dir: &std::path::Path, cross_file_min_tokens: u32) -> RuleSet {
    let cfg = Config {
        duplication: sextant_config::DuplicationRuleConfig {
            min_tokens: 100,
            cross_file_min_tokens,
        },
        ..Default::default()
    };
    RuleSet::load(dir, &cfg).unwrap()
}

const DUP_BODY: &str = "fn f() { let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; }\n";

#[test]
fn load_picks_up_built_ins_with_default_config() {
    let dir = tempfile::tempdir().unwrap();
    let ids = file_rule_ids(&loaded_default_set(dir.path()));
    assert!(ids.contains(&"builtin.size.file-length".to_string()));
    assert!(ids.contains(&"builtin.tests.pub-fn-untested".to_string()));
}

#[test]
fn load_with_no_judge_drops_llm_rules() {
    let dir = tempfile::tempdir().unwrap();
    let rules_dir = dir.path().join(".sextant").join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    std::fs::write(
        rules_dir.join("llm.md"),
        r#"---
id: repo.llm.demo
name: "LLM demo"
description: "x"
severity: warn
category: style
languages: [rust]
evaluator:
  type: llm
---
"#,
    )
    .unwrap();
    let set = RuleSet::load_with(dir.path(), &Config::default(), None).unwrap();
    assert!(!file_rule_ids(&set).contains(&"repo.llm.demo".to_string()));
}

#[test]
fn corpus_evaluators_includes_tokens_dup() {
    let dir = tempfile::tempdir().unwrap();
    let ids = corpus_rule_ids(&loaded_default_set(dir.path()));
    assert!(ids.contains(&"builtin.duplication.tokens".to_string()));
}

struct DupFixture {
    _dir: tempfile::TempDir,
    set: RuleSet,
    files: Vec<SourceFile>,
    root: std::path::PathBuf,
}

fn dup_fixture(cross_file_min_tokens: u32) -> DupFixture {
    let dir = tempfile::tempdir().unwrap();
    let set = dup_set_at(dir.path(), cross_file_min_tokens);
    let root = dir.path().to_path_buf();
    let files = vec![
        SourceFile::new(root.join("a.rs"), DUP_BODY),
        SourceFile::new(root.join("b.rs"), DUP_BODY),
    ];
    DupFixture {
        _dir: dir,
        set,
        files,
        root,
    }
}

#[test]
fn ruleset_runs_corpus_evaluators_across_files() {
    let fx = dup_fixture(15);
    let ctx = EvalContext {
        repo_root: &fx.root,
    };
    let findings = fx.set.grade_files(&fx.files, &ctx);
    let dup: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == "builtin.duplication.tokens")
        .collect();
    assert_eq!(dup.len(), 2, "expected dual-anchor findings: {dup:?}");
    let paths: std::collections::HashSet<_> = dup.iter().map(|f| f.path.clone()).collect();
    assert_eq!(paths.len(), 2, "findings must anchor at distinct files");
}

#[test]
fn grade_per_file_skips_corpus_evaluators() {
    let fx = dup_fixture(15);
    let ctx = EvalContext {
        repo_root: &fx.root,
    };
    let findings = fx.set.grade_per_file(&fx.files, &ctx);
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id != "builtin.duplication.tokens"),
        "grade_per_file must not run corpus pass: {:?}",
        findings
    );
}

#[test]
fn grade_corpus_runs_only_corpus_evaluators() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        size: sextant_config::SizeRuleConfig {
            file_length_warn: 1,
            file_length_error: 2,
            ..Default::default()
        },
        duplication: sextant_config::DuplicationRuleConfig {
            min_tokens: 100,
            cross_file_min_tokens: 15,
        },
        ..Default::default()
    };
    let set = RuleSet::load(dir.path(), &cfg).unwrap();
    let a = SourceFile::new(dir.path().join("a.rs"), DUP_BODY);
    let b = SourceFile::new(dir.path().join("b.rs"), DUP_BODY);
    let ctx = EvalContext {
        repo_root: dir.path(),
    };
    let findings = set.grade_corpus(&[a, b], &ctx);
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id == "builtin.duplication.tokens"),
        "grade_corpus must skip per-file rules: {:?}",
        findings
    );
    assert_eq!(findings.len(), 2);
}

#[test]
fn grade_files_runs_built_in_size_rule() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        size: sextant_config::SizeRuleConfig {
            file_length_warn: 5,
            file_length_error: 10,
            ..Default::default()
        },
        ..Default::default()
    };
    let set = RuleSet::load(dir.path(), &cfg).unwrap();
    let file = SourceFile::new(dir.path().join("a.rs"), "x\n".repeat(20));
    let ctx = EvalContext {
        repo_root: dir.path(),
    };
    let findings = set.grade_files(&[file], &ctx);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "builtin.size.file-length"));
}
