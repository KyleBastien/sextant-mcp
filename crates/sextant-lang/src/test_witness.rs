//! Locate public functions and the bodies of `#[test]` functions in a
//! Rust file. Used by the "pub fn without adjacent test" rule.
//!
//! Two structural decisions:
//!   - Only fully-public `pub fn`. `pub(crate)`, `pub(super)`, `pub(in …)`
//!     are intentionally excluded — they're internal API and the rule's
//!     intent is "public surface should have a test next to it".
//!   - Public functions inside a module whose preceding attribute is
//!     `#[cfg(test)]` are excluded — those are test helpers, not part of
//!     the public surface.

use tree_sitter::{Node, TreeCursor};

use crate::parser::{Language, ParsedFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubFnInfo {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Default)]
pub struct TestWitness {
    pub pub_fns: Vec<PubFnInfo>,
    /// Source slices of bodies of `#[test]`-annotated functions, joined
    /// into a single haystack to keep the rule's lookup cheap.
    pub test_haystack: String,
}

/// Walk the tree once and gather pub fns + test-body text. Non-Rust files
/// return an empty witness.
pub fn rust_test_witness(parsed: &ParsedFile) -> TestWitness {
    if parsed.language != Language::Rust {
        return TestWitness::default();
    }
    let mut state = WalkState {
        source: &parsed.source,
        out: TestWitness::default(),
    };
    let mut cursor = parsed.tree.walk();
    walk(&mut cursor, &mut state, /*in_cfg_test=*/ false);
    state.out
}

struct WalkState<'a> {
    source: &'a str,
    out: TestWitness,
}

fn walk(cursor: &mut TreeCursor<'_>, state: &mut WalkState<'_>, in_cfg_test: bool) {
    if !cursor.goto_first_child() {
        return;
    }
    let mut last_outer_attrs: Vec<Node<'_>> = Vec::new();
    loop {
        let node = cursor.node();
        match node.kind() {
            "attribute_item" => {
                last_outer_attrs.push(node);
            }
            "function_item" => {
                handle_function(node, &last_outer_attrs, state, in_cfg_test);
                last_outer_attrs.clear();
            }
            "mod_item" => {
                let nested_in_cfg_test =
                    in_cfg_test || attrs_indicate(&last_outer_attrs, state.source, "cfg(test)");
                last_outer_attrs.clear();
                walk(cursor, state, nested_in_cfg_test);
            }
            _ => {
                last_outer_attrs.clear();
                walk(cursor, state, in_cfg_test);
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    cursor.goto_parent();
}

fn handle_function(
    fn_node: Node<'_>,
    attrs: &[Node<'_>],
    state: &mut WalkState<'_>,
    in_cfg_test: bool,
) {
    let is_test = attrs_indicate(attrs, state.source, "test");
    if is_test {
        if let Some(body) = fn_node.child_by_field_name("body") {
            state
                .out
                .test_haystack
                .push_str(&state.source[body.byte_range()]);
            state.out.test_haystack.push('\n');
        }
        return;
    }
    if in_cfg_test {
        return;
    }
    let Some(name_node) = fn_node.child_by_field_name("name") else {
        return;
    };
    let Some(vis) = function_visibility(&fn_node, state.source) else {
        return;
    };
    if vis != "pub" {
        return;
    }
    state.out.pub_fns.push(PubFnInfo {
        name: state.source[name_node.byte_range()].to_string(),
        start_line: (fn_node.start_position().row as u32) + 1,
        end_line: (fn_node.end_position().row as u32) + 1,
    });
}

fn function_visibility<'a>(fn_node: &Node<'_>, source: &'a str) -> Option<&'a str> {
    let mut walker = fn_node.walk();
    let vis = fn_node
        .children(&mut walker)
        .find(|c| c.kind() == "visibility_modifier")
        .map(|n| n.byte_range());
    vis.map(|r| source[r].trim())
}

/// Does any of `attrs` mention `needle`? Used for both `test` (matches
/// `#[test]`, `#[tokio::test]`, `#[async_std::test]`) and `cfg(test)`.
fn attrs_indicate(attrs: &[Node<'_>], source: &str, needle: &str) -> bool {
    attrs
        .iter()
        .any(|a| contains_token(&source[a.byte_range()], needle))
}

/// Substring match with a left word boundary. Good enough to distinguish
/// `test` from `attest` while staying simple — and it doesn't need a
/// trailing boundary because `#[test]`/`cfg(test)` always have one.
fn contains_token(haystack: &str, needle: &str) -> bool {
    let mut start = 0;
    while let Some(idx) = haystack[start..].find(needle) {
        let abs = start + idx;
        if abs == 0 || !is_ident_char(haystack.as_bytes()[abs - 1]) {
            return true;
        }
        start = abs + needle.len();
    }
    false
}

/// True if `name` appears as a whole word inside the witness's test
/// haystack.
pub fn test_haystack_mentions(haystack: &str, name: &str) -> bool {
    let mut start = 0;
    while let Some(idx) = haystack[start..].find(name) {
        let abs = start + idx;
        let left_ok = abs == 0 || !is_ident_char(haystack.as_bytes()[abs - 1]);
        let end = abs + name.len();
        let right_ok = end >= haystack.len() || !is_ident_char(haystack.as_bytes()[end]);
        if left_ok && right_ok {
            return true;
        }
        start = abs + name.len();
    }
    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, Language};

    fn witness(src: &str) -> TestWitness {
        let parsed = parse(src, Language::Rust).unwrap();
        rust_test_witness(&parsed)
    }

    #[test]
    fn finds_pub_fns_and_their_tests() {
        let src = r#"
pub fn add(a: i32, b: i32) -> i32 { a + b }

pub fn untested() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_works() {
        assert_eq!(add(1, 2), 3);
    }
}
"#;
        let w = witness(src);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["add", "untested"]);
        assert!(test_haystack_mentions(&w.test_haystack, "add"));
        assert!(!test_haystack_mentions(&w.test_haystack, "untested"));
    }

    #[test]
    fn excludes_pub_fns_inside_cfg_test_mod() {
        let src = r#"
#[cfg(test)]
mod tests {
    pub fn helper() {}

    #[test]
    fn t() { helper(); }
}
"#;
        let w = witness(src);
        assert!(w.pub_fns.is_empty(), "{:?}", w.pub_fns);
    }

    #[test]
    fn excludes_pub_crate_and_pub_super() {
        let src = r#"
pub(crate) fn a() {}
pub(super) fn b() {}
pub(in crate::x) fn c() {}
pub fn d() {}
"#;
        let w = witness(src);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["d"]);
    }

    #[test]
    fn recognizes_tokio_test_attribute() {
        let src = r#"
pub fn run() {}

#[tokio::test]
async fn run_works() { run(); }
"#;
        let w = witness(src);
        assert!(test_haystack_mentions(&w.test_haystack, "run"));
    }

    #[test]
    fn cfg_test_mod_can_be_named_anything() {
        let src = r#"
pub fn x() {}

#[cfg(test)]
mod my_tests {
    #[test]
    fn t() { super::x(); }
}
"#;
        let w = witness(src);
        // x is found, and its mention in the test body is detected.
        assert_eq!(w.pub_fns.len(), 1);
        assert!(test_haystack_mentions(&w.test_haystack, "x"));
    }

    #[test]
    fn token_match_does_not_match_substrings() {
        let h = "let attest = 1; testify();";
        assert!(!test_haystack_mentions(h, "test"));
        assert!(test_haystack_mentions(h, "attest"));
    }

    #[test]
    fn non_rust_returns_empty() {
        let parsed = parse("def f(): pass\n", Language::Python).unwrap();
        let w = rust_test_witness(&parsed);
        assert!(w.pub_fns.is_empty());
        assert!(w.test_haystack.is_empty());
    }
}
