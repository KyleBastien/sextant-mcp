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
//! Within-file matching lives in [`find_clones`]; cross-file matching
//! (token-equal regions spread across files of the same language) lives
//! in [`find_cross_file_clones`]. Both share the leaf-token stream and
//! sliding-window machinery; only the grouping changes.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use tree_sitter::TreeCursor;

use crate::parser::{Language, ParsedFile};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossFileClonePair {
    pub file_a: PathBuf,
    pub a: CloneSpan,
    pub file_b: PathBuf,
    pub b: CloneSpan,
    pub token_count: u32,
}

/// Find token-clone pairs whose two occurrences live in *different* files.
/// Inputs of different languages are never matched against each other.
/// Same-file pairs are excluded — `find_clones` owns those.
///
/// Result is deterministic: pairs are sorted by
/// `(file_a, a.start_line, file_b, b.start_line, token_count)`.
pub fn find_cross_file_clones(
    parsed: &[(PathBuf, &ParsedFile)],
    min_tokens: usize,
) -> Vec<CrossFileClonePair> {
    if min_tokens < 2 || parsed.len() < 2 {
        return Vec::new();
    }
    let mut by_language: HashMap<Language, Vec<usize>> = HashMap::new();
    for (idx, (_, file)) in parsed.iter().enumerate() {
        by_language.entry(file.language).or_default().push(idx);
    }
    let mut out = Vec::new();
    for indices in by_language.values() {
        if indices.len() < 2 {
            continue;
        }
        emit_cross_file_pairs(parsed, indices, min_tokens, &mut out);
    }
    out.sort_by(|x, y| {
        (
            x.file_a.as_path(),
            x.a.start_line,
            x.file_b.as_path(),
            x.b.start_line,
            x.token_count,
        )
            .cmp(&(
                y.file_a.as_path(),
                y.a.start_line,
                y.file_b.as_path(),
                y.b.start_line,
                y.token_count,
            ))
    });
    out
}

fn emit_cross_file_pairs(
    parsed: &[(PathBuf, &ParsedFile)],
    indices: &[usize],
    min_tokens: usize,
    out: &mut Vec<CrossFileClonePair>,
) {
    let streams: Vec<Vec<Token>> = indices
        .iter()
        .map(|&i| collect_tokens(parsed[i].1))
        .collect();
    let groups = build_cross_file_hash_groups(&streams, min_tokens);
    let mut consumed: Vec<Vec<bool>> = streams.iter().map(|s| vec![false; s.len()]).collect();
    for positions in groups {
        for pair in positions.windows(2) {
            try_record_cross_pair(
                CrossPairCtx {
                    parsed,
                    indices,
                    streams: &streams,
                    min_tokens,
                },
                pair[0],
                pair[1],
                &mut consumed,
                out,
            );
        }
    }
}

fn build_cross_file_hash_groups(
    streams: &[Vec<Token>],
    min_tokens: usize,
) -> Vec<Vec<(usize, usize)>> {
    let mut by_hash: HashMap<u64, Vec<(usize, usize)>> = HashMap::new();
    for (file_local, tokens) in streams.iter().enumerate() {
        if tokens.len() < min_tokens {
            continue;
        }
        for start in 0..=(tokens.len() - min_tokens) {
            let h = window_hash(tokens, start, min_tokens);
            by_hash.entry(h).or_default().push((file_local, start));
        }
    }
    let mut groups: Vec<Vec<(usize, usize)>> =
        by_hash.into_values().filter(|g| g.len() >= 2).collect();
    for g in groups.iter_mut() {
        g.sort();
    }
    groups.sort_by_key(|g| g[0]);
    groups
}

struct CrossPairCtx<'a> {
    parsed: &'a [(PathBuf, &'a ParsedFile)],
    indices: &'a [usize],
    streams: &'a [Vec<Token>],
    min_tokens: usize,
}

fn try_record_cross_pair(
    ctx: CrossPairCtx<'_>,
    a: (usize, usize),
    b: (usize, usize),
    consumed: &mut [Vec<bool>],
    out: &mut Vec<CrossFileClonePair>,
) {
    let (a_file, a_pos) = a;
    let (b_file, b_pos) = b;
    if a_file == b_file || consumed[a_file][a_pos] || consumed[b_file][b_pos] {
        return;
    }
    let s_a = &ctx.streams[a_file];
    let s_b = &ctx.streams[b_file];
    if !equal_window_cross(s_a, a_pos, s_b, b_pos, ctx.min_tokens) {
        return;
    }
    let len = extend_match_cross(s_a, a_pos, s_b, b_pos, ctx.min_tokens);
    for k in 0..len {
        consumed[a_file][a_pos + k] = true;
        consumed[b_file][b_pos + k] = true;
    }
    out.push(CrossFileClonePair {
        file_a: ctx.parsed[ctx.indices[a_file]].0.clone(),
        a: span(s_a, a_pos, len),
        file_b: ctx.parsed[ctx.indices[b_file]].0.clone(),
        b: span(s_b, b_pos, len),
        token_count: len as u32,
    });
}

fn equal_window_cross(a: &[Token], a_pos: usize, b: &[Token], b_pos: usize, len: usize) -> bool {
    (0..len).all(|k| a[a_pos + k].kind_hash == b[b_pos + k].kind_hash)
}

/// Forward extension across two separate token streams. Stops at the end
/// of *either* file (no cross-file token bleed) or at first divergence.
fn extend_match_cross(
    a: &[Token],
    a_pos: usize,
    b: &[Token],
    b_pos: usize,
    min_len: usize,
) -> usize {
    let max_len = (a.len() - a_pos).min(b.len() - b_pos);
    let mut len = min_len;
    while len < max_len && a[a_pos + len].kind_hash == b[b_pos + len].kind_hash {
        len += 1;
    }
    len
}

#[cfg(test)]
#[path = "clones_tests.rs"]
mod tests;
