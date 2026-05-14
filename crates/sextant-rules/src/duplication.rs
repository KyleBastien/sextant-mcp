//! Token-duplication evaluator built on `sextant_lang::find_clones`.
//!
//! Each clone pair produces TWO findings — one anchored at each occurrence,
//! each pointing at the other. This is deliberate: in `grade_diff` the
//! finding whose anchor is inside a changed line set is the one that
//! survives the diff filter, so a developer only sees the side of the
//! clone they're touching.

use std::path::PathBuf;

use sextant_config::DuplicationRuleConfig;
use sextant_core::{CorpusEvaluator, EvalContext, Evaluator, Finding, Rule, SourceFile};
use sextant_lang::{
    find_clones, find_cross_file_clones, parse, ClonePair, CrossFileClonePair, Language, ParsedFile,
};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;

pub struct DuplicationRule {
    rule: Rule,
    min_tokens: usize,
    cross_file_min_tokens: usize,
}

impl DuplicationRule {
    pub fn from_parsed(parsed: ParsedRule, cfg: &DuplicationRuleConfig) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
            min_tokens: cfg.min_tokens as usize,
            cross_file_min_tokens: cfg.cross_file_min_tokens as usize,
        }
    }
}

impl Evaluator for DuplicationRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        let Some(hint) = file.language_hint() else {
            return Vec::new();
        };
        let Some(lang) = Language::from_hint(hint) else {
            return Vec::new();
        };
        let parsed = match parse(file.contents.clone(), lang) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let clones = find_clones(&parsed, self.min_tokens);
        let path = file.relative_to(ctx.repo_root);
        let mut out = Vec::with_capacity(clones.len() * 2);
        for c in clones {
            push_pair(&self.rule, &path, &c, &mut out);
        }
        out
    }
}

impl CorpusEvaluator for DuplicationRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_corpus(&self, files: &[SourceFile], ctx: &EvalContext<'_>) -> Vec<Finding> {
        let owned = parse_corpus(files);
        let refs: Vec<(PathBuf, &ParsedFile)> =
            owned.iter().map(|(p, pf)| (p.clone(), pf)).collect();
        let pairs = find_cross_file_clones(&refs, self.cross_file_min_tokens);
        let mut out = Vec::with_capacity(pairs.len() * 2);
        for c in pairs {
            push_cross_pair(&self.rule, ctx.repo_root, &c, &mut out);
        }
        out
    }
}

fn parse_corpus(files: &[SourceFile]) -> Vec<(PathBuf, ParsedFile)> {
    let mut out = Vec::new();
    for file in files {
        let Some(hint) = file.language_hint() else {
            continue;
        };
        let Some(lang) = Language::from_hint(hint) else {
            continue;
        };
        if let Ok(parsed) = parse(file.contents.clone(), lang) {
            out.push((file.path.clone(), parsed));
        }
    }
    out
}

fn push_cross_pair(
    rule: &Rule,
    repo_root: &std::path::Path,
    c: &CrossFileClonePair,
    out: &mut Vec<Finding>,
) {
    let rel_a = c
        .file_a
        .strip_prefix(repo_root)
        .unwrap_or(&c.file_a)
        .to_path_buf();
    let rel_b = c
        .file_b
        .strip_prefix(repo_root)
        .unwrap_or(&c.file_b)
        .to_path_buf();
    let msg_a = format!(
        "Duplicate of {} lines {}-{} ({} tokens). Extract a helper.",
        rel_b.display(),
        c.b.start_line,
        c.b.end_line,
        c.token_count
    );
    let msg_b = format!(
        "Duplicate of {} lines {}-{} ({} tokens). Extract a helper.",
        rel_a.display(),
        c.a.start_line,
        c.a.end_line,
        c.token_count
    );
    out.push(
        Finding::new(&rule.id, rule.severity, rel_a, msg_a).spanning(c.a.start_line, c.a.end_line),
    );
    out.push(
        Finding::new(&rule.id, rule.severity, rel_b, msg_b).spanning(c.b.start_line, c.b.end_line),
    );
}

fn push_pair(rule: &Rule, path: &std::path::Path, c: &ClonePair, out: &mut Vec<Finding>) {
    let token_count = c.token_count;
    let msg_a = format!(
        "Duplicate of lines {}-{} ({} tokens). Extract a helper.",
        c.b.start_line, c.b.end_line, token_count
    );
    let msg_b = format!(
        "Duplicate of lines {}-{} ({} tokens). Extract a helper.",
        c.a.start_line, c.a.end_line, token_count
    );
    out.push(
        Finding::new(&rule.id, rule.severity, path.to_path_buf(), msg_a)
            .spanning(c.a.start_line, c.a.end_line),
    );
    out.push(
        Finding::new(&rule.id, rule.severity, path.to_path_buf(), msg_b)
            .spanning(c.b.start_line, c.b.end_line),
    );
}

#[cfg(test)]
#[path = "duplication_tests.rs"]
mod tests;
