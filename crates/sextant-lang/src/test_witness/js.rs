//! JavaScript / TypeScript / TSX witness.
//!
//! Mirrors the contract of the Rust witness: `pub_fns` holds the exported
//! declarations that count as the file's public surface, and
//! `test_haystack` is the joined source of every in-file test callback
//! body. Only the `export` keyword counts as "public" — bare top-level
//! `function f` is module-private (analogous to Rust without `pub`) and
//! is skipped. Type-only exports (`export type`, `export interface`)
//! are skipped too.
//!
//! The haystack is built from `describe` / `it` / `test` / `suite` /
//! `context` / `fit` / `fdescribe` callback bodies, including chained
//! forms like `it.skip(…)`, `describe.only(…)`, and
//! `test.each(table)(name, fn)`. That covers Jest, Vitest, Mocha, and
//! Jasmine in-file test patterns.

use tree_sitter::Node;

use super::{PubFnInfo, TestWitness};
use crate::parser::{Language, ParsedFile};

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
    walk(parsed.tree.root_node(), &mut state);
    state.out
}

struct JsState<'a> {
    source: &'a str,
    out: TestWitness,
}

fn walk(node: Node<'_>, state: &mut JsState<'_>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "export_statement" => {
                collect_exports(child, state);
                // Test calls inside an export are vanishingly rare, but
                // recursing keeps the haystack consistent if they occur.
                walk(child, state);
            }
            "call_expression" if is_test_call(&child, state.source) => {
                if let Some(body) = test_callback_body(&child) {
                    state
                        .out
                        .test_haystack
                        .push_str(&state.source[body.byte_range()]);
                    state.out.test_haystack.push('\n');
                }
                // The body's text already covers nested test calls — no
                // need to descend.
            }
            _ => walk(child, state),
        }
    }
}

fn collect_exports(export_node: Node<'_>, state: &mut JsState<'_>) {
    let mut cursor = export_node.walk();
    for child in export_node.children(&mut cursor) {
        match child.kind() {
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration" => push_named_def(&child, state),
            "lexical_declaration" | "variable_declaration" => {
                collect_lexical_declarators(&child, state);
            }
            _ => {}
        }
    }
}

fn push_named_def(node: &Node<'_>, state: &mut JsState<'_>) {
    let Some(name) = node.child_by_field_name("name") else {
        return;
    };
    state.out.pub_fns.push(PubFnInfo {
        name: state.source[name.byte_range()].to_string(),
        start_line: (node.start_position().row as u32) + 1,
        end_line: (node.end_position().row as u32) + 1,
    });
}

fn collect_lexical_declarators(lex: &Node<'_>, state: &mut JsState<'_>) {
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
fn is_test_call(call: &Node<'_>, source: &str) -> bool {
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
fn test_callback_body<'a>(call: &Node<'a>) -> Option<Node<'a>> {
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
    use super::super::test_haystack_mentions;
    use super::*;
    use crate::parser::parse;

    fn js_witness(src: &str, lang: Language) -> TestWitness {
        let parsed = parse(src, lang).unwrap();
        js_test_witness(&parsed)
    }

    #[test]
    fn collects_exported_functions_and_arrows() {
        let src = "export function add(a, b) { return a + b; }\n\
                   export const square = (x) => x * x;\n\
                   function helper() {}\n";
        let w = js_witness(src, Language::JavaScript);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["add", "square"]);
    }

    #[test]
    fn skips_non_exported_top_level_functions() {
        let w = js_witness("function privateFn() {}\n", Language::JavaScript);
        assert!(w.pub_fns.is_empty(), "{:?}", w.pub_fns);
    }

    #[test]
    fn collects_default_named_export() {
        let w = js_witness("export default function greet() {}\n", Language::JavaScript);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["greet"]);
    }

    #[test]
    fn collects_exported_classes() {
        let w = js_witness("export class Box { area() {} }\n", Language::JavaScript);
        let names: Vec<_> = w.pub_fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["Box"]);
    }

    #[test]
    fn haystack_picks_up_describe_it_test() {
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
    fn haystack_picks_up_test_each_chain() {
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
    fn empty_for_non_js_languages() {
        let parsed = parse("pub fn x() {}\n", Language::Rust).unwrap();
        let w = js_test_witness(&parsed);
        assert!(w.pub_fns.is_empty());
        assert!(w.test_haystack.is_empty());
    }
}
