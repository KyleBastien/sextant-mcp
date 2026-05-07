//! Built-in rule evaluators + rule discovery for Sextant.
//!
//! In M3, all rules — built-in and repo-local — flow through one loader.
//! Built-ins are markdown files embedded via `rust-embed` whose
//! `evaluator: { type: builtin, name: ... }` frontmatter dispatches to a
//! Rust evaluator. Repo-local rules under `<root>/.sextant/rules/**/*.md`
//! use `evaluator: { type: regex, ... }` and require no Rust code at all.

mod complexity;
mod duplication;
mod file_length;
mod fn_length;
pub mod loader;
mod param_count;
mod pub_fn_test;
mod regex_rule;

use std::path::Path;
use std::sync::Arc;

use sextant_config::Config;
use sextant_core::{EvalContext, Evaluator, Finding, SourceFile};
use thiserror::Error;

pub use complexity::ComplexityRule;
pub use duplication::DuplicationRule;
pub use file_length::FileLengthRule;
pub use fn_length::FnLengthRule;
pub use loader::{builtin_rules, parse_rule_md, repo_rules, EvaluatorSpec, ParsedRule};
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
    #[error("unknown built-in evaluator name `{name}` in rule `{rule}`")]
    UnknownBuiltin { rule: String, name: String },
}

/// A bundle of evaluators discovered from built-ins + the repo's
/// `.sextant/rules/` directory.
pub struct RuleSet {
    evaluators: Vec<Arc<dyn Evaluator>>,
}

impl RuleSet {
    /// Load built-ins + repo-local rules and resolve overrides.
    pub fn load(repo_root: &Path, config: &Config) -> Result<Self, RuleSetError> {
        let parsed = loader::merge(loader::builtin_rules()?, loader::repo_rules(repo_root)?);
        let mut evaluators: Vec<Arc<dyn Evaluator>> = Vec::with_capacity(parsed.len());
        for rule in parsed {
            evaluators.push(build_evaluator(rule, config)?);
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

fn build_evaluator(rule: ParsedRule, config: &Config) -> Result<Arc<dyn Evaluator>, RuleSetError> {
    match rule.evaluator.clone() {
        EvaluatorSpec::Builtin { name } => build_builtin(&name, rule, config),
        EvaluatorSpec::Regex {
            pattern,
            exclude_paths,
        } => build_regex(rule, &pattern, &exclude_paths),
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
