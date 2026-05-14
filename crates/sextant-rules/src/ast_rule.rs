//! AST evaluator: tree-sitter-query-driven rules. Frontmatter form:
//!
//! ```yaml
//! evaluator:
//!   type: ast
//!   query: '((predefined_type) @t (#eq? @t "any"))'
//!   not_under: [catch_clause]   # optional ancestor-skip
//!   capture: t                  # optional; defaults to the first capture
//!   message: "..."              # optional override
//! ```
//!
//! The rule must declare `languages:` — the same query is compiled once per
//! listed language. AST findings are anchored to the captured node's start
//! row, then filtered through the engine's diff filter like every other rule.

use std::collections::HashMap;

use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};
use sextant_lang::{parse, Language};
use tree_sitter::{Node, Query, QueryCursor};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;

pub struct AstRule {
    rule: Rule,
    queries: HashMap<Language, Query>,
    capture: Option<String>,
    message: Option<String>,
    not_under: Vec<String>,
}

impl AstRule {
    pub fn from_parsed(parsed: ParsedRule, spec: AstRuleSpec<'_>) -> Result<Self, AstBuildError> {
        if parsed.languages.is_empty() {
            return Err(AstBuildError::MissingLanguages {
                rule: parsed.id.clone(),
            });
        }
        let mut queries = HashMap::new();
        for hint in &parsed.languages {
            let language =
                Language::from_hint(hint).ok_or_else(|| AstBuildError::UnknownLanguage {
                    rule: parsed.id.clone(),
                    language: hint.clone(),
                })?;
            let query = Query::new(&language.ts_language(), spec.query).map_err(|err| {
                AstBuildError::Query {
                    rule: parsed.id.clone(),
                    language: hint.clone(),
                    message: err.to_string(),
                }
            })?;
            queries.insert(language, query);
        }
        let rule = rule_from_parsed(parsed);
        Ok(Self {
            rule,
            queries,
            capture: spec.capture.map(str::to_string),
            message: spec.message.map(str::to_string),
            not_under: spec.not_under.to_vec(),
        })
    }
}

pub struct AstRuleSpec<'a> {
    pub query: &'a str,
    pub capture: Option<&'a str>,
    pub message: Option<&'a str>,
    pub not_under: &'a [String],
}

#[derive(Debug, thiserror::Error)]
pub enum AstBuildError {
    #[error("ast rule `{rule}` must declare at least one language")]
    MissingLanguages { rule: String },
    #[error("ast rule `{rule}` references unknown language `{language}`")]
    UnknownLanguage { rule: String, language: String },
    #[error("ast rule `{rule}` query (lang {language}) failed to compile: {message}")]
    Query {
        rule: String,
        language: String,
        message: String,
    },
}

impl Evaluator for AstRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        let rel = file.relative_to(ctx.repo_root);
        let Some(language) = file.language_hint().and_then(Language::from_hint) else {
            return Vec::new();
        };
        let Some(query) = self.queries.get(&language) else {
            return Vec::new();
        };
        let Ok(parsed) = parse(file.contents.clone(), language) else {
            return Vec::new();
        };
        let mut cursor = QueryCursor::new();
        let source_bytes = parsed.source.as_bytes();
        let capture_idx = self.resolve_capture_index(query);
        let mut out = Vec::new();
        for m in cursor.matches(query, parsed.tree.root_node(), source_bytes) {
            let Some(capture) = pick_capture(&m, capture_idx) else {
                continue;
            };
            if has_ancestor_in(capture.node, &self.not_under) {
                continue;
            }
            let snippet = capture
                .node
                .utf8_text(source_bytes)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .trim();
            let msg = self
                .message
                .clone()
                .unwrap_or_else(|| format!("{}: matched `{}`", self.rule.name, snippet));
            let start = capture.node.start_position().row as u32 + 1;
            let end = capture.node.end_position().row as u32 + 1;
            let finding = Finding::new(&self.rule.id, self.rule.severity, rel.clone(), msg);
            out.push(if start == end {
                finding.at_line(start)
            } else {
                finding.spanning(start, end)
            });
        }
        out
    }
}

impl AstRule {
    fn resolve_capture_index(&self, query: &Query) -> Option<u32> {
        let name = self.capture.as_deref()?;
        query.capture_index_for_name(name)
    }
}

fn pick_capture<'tree>(
    m: &tree_sitter::QueryMatch<'_, 'tree>,
    capture_idx: Option<u32>,
) -> Option<tree_sitter::QueryCapture<'tree>> {
    if let Some(idx) = capture_idx {
        m.captures.iter().find(|c| c.index == idx).copied()
    } else {
        m.captures.first().copied()
    }
}

fn has_ancestor_in(node: Node<'_>, kinds: &[String]) -> bool {
    if kinds.is_empty() {
        return false;
    }
    let mut cur = node.parent();
    while let Some(n) = cur {
        if kinds.iter().any(|k| k == n.kind()) {
            return true;
        }
        cur = n.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::parse_rule_md;
    use sextant_core::RuleSource;
    use std::path::Path;

    fn ctx<'a>(root: &'a Path) -> EvalContext<'a> {
        EvalContext { repo_root: root }
    }

    fn build(query: &str, languages: &[&str], not_under: &[&str]) -> AstRule {
        let langs = languages
            .iter()
            .map(|l| format!("\"{l}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let md = format!(
            r#"---
id: test.ast
name: "AST test"
description: "x"
severity: error
category: style
languages: [{langs}]
evaluator: {{ type: ast, query: "{}" }}
---
"#,
            query.replace('"', "\\\"")
        );
        let parsed = parse_rule_md(&md, RuleSource::Repo, None).unwrap();
        let not_under_owned: Vec<String> = not_under.iter().map(|s| s.to_string()).collect();
        AstRule::from_parsed(
            parsed,
            AstRuleSpec {
                query,
                capture: None,
                message: None,
                not_under: &not_under_owned,
            },
        )
        .unwrap()
    }

    fn run(rule: &AstRule, name: &str, body: &str) -> Vec<Finding> {
        let file = SourceFile::new(name, body);
        let root = std::env::current_dir().unwrap();
        rule.evaluate_file(&file, &ctx(&root))
    }

    const ANY_QUERY: &str = r#"((predefined_type) @t (#eq? @t "any"))"#;
    const UNKNOWN_QUERY: &str = r#"((predefined_type) @t (#eq? @t "unknown"))"#;

    #[test]
    fn fires_on_predefined_any_in_typescript() {
        let rule = build(ANY_QUERY, &["typescript", "tsx"], &[]);
        let f = run(&rule, "a.ts", "const x: any = 1;\n");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, Some(1));
    }

    #[test]
    fn does_not_fire_on_other_predefined_types() {
        let rule = build(ANY_QUERY, &["typescript"], &[]);
        assert!(run(&rule, "a.ts", "const x: number = 1;\n").is_empty());
    }

    #[test]
    fn skips_matches_under_excluded_ancestor() {
        let rule = build(UNKNOWN_QUERY, &["typescript"], &["catch_clause"]);
        let body = "try { foo(); } catch (e: unknown) { throw e; }\n";
        assert!(run(&rule, "a.ts", body).is_empty());
    }

    #[test]
    fn fires_outside_excluded_ancestor_even_when_present_elsewhere() {
        let rule = build(UNKNOWN_QUERY, &["typescript"], &["catch_clause"]);
        let body = "const y: unknown = 1;\ntry {} catch (e: unknown) {}\n";
        let f = run(&rule, "a.ts", body);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, Some(1));
    }

    #[test]
    fn skips_files_in_unsupported_language() {
        let rule = build(ANY_QUERY, &["typescript"], &[]);
        assert!(run(&rule, "a.rs", "fn x() { let any: u8 = 1; }\n").is_empty());
    }

    fn try_build(rule_md: &str, query: &str) -> Result<AstRule, AstBuildError> {
        let parsed = parse_rule_md(rule_md, RuleSource::Repo, None).unwrap();
        AstRule::from_parsed(
            parsed,
            AstRuleSpec {
                query,
                capture: None,
                message: None,
                not_under: &[],
            },
        )
    }

    fn expect_build_err(rule_md: &str, query: &str) -> AstBuildError {
        match try_build(rule_md, query) {
            Ok(_) => panic!("expected build error"),
            Err(err) => err,
        }
    }

    #[test]
    fn build_rejects_invalid_query() {
        let md = r#"---
id: test.bad
name: "bad"
description: "x"
severity: error
category: style
languages: [typescript]
evaluator: { type: ast, query: "((not_a_real_node) @t)" }
---
"#;
        let err = expect_build_err(md, "((not_a_real_node) @t)");
        assert!(matches!(err, AstBuildError::Query { .. }));
    }

    #[test]
    fn build_rejects_missing_languages() {
        let md = r#"---
id: test.no-lang
name: "no lang"
description: "x"
severity: error
category: style
evaluator: { type: ast, query: "((identifier) @i)" }
---
"#;
        let err = expect_build_err(md, "((identifier) @i)");
        assert!(matches!(err, AstBuildError::MissingLanguages { .. }));
    }

    #[test]
    fn build_handles_well_formed_minimal_input() {
        let md = r#"---
id: test.ok
name: "ok"
description: "x"
severity: error
category: style
languages: [typescript]
evaluator: { type: ast, query: "((identifier) @i)" }
---
"#;
        try_build(md, "((identifier) @i)").unwrap();
    }
}
