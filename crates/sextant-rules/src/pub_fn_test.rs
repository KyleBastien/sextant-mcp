//! Flag public-API definitions whose name doesn't appear in any test
//! body — either in the same file or in a conventional peer test file
//! sitting next to the source. The rule prefers peer-file tests (the
//! more common layout) but accepts in-file tests too.
//!
//! Severity is `info` — this is a signal that helps the agent decide
//! where to focus, not a verdict-breaker.

use std::path::{Path, PathBuf};

use sextant_core::{EvalContext, Evaluator, Finding, Rule, SourceFile};
use sextant_lang::{parse, test_haystack_mentions, test_witness, Language};

use crate::file_length::rule_from_parsed;
use crate::loader::ParsedRule;
use crate::patch::create_file_diff;

pub struct PubFnUntestedRule {
    rule: Rule,
}

impl PubFnUntestedRule {
    pub fn from_parsed(parsed: ParsedRule) -> Self {
        Self {
            rule: rule_from_parsed(parsed),
        }
    }
}

impl Evaluator for PubFnUntestedRule {
    fn rule(&self) -> &Rule {
        &self.rule
    }

    fn evaluate_file(&self, file: &SourceFile, ctx: &EvalContext<'_>) -> Vec<Finding> {
        let Some(lang) = supported_language(file) else {
            return Vec::new();
        };
        let Ok(parsed) = parse(file.contents.clone(), lang) else {
            return Vec::new();
        };
        let witness = test_witness(&parsed);
        if witness.pub_fns.is_empty() {
            return Vec::new();
        }
        let path = file.relative_to(ctx.repo_root);
        let peers = peer_test_files(lang, &file.path);
        let peer_haystack = read_peer_haystack(&peers);
        witness
            .pub_fns
            .iter()
            .filter(|pf| !test_haystack_mentions(&witness.test_haystack, &pf.name))
            .filter(|pf| !test_haystack_mentions(&peer_haystack, &pf.name))
            .map(|pf| self.build_finding(lang, &path, pf, ctx.repo_root))
            .collect()
    }
}

impl PubFnUntestedRule {
    fn build_finding(
        &self,
        lang: Language,
        path: &Path,
        pf: &sextant_lang::PubFnInfo,
        repo_root: &Path,
    ) -> Finding {
        let msg = message_for(lang, &pf.name);
        let mut finding = Finding::new(&self.rule.id, self.rule.severity, path.to_path_buf(), msg)
            .spanning(pf.start_line, pf.end_line);
        if let Some((peer_path, stub)) = peer_test_stub(lang, path, &pf.name) {
            if !repo_root.join(&peer_path).exists() {
                finding = finding.with_patch(create_file_diff(&peer_path, &stub));
            }
        }
        finding
    }
}

fn supported_language(file: &SourceFile) -> Option<Language> {
    let lang = Language::from_hint(file.language_hint()?)?;
    matches!(
        lang,
        Language::Rust | Language::JavaScript | Language::TypeScript | Language::Tsx
    )
    .then_some(lang)
}

/// Conventional peer test files for a given source file. Returned paths
/// are absolute (joined onto `src_path`'s parent / ancestors) and may
/// not exist on disk — the caller is expected to filter via `Path::exists`.
fn peer_test_files(lang: Language, src_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = src_path.parent() else {
        return Vec::new();
    };
    let Some(stem) = src_path.file_stem().and_then(|s| s.to_str()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    match lang {
        Language::Rust => {
            out.push(parent.join(format!("{stem}_tests.rs")));
            out.push(parent.join("tests").join(format!("{stem}.rs")));
            // Walk up to the crate root (parent of `src/`) and try
            // `tests/<stem>.rs` — Cargo's integration-test convention.
            let mut cursor = parent;
            while let Some(p) = cursor.parent() {
                if cursor.file_name().and_then(|n| n.to_str()) == Some("src") {
                    out.push(p.join("tests").join(format!("{stem}.rs")));
                    break;
                }
                cursor = p;
            }
        }
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            let exts = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];
            for ext in exts {
                out.push(parent.join(format!("{stem}.test.{ext}")));
                out.push(parent.join(format!("{stem}.spec.{ext}")));
                out.push(parent.join("__tests__").join(format!("{stem}.test.{ext}")));
                out.push(parent.join("__tests__").join(format!("{stem}.spec.{ext}")));
            }
        }
        _ => {}
    }
    out
}

/// Read all existing peer files and concatenate their contents into a
/// single haystack for whole-word identifier matching.
fn read_peer_haystack(peers: &[PathBuf]) -> String {
    let mut out = String::new();
    for p in peers {
        if let Ok(contents) = std::fs::read_to_string(p) {
            out.push_str(&contents);
            out.push('\n');
        }
    }
    out
}

/// Build a `(peer_path, peer_contents)` pair for a fresh peer test
/// file. The peer path is the conventional sibling location for `lang`
/// (`<stem>_tests.rs` for Rust, `<stem>.test.<ext>` for JS/TS). Returns
/// `None` for unsupported languages or when the source path can't be
/// decomposed into parent + stem.
fn peer_test_stub(lang: Language, src_path: &Path, name: &str) -> Option<(PathBuf, String)> {
    let parent = src_path.parent()?;
    let stem = src_path.file_stem()?.to_str()?;
    match lang {
        Language::Rust => Some(rust_peer_stub(parent, stem, name)),
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            Some(js_peer_stub(lang, parent, stem, name))
        }
        _ => None,
    }
}

fn rust_peer_stub(parent: &Path, stem: &str, name: &str) -> (PathBuf, String) {
    let peer = parent.join(format!("{stem}_tests.rs"));
    let body = format!(
        "use super::*;\n\n#[test]\nfn {name}_smoke() {{\n    // TODO: exercise `{name}` and assert its behaviour.\n    let _ = {name};\n}}\n"
    );
    (peer, body)
}

fn js_peer_stub(lang: Language, parent: &Path, stem: &str, name: &str) -> (PathBuf, String) {
    let ext = match lang {
        Language::TypeScript => "ts",
        Language::Tsx => "tsx",
        _ => "js",
    };
    let peer = parent.join(format!("{stem}.test.{ext}"));
    let body = format!(
        "import {{ describe, it, expect }} from 'vitest';\nimport {{ {name} }} from './{stem}';\n\ndescribe('{name}', () => {{\n    it('is exercised', () => {{\n        // TODO: exercise `{name}` and assert its behaviour.\n        expect({name}).toBeDefined();\n    }});\n}});\n"
    );
    (peer, body)
}

fn message_for(lang: Language, name: &str) -> String {
    match lang {
        Language::Rust => format!(
            "Public function `{name}` is not referenced by any `#[test]` in this file or a peer \
             test file (`<stem>_tests.rs` sibling or `tests/<stem>.rs`). Prefer adding the test \
             in a peer file; an in-file `#[cfg(test)] mod` is also fine. Or reduce visibility to \
             `pub(crate)`."
        ),
        Language::JavaScript | Language::TypeScript | Language::Tsx => format!(
            "Exported `{name}` is not referenced by any `describe`/`it`/`test` block in this file \
             or a peer test file (sibling `*.test.*` / `*.spec.*`, or one under `__tests__/`). \
             Prefer adding the test in a peer file; an in-file Vitest block is also fine. Or drop \
             the `export`."
        ),
        _ => format!(
            "Public `{name}` is not referenced by any test in this file or a peer test file."
        ),
    }
}

#[cfg(test)]
#[path = "pub_fn_test_tests.rs"]
mod tests;
