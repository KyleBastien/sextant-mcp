//! Built-in rule evaluators + rule discovery for Sextant.
//!
//! In M3, all rules — built-in and repo-local — flow through one loader.
//! Built-ins are markdown files embedded via `rust-embed` whose
//! `evaluator: { type: builtin, name: ... }` frontmatter dispatches to a
//! Rust evaluator. Repo-local rules under `<root>/.sextant/rules/**/*.md`
//! use `evaluator: { type: regex, ... }` and require no Rust code at all.

mod ast_rule;
mod complexity;
mod duplication;
pub mod fetcher;
mod file_length;
mod fn_length;
mod llm_rule;
pub mod loader;
pub mod lock;
mod merge;
mod param_count;
mod pub_fn_test;
mod regex_rule;

use std::path::Path;
use std::sync::Arc;

use sextant_config::Config;
use sextant_core::{EvalContext, Evaluator, Finding, SourceFile};
use sextant_judge::Judge;
use thiserror::Error;

pub use ast_rule::{AstBuildError, AstRule, AstRuleSpec};
pub use complexity::ComplexityRule;
pub use duplication::DuplicationRule;
pub use file_length::FileLengthRule;
pub use fn_length::FnLengthRule;
pub use llm_rule::{LlmRule, LlmRuleSpec};
pub use loader::{
    builtin_rules, parse_rule_md, repo_rules, vendor_rules, EvaluatorSpec, ParsedRule,
};
pub use param_count::ParamCountRule;
pub use pub_fn_test::PubFnUntestedRule;
pub use regex_rule::{RegexBuildError, RegexRule};

#[derive(Debug, Error)]
pub enum RuleSetError {
    #[error(transparent)]
    Loader(#[from] loader::LoaderError),
    #[error("regex evaluator for rule `{rule}`: {source}")]
    Regex {
        rule: String,
        #[source]
        source: RegexBuildError,
    },
    #[error("ast evaluator: {0}")]
    Ast(#[from] AstBuildError),
    #[error("unknown built-in evaluator name `{name}` in rule `{rule}`")]
    UnknownBuiltin { rule: String, name: String },
}

/// A bundle of evaluators discovered from built-ins + the repo's
/// `.sextant/rules/` directory.
pub struct RuleSet {
    evaluators: Vec<Arc<dyn Evaluator>>,
}

impl RuleSet {
    /// Load built-ins + repo-local rules and resolve overrides. LLM rules
    /// are dropped when no judge is configured (use [`load_with`] to wire
    /// a judge in).
    pub fn load(repo_root: &Path, config: &Config) -> Result<Self, RuleSetError> {
        Self::load_with(repo_root, config, None)
    }

    /// Like [`load`], but with an optional `Judge`. When `judge` is
    /// `None`, LLM-evaluated rules are skipped at load time (a single
    /// log line per dropped rule). When `Some`, LLM rules are wired up
    /// and runtime errors degrade to `info` findings.
    pub fn load_with(
        repo_root: &Path,
        config: &Config,
        judge: Option<Arc<Judge>>,
    ) -> Result<Self, RuleSetError> {
        let parsed = loader::merge_all(
            loader::builtin_rules()?,
            loader::vendor_rules(repo_root)?,
            loader::repo_rules(repo_root)?,
        )?;
        let mut evaluators: Vec<Arc<dyn Evaluator>> = Vec::with_capacity(parsed.len());
        for rule in parsed {
            if let Some(ev) = build_evaluator(rule, config, judge.as_ref())? {
                evaluators.push(ev);
            }
        }
        Ok(Self { evaluators })
    }

    pub fn evaluators(&self) -> &[Arc<dyn Evaluator>] {
        &self.evaluators
    }

    pub fn grade_files(&self, files: &[SourceFile], ctx: &EvalContext<'_>) -> Vec<Finding> {
        let mut out = Vec::new();
        for file in files {
            for ev in &self.evaluators {
                if !rule_applies_to_file(ev.as_ref(), file) {
                    continue;
                }
                out.extend(ev.evaluate_file(file, ctx));
            }
        }
        out
    }
}

fn build_evaluator(
    rule: ParsedRule,
    config: &Config,
    judge: Option<&Arc<Judge>>,
) -> Result<Option<Arc<dyn Evaluator>>, RuleSetError> {
    match rule.evaluator.clone() {
        EvaluatorSpec::Builtin { name } => build_builtin(&name, rule, config).map(Some),
        EvaluatorSpec::Regex {
            pattern,
            exclude_paths,
        } => build_regex(rule, &pattern, &exclude_paths).map(Some),
        EvaluatorSpec::Llm {
            model,
            max_tokens,
            temperature,
            exclude_paths,
            ..
        } => {
            let Some(judge) = judge else {
                tracing::info!(rule = %rule.id, "skipping LLM rule: no judge configured");
                return Ok(None);
            };
            let spec = LlmRuleSpec {
                model: model.unwrap_or_else(|| config.judge.model.clone()),
                max_tokens: max_tokens.unwrap_or(config.judge.max_tokens),
                temperature: temperature.unwrap_or(config.judge.temperature),
                exclude_paths,
            };
            build_llm(rule, Arc::clone(judge), spec).map(Some)
        }
        EvaluatorSpec::Ast {
            query,
            capture,
            message,
            not_under,
            exclude_paths,
        } => {
            let spec = AstRuleSpec {
                query: &query,
                capture: capture.as_deref(),
                message: message.as_deref(),
                not_under: &not_under,
                exclude_paths: &exclude_paths,
            };
            Ok(Some(Arc::new(AstRule::from_parsed(rule, spec)?)))
        }
    }
}

fn build_builtin(
    name: &str,
    rule: ParsedRule,
    config: &Config,
) -> Result<Arc<dyn Evaluator>, RuleSetError> {
    match name {
        "file_length" => Ok(Arc::new(FileLengthRule::from_parsed(rule, &config.size))),
        "fn_length" => Ok(Arc::new(FnLengthRule::from_parsed(rule, &config.size))),
        "param_count" => Ok(Arc::new(ParamCountRule::from_parsed(rule, &config.size))),
        "cyclomatic" => Ok(Arc::new(ComplexityRule::cyclomatic(
            rule,
            &config.complexity,
        ))),
        "nesting" => Ok(Arc::new(ComplexityRule::nesting(rule, &config.complexity))),
        "tokens_dup" => Ok(Arc::new(DuplicationRule::from_parsed(
            rule,
            &config.duplication,
        ))),
        "pub_fn_untested" => Ok(Arc::new(PubFnUntestedRule::from_parsed(rule))),
        other => Err(RuleSetError::UnknownBuiltin {
            rule: rule.id,
            name: other.to_string(),
        }),
    }
}

fn build_regex(
    rule: ParsedRule,
    pattern: &str,
    exclude_paths: &[String],
) -> Result<Arc<dyn Evaluator>, RuleSetError> {
    let id = rule.id.clone();
    let built = RegexRule::from_parsed(rule, pattern, exclude_paths)
        .map_err(|source| RuleSetError::Regex { rule: id, source })?;
    Ok(Arc::new(built))
}

fn build_llm(
    rule: ParsedRule,
    judge: Arc<Judge>,
    spec: LlmRuleSpec,
) -> Result<Arc<dyn Evaluator>, RuleSetError> {
    let id = rule.id.clone();
    let built = LlmRule::from_parsed(rule, judge, spec)
        .map_err(|source| RuleSetError::Regex { rule: id, source })?;
    Ok(Arc::new(built))
}

fn rule_applies_to_file(ev: &dyn Evaluator, file: &SourceFile) -> bool {
    let rule = ev.rule();
    if !rule.enabled {
        return false;
    }
    if rule.languages.is_empty() {
        return true;
    }
    match file.language_hint() {
        Some(lang) => rule.languages.iter().any(|l| l == lang),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sextant_core::EvalContext;

    #[test]
    fn load_picks_up_built_ins_with_default_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::default();
        let set = RuleSet::load(dir.path(), &cfg).unwrap();
        let ids: Vec<_> = set
            .evaluators()
            .iter()
            .map(|e| e.rule().id.clone())
            .collect();
        assert!(ids.contains(&"builtin.size.file-length".to_string()));
        assert!(ids.contains(&"builtin.tests.pub-fn-untested".to_string()));
    }

    #[test]
    fn load_with_no_judge_drops_llm_rules() {
        // Repo with one LLM rule. With `judge = None`, it must not surface
        // among the loaded evaluators.
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
        let cfg = Config::default();
        let set = RuleSet::load_with(dir.path(), &cfg, None).unwrap();
        let ids: Vec<_> = set
            .evaluators()
            .iter()
            .map(|e| e.rule().id.clone())
            .collect();
        assert!(!ids.contains(&"repo.llm.demo".to_string()));
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
}
