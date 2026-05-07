//! Tree-sitter parsing and language-specific queries.
//!
//! Public API: callers ask for a `ParsedFile` and then derive specific
//! structures (`function_ranges`, `function_complexity`) without ever
//! touching tree-sitter directly. Supported languages: Rust and Python.

use thiserror::Error;
use tree_sitter::{Node, Parser, Query, QueryCursor, Tree, TreeCursor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
}

impl Language {
    pub fn from_hint(hint: &str) -> Option<Self> {
        match hint {
            "rust" => Some(Language::Rust),
            "python" => Some(Language::Python),
            _ => None,
        }
    }

    fn ts_language(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::language(),
            Language::Python => tree_sitter_python::language(),
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
    extract_function_ranges(parsed, query_src)
}

fn extract_function_ranges(
    parsed: &ParsedFile,
    query_src: &str,
) -> Result<Vec<FunctionRange>, LangError> {
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

// ============================================================================
// Complexity metrics
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionComplexity {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    /// McCabe cyclomatic complexity. 1 + count of branching nodes.
    pub cyclomatic: u32,
    /// Maximum depth of nested control structures within the function body.
    /// 0 = no control structures.
    pub max_nesting: u32,
}

/// Compute cyclomatic complexity and max-nesting for every top-level function
/// in the file. Functions defined inside other functions are computed
/// independently — each gets its own row.
pub fn function_complexity(parsed: &ParsedFile) -> Result<Vec<FunctionComplexity>, LangError> {
    let ranges = function_ranges(parsed)?;
    let mut out = Vec::with_capacity(ranges.len());
    for r in ranges {
        // Locate the body node within the function's byte range. We use
        // the def node, which encloses the whole function, and walk it.
        let Some(def_node) = find_def_at_line(&parsed.tree, r.start_line, parsed.language) else {
            continue;
        };
        let cyclomatic = 1 + count_branches(def_node, parsed.language);
        let max_nesting = max_depth(def_node, parsed.language);
        out.push(FunctionComplexity {
            name: r.name,
            start_line: r.start_line,
            end_line: r.end_line,
            cyclomatic,
            max_nesting,
        });
    }
    Ok(out)
}

fn find_def_at_line(tree: &Tree, line: u32, language: Language) -> Option<Node<'_>> {
    let target_kind = match language {
        Language::Rust => "function_item",
        Language::Python => "function_definition",
    };
    let mut cursor = tree.walk();
    find_def_recursive(&mut cursor, target_kind, line)
}

fn find_def_recursive<'tree>(
    cursor: &mut TreeCursor<'tree>,
    target_kind: &str,
    line: u32,
) -> Option<Node<'tree>> {
    let node = cursor.node();
    if node.kind() == target_kind && (node.start_position().row as u32) + 1 == line {
        return Some(node);
    }
    if cursor.goto_first_child() {
        loop {
            if let Some(found) = find_def_recursive(cursor, target_kind, line) {
                return Some(found);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    None
}

fn count_branches(root: Node<'_>, language: Language) -> u32 {
    let mut count = 0u32;
    let mut cursor = root.walk();
    walk(&mut cursor, &mut |node| {
        if is_branch(node, language) {
            count += 1;
        }
    });
    count
}

fn max_depth(root: Node<'_>, language: Language) -> u32 {
    fn recurse(cursor: &mut TreeCursor, depth: u32, max: &mut u32, lang: Language) {
        let node = cursor.node();
        let new_depth = if is_nesting_increment(&node, lang) {
            depth + 1
        } else {
            depth
        };
        if new_depth > *max {
            *max = new_depth;
        }
        if cursor.goto_first_child() {
            loop {
                recurse(cursor, new_depth, max, lang);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
    let mut max = 0u32;
    let mut cursor = root.walk();
    // Skip the function's own def node — the first nesting level should be
    // counted from inside its body.
    if cursor.goto_first_child() {
        loop {
            recurse(&mut cursor, 0, &mut max, language);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    max
}

fn walk<F: FnMut(&Node<'_>)>(cursor: &mut TreeCursor, visit: &mut F) {
    visit(&cursor.node());
    if cursor.goto_first_child() {
        loop {
            walk(cursor, visit);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn is_branch(node: &Node<'_>, language: Language) -> bool {
    match language {
        Language::Rust => matches!(
            node.kind(),
            "if_expression"
                | "match_arm"
                | "while_expression"
                | "while_let_expression"
                | "for_expression"
                | "try_expression"
        ),
        Language::Python => matches!(
            node.kind(),
            "if_statement"
                | "elif_clause"
                | "while_statement"
                | "for_statement"
                | "except_clause"
                | "conditional_expression"
        ),
    }
}

fn is_nesting_increment(node: &Node<'_>, language: Language) -> bool {
    match language {
        Language::Rust => matches!(
            node.kind(),
            "if_expression"
                | "match_expression"
                | "while_expression"
                | "while_let_expression"
                | "for_expression"
                | "loop_expression"
        ),
        Language::Python => matches!(
            node.kind(),
            "if_statement" | "while_statement" | "for_statement" | "try_statement"
        ),
    }
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
        // 1 base + 1 if + 1 match arm + 1 match arm + 1 while + 1 for = 6
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
        // 1 base + 1 if + 1 elif + 1 while + 1 for + 1 except = 6
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
