//! Tree-sitter parsing and language-specific queries.
//!
//! Public API: callers ask for a `ParsedFile` and then derive specific
//! structures (`function_ranges`, `function_complexity`) without ever
//! touching tree-sitter directly. Supported languages: Rust and Python.

mod clones;
mod complexity;
mod parser;
mod ranges;
mod test_witness;

pub use clones::{find_clones, ClonePair, CloneSpan};
pub use complexity::{function_complexity, FunctionComplexity};
pub use parser::{parse, LangError, Language, ParsedFile};
pub use ranges::{function_ranges, FunctionRange};
pub use test_witness::{rust_test_witness, test_haystack_mentions, PubFnInfo, TestWitness};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_ranges_rust_basic() {
        let src = "fn one() {}\n\nfn two(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
        let parsed = parse(src, Language::Rust).unwrap();
        let fns = function_ranges(&parsed).unwrap();
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "one");
        assert_eq!(fns[1].name, "two");
        assert_eq!(fns[1].param_count, 2);
    }

    #[test]
    fn function_ranges_methods_and_self() {
        let src = "impl S {\n    fn m(&self, x: i32) {}\n    fn n(&mut self) {}\n}\n";
        let parsed = parse(src, Language::Rust).unwrap();
        let fns = function_ranges(&parsed).unwrap();
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].param_count, 2);
        assert_eq!(fns[1].param_count, 1);
    }

    #[test]
    fn function_ranges_skip_trait_signatures() {
        let src = "trait T {\n    fn declared(&self);\n}\n\nfn impl_fn() {}\n";
        let parsed = parse(src, Language::Rust).unwrap();
        let fns = function_ranges(&parsed).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "impl_fn");
    }

    #[test]
    fn function_ranges_python_basic() {
        let src = "def alpha():\n    pass\n\ndef beta(a, b, c):\n    return a + b + c\n";
        let parsed = parse(src, Language::Python).unwrap();
        let fns = function_ranges(&parsed).unwrap();
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "alpha");
        assert_eq!(fns[1].name, "beta");
        assert_eq!(fns[1].param_count, 3);
    }

    #[test]
    fn language_from_hint() {
        assert_eq!(Language::from_hint("rust"), Some(Language::Rust));
        assert_eq!(Language::from_hint("python"), Some(Language::Python));
        assert_eq!(Language::from_hint("nope"), None);
    }

    #[test]
    fn cyclomatic_rust_simple_function() {
        let src = "fn straight() { let x = 1; let y = 2; }\n";
        let parsed = parse(src, Language::Rust).unwrap();
        let cs = function_complexity(&parsed).unwrap();
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].cyclomatic, 1, "{cs:?}");
        assert_eq!(cs[0].max_nesting, 0);
    }

    #[test]
    fn cyclomatic_rust_branching() {
        let src = r#"
fn f(x: i32) -> i32 {
    if x > 0 {
        match x {
            1 => 1,
            _ => 2,
        }
    } else {
        let mut i = 0;
        while i < 10 { i += 1; }
        for _ in 0..5 {}
        0
    }
}
"#;
        let parsed = parse(src, Language::Rust).unwrap();
        let cs = function_complexity(&parsed).unwrap();
        assert_eq!(cs.len(), 1);
        assert!(cs[0].cyclomatic >= 5, "got {}", cs[0].cyclomatic);
        assert!(cs[0].max_nesting >= 2, "got {}", cs[0].max_nesting);
    }

    #[test]
    fn cyclomatic_python_branching() {
        let src = r#"
def f(x):
    if x > 0:
        return 1
    elif x < 0:
        try:
            while x < 0:
                x += 1
            for _ in range(5):
                pass
        except Exception:
            return 0
    return 0
"#;
        let parsed = parse(src, Language::Python).unwrap();
        let cs = function_complexity(&parsed).unwrap();
        assert_eq!(cs.len(), 1);
        assert!(cs[0].cyclomatic >= 5, "got {}", cs[0].cyclomatic);
        assert!(cs[0].max_nesting >= 2, "got {}", cs[0].max_nesting);
    }
}
