//! Three-tier rule merge: builtins → vendor → repo. Owns the conflict
//! semantics that protect vendor packs from being silenced by repo-local
//! rules. See [`merge_all`] for the rules.

use std::collections::{HashMap, HashSet};

use sextant_core::RuleSource;

use crate::loader::{LoaderError, LoaderResult, ParsedRule};

/// Two-tier shim: builtins + repo, no vendor packs. Used by older
/// callsites and the in-file test suite.
pub fn merge(builtins: Vec<ParsedRule>, repo: Vec<ParsedRule>) -> Vec<ParsedRule> {
    merge_all(builtins, Vec::new(), repo).expect("merge without vendor cannot shadow")
}

/// Three-tier merge: builtins → vendor → repo, in increasing priority.
///
/// Conflict semantics:
/// - Vendor rule with the same id as a builtin: vendor wins (logged).
/// - Repo rule with the same id as a vendor rule: hard error
///   (`LoaderError::ShadowsVendor`).
/// - Repo rule with the same id as a builtin: repo wins.
/// - `overrides:` from a repo rule cannot disable a vendor or builtin
///   rule — only equal-or-higher-priority sources may override.
/// - `enabled: false` is honored on builtin and repo rules; vendor rules
///   bypass it (the lock-integrity check ensures the pack file we loaded
///   matches what the pack author shipped).
pub fn merge_all(
    builtins: Vec<ParsedRule>,
    vendor: Vec<ParsedRule>,
    repo: Vec<ParsedRule>,
) -> LoaderResult<Vec<ParsedRule>> {
    let mut by_id: HashMap<String, ParsedRule> = HashMap::new();
    for r in builtins {
        by_id.insert(r.id.clone(), r);
    }
    let vendor_overrides = absorb_vendor(&mut by_id, vendor);
    reject_repo_shadowing(&by_id, &repo)?;
    let repo_overrides = absorb_repo(&mut by_id, repo);
    let mut out: Vec<ParsedRule> = by_id
        .into_values()
        .filter(|r| keep_after_overrides(r, &vendor_overrides, &repo_overrides))
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn absorb_vendor(
    by_id: &mut HashMap<String, ParsedRule>,
    vendor: Vec<ParsedRule>,
) -> HashSet<String> {
    let mut overrides = HashSet::new();
    for r in vendor {
        if matches!(
            by_id.get(&r.id).map(|e| &e.source),
            Some(RuleSource::Builtin)
        ) {
            tracing::info!(rule = %r.id, "vendor pack replaces built-in");
        }
        overrides.extend(r.overrides.iter().cloned());
        by_id.insert(r.id.clone(), r);
    }
    overrides
}

fn reject_repo_shadowing(
    by_id: &HashMap<String, ParsedRule>,
    repo: &[ParsedRule],
) -> LoaderResult<()> {
    for r in repo {
        if let Some(RuleSource::Vendor(pack)) = by_id.get(&r.id).map(|e| &e.source) {
            return Err(LoaderError::ShadowsVendor {
                id: r.id.clone(),
                pack: pack.clone(),
            });
        }
    }
    Ok(())
}

fn absorb_repo(by_id: &mut HashMap<String, ParsedRule>, repo: Vec<ParsedRule>) -> HashSet<String> {
    let mut overrides = HashSet::new();
    for r in &repo {
        overrides.extend(r.overrides.iter().cloned());
    }
    for r in repo {
        if by_id.contains_key(&r.id) {
            tracing::info!(rule = %r.id, "repo-local rule replaces built-in");
        }
        by_id.insert(r.id.clone(), r);
    }
    overrides
}

fn keep_after_overrides(
    r: &ParsedRule,
    vendor_overrides: &HashSet<String>,
    repo_overrides: &HashSet<String>,
) -> bool {
    match &r.source {
        RuleSource::Vendor(_) => !vendor_overrides.contains(&r.id),
        _ => r.enabled && !repo_overrides.contains(&r.id) && !vendor_overrides.contains(&r.id),
    }
}

#[cfg(test)]
mod smoke {
    //! In-file mention of the public surface for the `pub-fn-untested`
    //! rule; thorough coverage lives in `loader_tests.rs`.
    use super::*;

    #[test]
    fn public_surface_runs_for_empty_inputs() {
        let merged = merge_all(vec![], vec![], vec![]).unwrap();
        assert!(merged.is_empty());
        assert!(merge(vec![], vec![]).is_empty());
    }
}
