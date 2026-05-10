use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};

use crate::loader::ParsedRule;
use crate::patch::replace_line_diff;

pub struct RegexRule {
    rule: Rule,
    re: Regex,
    exclude: GlobSet,
    /// Replacement template using regex-crate substitution syntax (e.g.
    /// `$1`, `$name`). When set, every match yields a unified-diff patch
    /// that rewrites that match in place on the affected line.
    replacement: Option<String>,
}

impl RegexRule {
    pub fn from_parsed(
        parsed: ParsedRule,
        pattern: &str,
        exclude_paths: &[String],
        replacement: Option<&str>,
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
        Ok(Self {
            rule,
            re,
            exclude,
            replacement: replacement.map(str::to_string),
        })
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
            let cursor = LineCursor {
                contents: &file.contents,
                rel: &rel,
                line,
                line_no: i as u32 + 1,
            };
            self.collect_line_findings(cursor, &mut out);
        }
        out
    }
}

/// One physical line's worth of context for the regex pass: the
/// surrounding file (for patch construction), the path (for diff
/// headers), the line itself (the match haystack), and the 1-indexed
/// line number.
struct LineCursor<'a> {
    contents: &'a str,
    rel: &'a std::path::Path,
    line: &'a str,
    line_no: u32,
}

impl RegexRule {
    /// Push every match on `cursor.line` into `out` as a `Finding`,
    /// attaching a patch when the rule has a `replacement` template.
    fn collect_line_findings(&self, cursor: LineCursor<'_>, out: &mut Vec<Finding>) {
        for m in self.re.find_iter(cursor.line) {
            let snippet = m.as_str();
            let msg = format!("{}: matched `{}`", self.rule.name, snippet);
            let mut finding = Finding::new(
                &self.rule.id,
                self.rule.severity,
                cursor.rel.to_path_buf(),
                msg,
            )
            .at_line(cursor.line_no);
            if let Some(patch) = self.patch_for(cursor.rel, cursor.contents, cursor.line_no) {
                finding = finding.with_patch(patch);
            }
            out.push(finding);
        }
    }

    fn patch_for(&self, rel: &std::path::Path, contents: &str, line_no: u32) -> Option<String> {
        let template = self.replacement.as_deref()?;
        line_replacement_patch(&self.re, rel, contents, line_no, template)
    }
}

/// Run the regex replacement on the given line and emit a unified diff
/// against `path`. Returns `None` when the resulting line is identical
/// (no-op replacement) so we don't ship empty hunks.
fn line_replacement_patch(
    re: &Regex,
    path: &std::path::Path,
    contents: &str,
    line_no: u32,
    template: &str,
) -> Option<String> {
    let original_line = contents.split_inclusive('\n').nth((line_no - 1) as usize)?;
    let stripped = original_line.strip_suffix('\n').unwrap_or(original_line);
    let replaced = re.replace_all(stripped, template);
    let new_line: &str = replaced.as_ref();
    if new_line == stripped {
        return None;
    }
    replace_line_diff(path, contents, line_no, new_line)
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
        build_with(pat, exclude, None)
    }

    fn build_with(pat: &str, exclude: &[&str], replacement: Option<&str>) -> RegexRule {
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
        RegexRule::from_parsed(parsed, pat, &exclude, replacement).unwrap()
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
    fn replacement_attaches_unified_diff_patch() {
        let rule = build_with(r"\bvar\b", &[], Some("let"));
        let file = SourceFile::new("a.ts", "var x = 1;\nlet y = 2;\n");
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(&file, &ctx(&root));
        assert_eq!(f.len(), 1);
        let patch = f[0].patch.as_deref().expect("patch attached");
        assert!(patch.contains("@@ -1,1 +1,1 @@"), "patch was: {patch}");
        assert!(patch.contains("-var x = 1;"), "patch was: {patch}");
        assert!(patch.contains("+let x = 1;"), "patch was: {patch}");
    }

    #[test]
    fn no_replacement_means_no_patch() {
        let rule = build(r"TODO", &[]);
        let file = SourceFile::new("a.rs", "// TODO\n");
        let root = std::env::current_dir().unwrap();
        let f = rule.evaluate_file(&file, &ctx(&root));
        assert!(f[0].patch.is_none());
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
