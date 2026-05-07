//! Function-range extraction (name, span, parameter count).

use tree_sitter::{Node, Query, QueryCursor};

use crate::parser::{LangError, Language, ParsedFile};

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

const PYTHON_FN_QUERY: &str = r#"
(function_definition
  name: (identifier) @fn.name
  parameters: (parameters) @fn.params) @fn.def
"#;

/// Extract function ranges from a parsed file.
pub fn function_ranges(parsed: &ParsedFile) -> Result<Vec<FunctionRange>, LangError> {
    let query_src = match parsed.language {
        Language::Rust => RUST_FN_QUERY,
        Language::Python => PYTHON_FN_QUERY,
    };
    extract(parsed, query_src)
}

fn extract(parsed: &ParsedFile, query_src: &str) -> Result<Vec<FunctionRange>, LangError> {
    let lang = parsed.language.ts_language();
    let query = Query::new(&lang, query_src).map_err(|e| LangError::Ts(e.to_string()))?;
    let idx_def = capture_index(&query, "fn.def")?;
    let idx_name = capture_index(&query, "fn.name")?;
    let idx_params = capture_index(&query, "fn.params")?;

    let mut cursor = QueryCursor::new();
    let mut out = Vec::new();
    for m in cursor.matches(&query, parsed.tree.root_node(), parsed.source.as_bytes()) {
        let def = capture(&m, idx_def);
        let name_node = capture(&m, idx_name);
        let params_node = capture(&m, idx_params);
        let (Some(def), Some(name_node), Some(params_node)) = (def, name_node, params_node) else {
            continue;
        };
        out.push(FunctionRange {
            name: node_text(&name_node, &parsed.source).to_string(),
            start_line: (def.start_position().row as u32) + 1,
            end_line: (def.end_position().row as u32) + 1,
            param_count: count_named_children(&params_node),
        });
    }
    out.sort_by_key(|f| f.start_line);
    Ok(out)
}

fn capture_index(query: &Query, name: &str) -> Result<u32, LangError> {
    query
        .capture_index_for_name(name)
        .ok_or_else(|| LangError::Ts(format!("missing capture: {name}")))
}

fn capture<'a>(m: &tree_sitter::QueryMatch<'a, 'a>, idx: u32) -> Option<Node<'a>> {
    m.captures.iter().find(|c| c.index == idx).map(|c| c.node)
}

fn node_text<'a>(node: &Node<'_>, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn count_named_children(parent: &Node<'_>) -> u32 {
    let mut walker = parent.walk();
    let mut n = 0u32;
    for child in parent.named_children(&mut walker) {
        if child.kind() == "line_comment"
            || child.kind() == "block_comment"
            || child.kind() == "comment"
        {
            continue;
        }
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn ranges(src: &str, lang: Language) -> Vec<FunctionRange> {
        function_ranges(&parse(src, lang).unwrap()).unwrap()
    }

    #[test]
    fn rust_basic_two_top_level_fns() {
        let fns = ranges(
            "fn one() {}\n\nfn two(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
            Language::Rust,
        );
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "one");
        assert_eq!(fns[1].name, "two");
        assert_eq!(fns[1].param_count, 2);
    }

    #[test]
    fn methods_count_self_as_a_param() {
        let fns = ranges(
            "impl S {\n    fn m(&self, x: i32) {}\n    fn n(&mut self) {}\n}\n",
            Language::Rust,
        );
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].param_count, 2);
        assert_eq!(fns[1].param_count, 1);
    }

    #[test]
    fn skips_trait_signatures() {
        let fns = ranges(
            "trait T {\n    fn declared(&self);\n}\n\nfn impl_fn() {}\n",
            Language::Rust,
        );
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "impl_fn");
    }

    #[test]
    fn python_basic_two_top_level_fns() {
        let fns = ranges(
            "def alpha():\n    pass\n\ndef beta(a, b, c):\n    return a + b + c\n",
            Language::Python,
        );
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "alpha");
        assert_eq!(fns[1].name, "beta");
        assert_eq!(fns[1].param_count, 3);
    }

    #[test]
    fn line_count_is_inclusive() {
        let r = FunctionRange {
            name: "x".into(),
            start_line: 5,
            end_line: 8,
            param_count: 0,
        };
        assert_eq!(r.line_count(), 4);
    }
}
