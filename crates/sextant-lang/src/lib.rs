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
