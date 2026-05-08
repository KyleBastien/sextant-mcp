//! Locate public functions and the bodies of test functions in a single
//! source file. Used by the "pub fn without adjacent test" rule.
//!
//! Two structural decisions, applied per-language:
//!   - Rust: only fully-public `pub fn`. `pub(crate)`, `pub(super)`,
//!     `pub(in …)` are intentionally excluded — they're internal API and
//!     the rule's intent is "public surface should have a test next to
//!     it". Public functions inside a module whose preceding attribute is
//!     `#[cfg(test)]` are excluded — those are test helpers, not part of
//!     the public surface.
//!   - JS/TS/TSX: only declarations exported with the `export` keyword.
//!     `export function f`, `export const f = () => {}`, `export class C`,
//!     and `export default function f` count. Bare top-level `function f`
//!     is module-private (analogous to Rust's lack of `pub`). Type-only
//!     exports (`export type`, `export interface`) are skipped. The test
//!     haystack is the body source of every `describe`/`it`/`test`/
//!     `suite`/`context` call (including `.skip`/`.only`/`.each` member
//!     forms), which covers Jest, Vitest, Mocha, and Jasmine in-file
//!     test patterns.

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

/// JS/TS/TSX equivalent of [`rust_test_witness`]. Collects exported
/// declarations and the bodies of in-file `describe`/`it`/`test` calls.
/// Returns an empty witness for files outside the JS family.
pub fn js_test_witness(parsed: &ParsedFile) -> TestWitness {
    if !matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return TestWitness::default();
    }
    let mut state = JsState {
        source: &parsed.source,
        out: TestWitness::default(),
    };
    walk_js(parsed.tree.root_node(), &mut state);
    state.out
}

/// Dispatch to the language-appropriate witness collector. Returns an
/// empty witness for unsupported languages.
pub fn test_witness(parsed: &ParsedFile) -> TestWitness {
    match parsed.language {
        Language::Rust => rust_test_witness(parsed),
        Language::JavaScript | Language::TypeScript | Language::Tsx => js_test_witness(parsed),
        _ => TestWitness::default(),
    }
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
    // A function counts as part of the "test scaffolding" if it's
    // `#[test]`-attributed itself, or if it lives inside a
    // `#[cfg(test)]` module — that catches test helpers like `build()`,
    // `ctx()`, and the `parsed_for_test()` factories that tests
    // routinely delegate to. The function name we're looking for shows
    // up there, not in the `#[test]` body itself.
    if is_test || in_cfg_test {
        if let Some(body) = fn_node.child_by_field_name("body") {
            state
                .out
                .test_haystack
                .push_str(&state.source[body.byte_range()]);
            state.out.test_haystack.push('\n');
        }
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

struct JsState<'a> {
    source: &'a str,
    out: TestWitness,
}

fn walk_js(node: Node<'_>, state: &mut JsState<'_>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "export_statement" => {
                collect_js_exports(child, state);
                // Test calls inside an export are vanishingly rare, but
                // recursing keeps the haystack consistent if they occur.
                walk_js(child, state);
            }
            "call_expression" if is_js_test_call(&child, state.source) => {
                if let Some(body) = js_test_callback_body(&child) {
                    state
                        .out
                        .test_haystack
                        .push_str(&state.source[body.byte_range()]);
                    state.out.test_haystack.push('\n');
                }
                // The body's text already covers nested test calls — no
                // need to descend.
            }
            _ => walk_js(child, state),
        }
    }
}

fn collect_js_exports(export_node: Node<'_>, state: &mut JsState<'_>) {
    let mut cursor = export_node.walk();
    for child in export_node.children(&mut cursor) {
        match child.kind() {
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration" => {
                push_js_named_def(&child, state);
            }
            "lexical_declaration" | "variable_declaration" => {
                collect_js_lexical_declarators(&child, state);
            }
            _ => {}
        }
    }
}

fn push_js_named_def(node: &Node<'_>, state: &mut JsState<'_>) {
    let Some(name) = node.child_by_field_name("name") else {
        return;
    };
    state.out.pub_fns.push(PubFnInfo {
        name: state.source[name.byte_range()].to_string(),
        start_line: (node.start_position().row as u32) + 1,
        end_line: (node.end_position().row as u32) + 1,
    });
}

fn collect_js_lexical_declarators(lex: &Node<'_>, state: &mut JsState<'_>) {
    let mut cursor = lex.walk();
    for decl in lex.named_children(&mut cursor) {
        if decl.kind() != "variable_declarator" {
            continue;
        }
        let Some(name) = decl.child_by_field_name("name") else {
            continue;
        };
        if name.kind() != "identifier" {
            continue;
        }
        let Some(value) = decl.child_by_field_name("value") else {
            continue;
        };
        if !matches!(
            value.kind(),
            "arrow_function" | "function_expression" | "function" | "generator_function"
        ) {
            continue;
        }
        state.out.pub_fns.push(PubFnInfo {
            name: state.source[name.byte_range()].to_string(),
            start_line: (decl.start_position().row as u32) + 1,
            end_line: (decl.end_position().row as u32) + 1,
        });
    }
}

/// True when `call`'s callee is one of the well-known test-runner
/// identifiers — bare or as the object of a chained access like
/// `it.skip(...)`, `test.each(...)(...)`, or `describe.only(...)`.
fn is_js_test_call(call: &Node<'_>, source: &str) -> bool {
    let Some(fn_node) = call.child_by_field_name("function") else {
        return false;
    };
    let mut current = fn_node;
    loop {
        match current.kind() {
            "identifier" => {
                let name = &source[current.byte_range()];
                return matches!(
                    name,
                    "describe" | "it" | "test" | "suite" | "context" | "fit" | "fdescribe"
                );
            }
            "member_expression" => {
                let Some(obj) = current.child_by_field_name("object") else {
                    return false;
                };
                current = obj;
            }
            "call_expression" => {
                // Patterns like `test.each(table)(name, fn)` — recurse
                // into the inner call's callee.
                let Some(inner_fn) = current.child_by_field_name("function") else {
                    return false;
                };
                current = inner_fn;
            }
            _ => return false,
        }
    }
}

/// The body of the last function/arrow argument to a test call —
/// `it('x', () => { … })`'s `{ … }`. Returns `None` for `it.todo('x')`
/// and other call shapes without a callback.
fn js_test_callback_body<'a>(call: &Node<'a>) -> Option<Node<'a>> {
    let args = call.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    let mut last: Option<Node<'a>> = None;
    for arg in args.named_children(&mut cursor) {
        if matches!(arg.kind(), "arrow_function" | "function_expression") {
            if let Some(body) = arg.child_by_field_name("body") {
                last = Some(body);
            }
        }
    }
    last
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
    fn includes_helpers_inside_cfg_test_mod() {
        let src = r#"
pub fn from_parsed(x: i32) -> i32 { x }

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> i32 { from_parsed(42) }

    #[test]
    fn it_works() {
        let _ = build();
    }
}
"#;
        let w = witness(src);
        assert!(test_haystack_mentions(&w.test_haystack, "from_parsed"));
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

    fn js_witness(src: &str, lang: Language) -> TestWitness {
        let parsed = parse(src, lang).unwrap();
        js_test_witness(&parsed)
    }

    #[test]
    fn js_collects_exported_functions_and_arrows() {
        let src = "export function add(a, b) { return a + b; }\n\
                   export const square = (x) => x * x;\n\
                   function helper() {}\n";
        let w = js_witness(src, Language::JavaScript);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["add", "square"]);
    }

    #[test]
    fn js_skips_non_exported_top_level_functions() {
        let w = js_witness("function privateFn() {}\n", Language::JavaScript);
        assert!(w.pub_fns.is_empty(), "{:?}", w.pub_fns);
    }

    #[test]
    fn js_collects_default_named_export() {
        let w = js_witness(
            "export default function greet() {}\n",
            Language::JavaScript,
        );
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["greet"]);
    }

    #[test]
    fn js_collects_exported_classes() {
        let w = js_witness("export class Box { area() {} }\n", Language::JavaScript);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["Box"]);
    }

    #[test]
    fn js_haystack_picks_up_describe_it_test() {
        let src = "export function add(a, b) { return a + b; }\n\
                   export function untested() {}\n\
                   describe('add', () => {\n\
                     it('sums', () => { expect(add(1, 2)).toBe(3); });\n\
                   });\n";
        let w = js_witness(src, Language::JavaScript);
        assert!(test_haystack_mentions(&w.test_haystack, "add"));
        assert!(!test_haystack_mentions(&w.test_haystack, "untested"));
    }

    #[test]
    fn js_haystack_picks_up_test_each_chain() {
        let src = "export const fmt = (x) => String(x);\n\
                   test.each([[1], [2]])('fmt %p', (n) => { fmt(n); });\n";
        let w = js_witness(src, Language::JavaScript);
        assert!(test_haystack_mentions(&w.test_haystack, "fmt"));
    }

    #[test]
    fn ts_collects_exported_function_with_types() {
        let src = "export function add(a: number, b: number): number { return a + b; }\n\
                   export type Adder = (a: number, b: number) => number;\n\
                   export interface Shape { area(): number }\n";
        let w = js_witness(src, Language::TypeScript);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["add"]);
    }

    #[test]
    fn tsx_collects_exported_components() {
        let src = "export const Hello = ({ name }: { name: string }) => <div>Hi {name}</div>;\n\
                   export function App() { return <Hello name=\"world\" />; }\n";
        let w = js_witness(src, Language::Tsx);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"Hello"));
        assert!(names.contains(&"App"));
    }

    #[test]
    fn js_witness_empty_for_non_js_languages() {
        let parsed = parse("pub fn x() {}\n", Language::Rust).unwrap();
        let w = js_test_witness(&parsed);
        assert!(w.pub_fns.is_empty());
        assert!(w.test_haystack.is_empty());
    }

    #[test]
    fn dispatch_picks_correct_witness() {
        let rs = parse("pub fn r() {}\n", Language::Rust).unwrap();
        assert_eq!(test_witness(&rs).pub_fns.len(), 1);
        let js = parse("export function j() {}\n", Language::JavaScript).unwrap();
        assert_eq!(test_witness(&js).pub_fns.len(), 1);
        let py = parse("def p(): pass\n", Language::Python).unwrap();
        assert!(test_witness(&py).pub_fns.is_empty());
    }
}
