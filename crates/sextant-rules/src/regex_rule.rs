use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};

use crate::loader::ParsedRule;

pub struct RegexRule {
    rule: Rule,
    re: Regex,
    exclude: GlobSet,
}

impl RegexRule {
    pub fn from_parsed(
        parsed: ParsedRule,
        pattern: &str,
        exclude_paths: &[String],
    ) -> RegexBuildResult {
        let re = match Regex::new(pattern) {
            Ok(re) => re,
            Err(err) => {
                return Err(RegexBuildError::Pattern {
                    pattern: pattern.to_string(),
                    source: err,
                });
            }
        };
        let mut builder = GlobSetBuilder::new();
        for p in exclude_paths {
            let glob = Glob::new(p).map_err(|source| RegexBuildError::Glob {
                pattern: p.clone(),
                source,
            })?;
            builder.add(glob);
        }
        let exclude = builder.build().map_err(|source| RegexBuildError::Glob {
            pattern: "<set>".into(),
            source,
        })?;

        let rule = Rule {
            id: parsed.id,
            name: parsed.name,
            description: parsed.description,
            body: parsed.body,
            severity: parsed.severity,
            category: parsed.category,
            scope: parsed.scope,
            languages: parsed.languages,
            enabled: parsed.enabled,
            tags: parsed.tags,
            source: parsed.source,
        };
        Ok(Self { rule, re, exclude })
    }
}

pub type RegexBuildResult = Result<RegexRule, RegexBuildError>;

#[derive(Debug, thiserror::Error)]
pub enum RegexBuildError {
    #[error("invalid pattern `{pattern}`: {source}")]
    Pattern {
        pattern: String,
        #[source]
        source: regex::Error,
    },
    #[error("invalid glob `{pattern}`: {source}")]
    Glob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}

impl Evaluator for RegexRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        let rel = file.relative_to(ctx.repo_root);
        if self.exclude.is_match(&rel) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, line) in file.contents.lines().enumerate() {
            for m in self.re.find_iter(line) {
                let snippet = m.as_str();
                let msg = format!("{}: matched `{}`", self.rule.name, snippet);
                out.push(
                    Finding::new(&self.rule.id, self.rule.severity, rel.clone(), msg)
                        .at_line(i as u32 + 1),
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::{RuleSource, Severity};
    use std::path::Path;

    fn ctx<'a>(root: &'a Path) -> EvalContext<'a> {
        EvalContext { repo_root: root }
    }

    fn build(pat: &str, exclude: &[&str]) -> RegexRule {
        let parsed = parse_rule_md(
            r#"---
id: test.todo
name: "TODO marker"
description: "x"
severity: warn
category: style
evaluator: { type: regex, pattern: "TODO" }
---
"#,
            RuleSource::Repo,
            None,
        )
        .unwrap();
        let exclude: Vec<String> = exclude.iter().map(|s| s.to_string()).collect();
        RegexRule::from_parsed(parsed, pat, &exclude).unwrap()
    }

    #[test]
    fn fires_on_match_with_line_number() {
        let rule = build(r"TODO", &[]);
        let file = SourceFile::new("a.rs", "fn ok() {}\n// TODO: refactor\nfn x() {}\n");
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(&file, &ctx(&root));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, Some(2));
        assert_eq!(f[0].severity, Severity::Warn);
    }

    #[test]
    fn skips_excluded_paths() {
        let rule = build(r"TODO", &["**/tests/**"]);
        let mut file = SourceFile::new(
            std::env::current_dir().unwrap().join("tests/foo.rs"),
            "// TODO\n",
        );
        // adjust path so the relative form starts with `tests/`
        file.path = std::env::current_dir()
            .unwrap()
            .join("tests")
            .join("foo.rs");
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(&file, &ctx(&root));
        assert!(f.is_empty());
    }
}
