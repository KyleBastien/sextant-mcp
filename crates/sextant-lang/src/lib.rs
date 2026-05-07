//! Tree-sitter parsing and language-specific queries.
//!
//! For M2 we only support Rust. The public API is small on purpose:
//! callers ask for a `ParsedFile` and then derive specific structures
//! (e.g. `function_ranges`) without ever touching tree-sitter directly.

use thiserror::Error;
use tree_sitter::{Node, Parser, Query, QueryCursor, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
}

impl Language {
    pub fn from_hint(hint: &str) -> Option<Self> {
        match hint {
            "rust" => Some(Language::Rust),
            _ => None,
        }
    }

    fn ts_language(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::language(),
        }
    }
}

#[derive(Debug, Error)]
pub enum LangError {
    #[error("tree-sitter: {0}")]
    Ts(String),
}

pub struct ParsedFile {
    pub language: Language,
    pub source: String,
    pub tree: Tree,
}

pub fn parse(source: impl Into<String>, language: Language) -> Result<ParsedFile, LangError> {
    let source = source.into();
    let mut parser = Parser::new();
    parser
        .set_language(&language.ts_language())
        .map_err(|e| LangError::Ts(e.to_string()))?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| LangError::Ts("parse returned None".into()))?;
    Ok(ParsedFile {
        language,
        source,
        tree,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRange {
    pub name: String,
    /// 1-based inclusive line where the function declaration begins.
    pub start_line: u32,
    /// 1-based inclusive line where the function ends.
    pub end_line: u32,
    pub param_count: u32,
}

impl FunctionRange {
    pub fn line_count(&self) -> u32 {
        self.end_line - self.start_line + 1
    }
}

const RUST_FN_QUERY: &str = r#"
(function_item
  name: (identifier) @fn.name
  parameters: (parameters) @fn.params) @fn.def
"#;

/// Extract function ranges from a parsed file. Currently only `function_item`
/// in Rust is recognized (covers free functions, impl methods, trait
/// implementations). Trait *signatures* in trait declarations are excluded
/// because they are `function_signature_item`, not `function_item`.
pub fn function_ranges(parsed: &ParsedFile) -> Result<Vec<FunctionRange>, LangError> {
    match parsed.language {
        Language::Rust => rust_function_ranges(parsed),
    }
}

fn rust_function_ranges(parsed: &ParsedFile) -> Result<Vec<FunctionRange>, LangError> {
    let lang = Language::Rust.ts_language();
    let query = Query::new(&lang, RUST_FN_QUERY).map_err(|e| LangError::Ts(e.to_string()))?;
    let idx_def = query
        .capture_index_for_name("fn.def")
        .ok_or_else(|| LangError::Ts("missing capture: fn.def".into()))?;
    let idx_name = query
        .capture_index_for_name("fn.name")
        .ok_or_else(|| LangError::Ts("missing capture: fn.name".into()))?;
    let idx_params = query
        .capture_index_for_name("fn.params")
        .ok_or_else(|| LangError::Ts("missing capture: fn.params".into()))?;

    let mut cursor = QueryCursor::new();
    let mut out = Vec::new();
    for m in cursor.matches(&query, parsed.tree.root_node(), parsed.source.as_bytes()) {
        let def = capture(&m, idx_def);
        let name_node = capture(&m, idx_name);
        let params_node = capture(&m, idx_params);
        let (Some(def), Some(name_node), Some(params_node)) = (def, name_node, params_node) else {
            continue;
        };

        let name = node_text(&name_node, &parsed.source).to_string();
        let start_line = (def.start_position().row as u32) + 1;
        let end_line = (def.end_position().row as u32) + 1;
        let param_count = count_named_children(&params_node);
        out.push(FunctionRange {
            name,
            start_line,
            end_line,
            param_count,
        });
    }
    out.sort_by_key(|f| f.start_line);
    Ok(out)
}

fn capture<'a>(m: &tree_sitter::QueryMatch<'a, 'a>, idx: u32) -> Option<Node<'a>> {
    m.captures.iter().find(|c| c.index == idx).map(|c| c.node)
}

fn node_text<'a>(node: &Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

/// Count named (non-anonymous) children — this naturally excludes
/// punctuation (`(`, `,`, `)`) and yields the parameter count.
fn count_named_children(parent: &Node<'_>) -> u32 {
    let mut walker = parent.walk();
    let mut n = 0u32;
    for child in parent.named_children(&mut walker) {
        // Skip line/block comments inside the parameter list.
        if child.kind() == "line_comment" || child.kind() == "block_comment" {
            continue;
        }
        n += 1;
    }
    n
}

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
        assert_eq!(fns[0].param_count, 0);
        assert_eq!(fns[0].start_line, 1);
        assert_eq!(fns[0].end_line, 1);

        assert_eq!(fns[1].name, "two");
        assert_eq!(fns[1].param_count, 2);
        assert_eq!(fns[1].start_line, 3);
        assert_eq!(fns[1].end_line, 5);
    }

    #[test]
    fn function_ranges_methods_and_self() {
        let src = "impl S {\n    fn m(&self, x: i32) {}\n    fn n(&mut self) {}\n}\n";
        let parsed = parse(src, Language::Rust).unwrap();
        let fns = function_ranges(&parsed).unwrap();
        assert_eq!(fns.len(), 2);
        // `self` parameters count toward the parameter total — that matches
        // how a human would describe the function's signature size.
        assert_eq!(fns[0].name, "m");
        assert_eq!(fns[0].param_count, 2);
        assert_eq!(fns[1].name, "n");
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
    fn language_from_hint() {
        assert_eq!(Language::from_hint("rust"), Some(Language::Rust));
        assert_eq!(Language::from_hint("python"), None);
    }
}
