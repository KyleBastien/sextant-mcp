use super::*;
use crate::parser::parse;
use std::path::PathBuf;

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

const DUP_FN_A: &str = r#"
fn one() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}
"#;

const DUP_FN_B: &str = r#"
fn two() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}
"#;

fn cross_pairs(inputs: &[(&str, &str, Language)], min: usize) -> Vec<CrossFileClonePair> {
    let parsed: Vec<(PathBuf, ParsedFile)> = inputs
        .iter()
        .map(|(p, src, lang)| (PathBuf::from(p), parse(*src, *lang).unwrap()))
        .collect();
    let refs: Vec<(PathBuf, &ParsedFile)> = parsed.iter().map(|(p, pf)| (p.clone(), pf)).collect();
    find_cross_file_clones(&refs, min)
}

#[test]
fn cross_file_finds_exact_duplicate_function() {
    let pairs = cross_pairs(
        &[
            ("a.rs", DUP_FN_A, Language::Rust),
            ("b.rs", DUP_FN_B, Language::Rust),
        ],
        15,
    );
    assert_eq!(pairs.len(), 1, "{pairs:?}");
    let p = &pairs[0];
    assert_eq!(p.file_a, PathBuf::from("a.rs"));
    assert_eq!(p.file_b, PathBuf::from("b.rs"));
    assert!(p.token_count >= 15);
}

#[test]
fn cross_file_detects_type2_renamed_clone() {
    let a = "fn alpha(x: i32) -> i32 { if x > 0 { return 1; } return 0; }";
    let b = "fn beta(y: i32) -> i32 { if y > 5 { return 2; } return 99; }";
    let pairs = cross_pairs(
        &[("a.rs", a, Language::Rust), ("b.rs", b, Language::Rust)],
        10,
    );
    assert!(!pairs.is_empty(), "expected match, got: {pairs:?}");
}

#[test]
fn cross_file_skips_same_file_pairs() {
    let src = r#"
fn f() {
    let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; let f = 6;
    let a = 1; let b = 2; let c = 3; let d = 4; let e = 5; let f = 6;
}
"#;
    let pairs = cross_pairs(&[("only.rs", src, Language::Rust)], 10);
    assert!(
        pairs.is_empty(),
        "in-file matcher owns same-file pairs: {pairs:?}"
    );
}

#[test]
fn cross_file_groups_by_language() {
    let rust_src = "fn alpha(x: i32) -> i32 { if x > 0 { return 1; } return 0; }";
    let py_src = "def alpha(x):\n    if x > 0:\n        return 1\n    return 0\n";
    let pairs = cross_pairs(
        &[
            ("a.rs", rust_src, Language::Rust),
            ("b.py", py_src, Language::Python),
        ],
        5,
    );
    assert!(pairs.is_empty(), "languages must not cross: {pairs:?}");
}

#[test]
fn cross_file_respects_min_tokens() {
    let pairs = cross_pairs(
        &[
            ("a.rs", DUP_FN_A, Language::Rust),
            ("b.rs", DUP_FN_B, Language::Rust),
        ],
        500,
    );
    assert!(pairs.is_empty(), "below threshold: {pairs:?}");
}

#[test]
fn cross_file_extends_to_maximal_within_file_bounds() {
    let pairs = cross_pairs(
        &[
            ("a.rs", DUP_FN_A, Language::Rust),
            ("b.rs", DUP_FN_B, Language::Rust),
        ],
        5,
    );
    assert_eq!(pairs.len(), 1, "{pairs:?}");
    let p = &pairs[0];
    assert!(
        p.a.end_line >= p.a.start_line && p.b.end_line >= p.b.start_line,
        "spans malformed: {p:?}"
    );
    assert!(p.token_count >= 15, "should extend past min: {p:?}");
}

#[test]
fn cross_file_emits_one_pair_per_match() {
    let src_c = r#"
fn three() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
}
"#;
    let pairs = cross_pairs(
        &[
            ("a.rs", DUP_FN_A, Language::Rust),
            ("b.rs", DUP_FN_B, Language::Rust),
            ("c.rs", src_c, Language::Rust),
        ],
        15,
    );
    assert!(
        !pairs.is_empty() && pairs.len() <= 3,
        "expected pairs across the three files: {pairs:?}"
    );
}
