//! Token-based clone detection.
//!
//! We extract a stream of leaf-node *kinds* from the tree-sitter parse,
//! hash sliding windows of `min_tokens`, group windows by hash, and emit
//! one `ClonePair` per non-overlapping pair (extended to maximal length).
//!
//! Hashing only the node kind catches "type-2" clones — code that's
//! structurally identical except for identifier and literal renames. That's
//! the most common form of accidental duplication and the most actionable
//! for refactoring.
//!
//! Scope is currently within a single file. Cross-file detection would
//! require an evaluator API that sees all files at once; deferred.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use tree_sitter::TreeCursor;

use crate::parser::ParsedFile;

#[derive(Debug, Clone, Copy)]
struct Token {
    kind_hash: u64,
    line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneSpan {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClonePair {
    pub a: CloneSpan,
    pub b: CloneSpan,
    pub token_count: u32,
}

/// Find all token-clone pairs of length >= `min_tokens`. The result is
/// sorted by `a.start_line` for determinism.
pub fn find_clones(parsed: &ParsedFile, min_tokens: usize) -> Vec<ClonePair> {
    if min_tokens < 2 {
        return Vec::new();
    }
    let tokens = collect_tokens(parsed);
    if tokens.len() < min_tokens * 2 {
        return Vec::new();
    }

    let groups = group_by_window_hash(&tokens, min_tokens);
    let mut consumed = vec![false; tokens.len()];
    let mut pairs = Vec::new();
    for positions in groups {
        emit_pairs(&tokens, &positions, min_tokens, &mut consumed, &mut pairs);
    }
    pairs.sort_by_key(|p| (p.a.start_line, p.b.start_line, p.token_count));
    pairs
}

fn group_by_window_hash(tokens: &[Token], min_tokens: usize) -> Vec<Vec<usize>> {
    let mut by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    for start in 0..=(tokens.len() - min_tokens) {
        let h = window_hash(tokens, start, min_tokens);
        by_hash.entry(h).or_default().push(start);
    }
    let mut groups: Vec<Vec<usize>> = by_hash.into_values().filter(|p| p.len() >= 2).collect();
    // Stable order: by smallest position in each group.
    for g in groups.iter_mut() {
        g.sort();
    }
    groups.sort_by_key(|g| g[0]);
    groups
}

fn emit_pairs(
    tokens: &[Token],
    positions: &[usize],
    min_tokens: usize,
    consumed: &mut [bool],
    pairs: &mut Vec<ClonePair>,
) {
    for window in positions.windows(2) {
        let (a, b) = (window[0], window[1]);
        if consumed[a] || consumed[b] {
            continue;
        }
        // Reject overlap of the two regions.
        if b < a + min_tokens {
            continue;
        }
        // Hash collision check — proves the windows are token-equal.
        if !equal_window(tokens, a, b, min_tokens) {
            continue;
        }
        let len = extend_match(tokens, a, b, min_tokens);
        for k in 0..len {
            consumed[a + k] = true;
            consumed[b + k] = true;
        }
        pairs.push(ClonePair {
            a: span(tokens, a, len),
            b: span(tokens, b, len),
            token_count: len as u32,
        });
    }
}

/// Greedy forward-extension of a confirmed match. Stops when tokens diverge,
/// when extension would cause the two regions to overlap, or at the file end.
fn extend_match(tokens: &[Token], a: usize, b: usize, min_len: usize) -> usize {
    let max_len = (b - a).min(tokens.len() - b);
    let mut len = min_len;
    while len < max_len && tokens[a + len].kind_hash == tokens[b + len].kind_hash {
        len += 1;
    }
    len
}

fn collect_tokens(parsed: &ParsedFile) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cursor = parsed.tree.walk();
    visit_leaves(&mut cursor, &mut tokens);
    tokens
}

fn visit_leaves(cursor: &mut TreeCursor, out: &mut Vec<Token>) {
    let node = cursor.node();
    if node.child_count() == 0 {
        push_leaf(node, out);
        return;
    }
    if cursor.goto_first_child() {
        loop {
            visit_leaves(cursor, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn push_leaf(node: tree_sitter::Node<'_>, out: &mut Vec<Token>) {
    let kind = node.kind();
    if matches!(kind, "line_comment" | "block_comment" | "comment") {
        return;
    }
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    out.push(Token {
        kind_hash: hasher.finish(),
        line: (node.start_position().row as u32) + 1,
    });
}

fn window_hash(tokens: &[Token], start: usize, len: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    for t in &tokens[start..start + len] {
        t.kind_hash.hash(&mut hasher);
    }
    hasher.finish()
}

fn equal_window(tokens: &[Token], a: usize, b: usize, len: usize) -> bool {
    (0..len).all(|k| tokens[a + k].kind_hash == tokens[b + k].kind_hash)
}

fn span(tokens: &[Token], start: usize, len: usize) -> CloneSpan {
    CloneSpan {
        start_line: tokens[start].line,
        end_line: tokens[start + len - 1].line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, Language};

    #[test]
    fn finds_exact_duplicate_function() {
        let src = r#"
fn one() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}

fn two() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}
"#;
        let parsed = parse(src, Language::Rust).unwrap();
        let clones = find_clones(&parsed, 20);
        assert_eq!(clones.len(), 1, "{clones:?}");
        let c = &clones[0];
        assert!(c.a.start_line < c.b.start_line);
        assert!(c.token_count >= 20);
    }

    #[test]
    fn detects_type2_renamed_clone() {
        // Same structure, different identifiers and literals.
        let src = r#"
fn alpha(x: i32) -> i32 {
    if x > 0 {
        return 1;
    }
    return 0;
}

fn beta(y: i32) -> i32 {
    if y > 5 {
        return 2;
    }
    return 99;
}
"#;
        let parsed = parse(src, Language::Rust).unwrap();
        let clones = find_clones(&parsed, 15);
        assert!(!clones.is_empty(), "expected a clone, got: {clones:?}");
    }

    #[test]
    fn ignores_short_overlap() {
        // Below threshold — no pairs reported.
        let src = "fn one() {}\nfn two() {}\n";
        let parsed = parse(src, Language::Rust).unwrap();
        assert!(find_clones(&parsed, 50).is_empty());
    }

    #[test]
    fn pairs_are_non_overlapping() {
        let src = r#"
fn f() {
    let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; let f = 6;
    let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; let f = 6;
}
"#;
        let parsed = parse(src, Language::Rust).unwrap();
        let clones = find_clones(&parsed, 10);
        for c in &clones {
            assert!(
                c.a.end_line <= c.b.start_line,
                "spans should not overlap: {c:?}"
            );
        }
    }

    #[test]
    fn works_for_python() {
        let src = r#"
def alpha(x):
    if x > 0:
        return 1
    elif x < 0:
        return -1
    else:
        return 0

def beta(y):
    if y > 0:
        return 1
    elif y < 0:
        return -1
    else:
        return 0
"#;
        let parsed = parse(src, Language::Python).unwrap();
        let clones = find_clones(&parsed, 15);
        assert!(!clones.is_empty(), "{clones:?}");
    }
}
