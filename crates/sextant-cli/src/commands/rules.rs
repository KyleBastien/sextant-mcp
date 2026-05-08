use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result};
use sextant_core::RuleSource;
use sextant_engine::{explain_rule, list_rules};
use sextant_rules::{parse_rule_md, EvaluatorSpec};

pub(crate) fn list() -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let rules = list_rules(&cwd).context("loading rules")?;
    for r in rules {
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

pub(crate) fn explain(id: &str) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let Some(r) = explain_rule(&cwd, id).context("loading rules")? else {
        eprintln!("error: no rule with id `{id}`");
        return Ok(ExitCode::from(2));
    };
    println!("# {} ({})", r.name, r.id);
    println!();
    println!(
        "**severity:** {}  •  **category:** {}  •  **source:** {}",
        r.severity.as_str(),
        r.category.name(),
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

pub(crate) fn check(path: &Path) -> Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match parse_rule_md(&text, RuleSource::Repo, Some(path.to_path_buf())) {
        Ok(rule) => {
            println!("OK: {} ({})", rule.id, rule.name);
            println!(
                "  severity={} category={} scope={:?}",
                rule.severity.as_str(),
                rule.category.name(),
                rule.scope,
            );
            match &rule.evaluator {
                EvaluatorSpec::Builtin { name } => {
                    println!("  evaluator=builtin name={name}");
                }
                EvaluatorSpec::Regex { pattern, .. } => {
                    println!("  evaluator=regex pattern={pattern:?}");
                }
                EvaluatorSpec::Llm { model, .. } => {
                    let m = model.as_deref().unwrap_or("<from config>");
                    println!("  evaluator=llm model={m}");
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
