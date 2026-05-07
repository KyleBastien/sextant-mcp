use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use sextant_config::Config;
use sextant_core::{Category, RuleSource};
use sextant_rules::{parse_rule_md, EvaluatorSpec, RuleSet};

pub fn list() -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd)?;
    let ruleset = RuleSet::load(&cwd, &config).context("loading rules")?;
    for ev in ruleset.evaluators() {
        let r = ev.rule();
        println!(
            "{}\t{}\t{}\t{}\t{}",
            r.id,
            r.severity.as_str(),
            format!("{:?}", r.scope).to_lowercase(),
            r.source.as_str(),
            r.name
        );
    }
    Ok(ExitCode::from(0))
}

pub fn explain(id: &str) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let config = Config::from_repo_root(&cwd)?;
    let ruleset = RuleSet::load(&cwd, &config).context("loading rules")?;
    let Some(rule) = ruleset.evaluators().iter().find(|e| e.rule().id == id) else {
        eprintln!("error: no rule with id `{id}`");
        return Ok(ExitCode::from(2));
    };
    let r = rule.rule();
    println!("# {} ({})", r.name, r.id);
    println!();
    println!(
        "**severity:** {}  •  **category:** {}  •  **source:** {}",
        r.severity.as_str(),
        category_str(&r.category),
        r.source.as_str()
    );
    println!();
    if !r.description.is_empty() {
        println!("{}", r.description);
        println!();
    }
    if !r.body.is_empty() {
        println!("{}", r.body);
    }
    Ok(ExitCode::from(0))
}

pub fn check(path: &Path) -> Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match parse_rule_md(&text, RuleSource::Repo, Some(path.to_path_buf())) {
        Ok(rule) => {
            println!("OK: {} ({})", rule.id, rule.name);
            println!(
                "  severity={} category={} scope={:?}",
                rule.severity.as_str(),
                category_str(&rule.category),
                rule.scope,
            );
            match &rule.evaluator {
                EvaluatorSpec::Builtin { name } => {
                    println!("  evaluator=builtin name={name}");
                }
                EvaluatorSpec::Regex { pattern, .. } => {
                    println!("  evaluator=regex pattern={pattern:?}");
                }
            }
            Ok(ExitCode::from(0))
        }
        Err(err) => {
            eprintln!("error: {err}");
            Ok(ExitCode::from(2))
        }
    }
}

fn category_str(c: &Category) -> String {
    use Category::*;
    match c {
        Complexity => "complexity".into(),
        Size => "size".into(),
        Duplication => "duplication".into(),
        Tests => "tests".into(),
        Reliability => "reliability".into(),
        Style => "style".into(),
        Security => "security".into(),
        Docs => "docs".into(),
        Custom(s) => format!("custom:{s}"),
    }
}
