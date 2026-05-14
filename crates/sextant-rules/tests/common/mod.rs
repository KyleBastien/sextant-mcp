//! Shared helpers for the TypeScript-pack integration tests. Splitting
//! these out keeps the per-file token-window stream from triggering the
//! cross-file `builtin.duplication.tokens` rule on identical boilerplate.

use std::path::PathBuf;

use sextant_core::RuleSource;
use sextant_rules::{parse_rule_md, AstRule, AstRuleSpec, EvaluatorSpec, ParsedRule};

pub fn pack_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("packs")
        .join("typescript")
}

pub fn parse_pack_rule(filename: &str) -> ParsedRule {
    let path = pack_root().join("rules").join(filename);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    parse_rule_md(
        &text,
        RuleSource::Vendor("typescript".into()),
        Some(path.clone()),
    )
    .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

pub fn load_rule(filename: &str) -> AstRule {
    let parsed = parse_pack_rule(filename);
    let (query, capture, message, not_under) = match &parsed.evaluator {
        EvaluatorSpec::Ast {
            query,
            capture,
            message,
            not_under,
        } => (
            query.clone(),
            capture.clone(),
            message.clone(),
            not_under.clone(),
        ),
        other => panic!("expected ast evaluator, got {other:?}"),
    };
    AstRule::from_parsed(
        parsed,
        AstRuleSpec {
            query: &query,
            capture: capture.as_deref(),
            message: message.as_deref(),
            not_under: &not_under,
        },
    )
    .unwrap_or_else(|e| panic!("building {filename}: {e}"))
}

#[cfg(test)]
mod smoke {
    //! Self-tests so the helpers are exercised even when only one
    //! integration test binary runs. Each helper is reachable from the
    //! peer integration files, but the `pub-fn-untested` rule wants a
    //! direct reference in this module too.
    use super::*;

    #[test]
    fn pack_root_points_at_the_typescript_pack() {
        let root = pack_root();
        assert!(root.ends_with("packs/typescript"), "{root:?}");
    }

    #[test]
    fn helpers_load_a_pack_rule() {
        // Touch both higher-level helpers via a representative rule so
        // they're named from a real test path.
        let parsed = parse_pack_rule("no-any.md");
        assert_eq!(parsed.id, "vendor.typescript.no-any");
        let _ast = load_rule("no-any.md");
    }
}
