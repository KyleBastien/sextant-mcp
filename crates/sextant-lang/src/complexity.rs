//! Cyclomatic complexity and max-nesting metrics over a parsed file.

use tree_sitter::{Node, Tree, TreeCursor};

use crate::parser::{LangError, Language, ParsedFile};
use crate::ranges::function_ranges;

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
        let Some(def_node) = find_def_at_line(&parsed.tree, r.start_line, parsed.language) else {
            continue;
        };
        out.push(FunctionComplexity {
            name: r.name,
            start_line: r.start_line,
            end_line: r.end_line,
            cyclomatic: 1 + count_branches(def_node, parsed.language),
            max_nesting: max_depth(def_node, parsed.language),
        });
    }
    Ok(out)
}

fn find_def_at_line(tree: &Tree, line: u32, language: Language) -> Option<Node<'_>> {
    // Some languages have more than one node kind for "a function" — Go
    // distinguishes free functions from methods on receivers, Java
    // distinguishes methods from constructors. The walker accepts any
    // of the kinds in the slice.
    let target_kinds: &[&str] = match language {
        Language::Rust => &["function_item"],
        Language::Python => &["function_definition"],
        Language::Go => &["function_declaration", "method_declaration"],
        Language::Java => &["method_declaration", "constructor_declaration"],
    };
    let mut cursor = tree.walk();
    find_def_recursive(&mut cursor, target_kinds, line)
}

fn find_def_recursive<'tree>(
    cursor: &mut TreeCursor<'tree>,
    target_kinds: &[&str],
    line: u32,
) -> Option<Node<'tree>> {
    let node = cursor.node();
    if target_kinds.contains(&node.kind()) && (node.start_position().row as u32) + 1 == line {
        return Some(node);
    }
    if cursor.goto_first_child() {
        loop {
            if let Some(found) = find_def_recursive(cursor, target_kinds, line) {
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
    let mut max = 0u32;
    let mut cursor = root.walk();
    // Skip the function's own def node — first nesting level is from inside its body.
    if cursor.goto_first_child() {
        loop {
            recurse_depth(&mut cursor, 0, &mut max, language);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    max
}

fn recurse_depth(cursor: &mut TreeCursor, depth: u32, max: &mut u32, lang: Language) {
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
            recurse_depth(cursor, new_depth, max, lang);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
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
        Language::Go => matches!(
            node.kind(),
            "if_statement"
                | "for_statement"
                | "expression_case"
                | "type_case"
                | "communication_case"
                | "select_statement"
        ),
        Language::Java => matches!(
            node.kind(),
            "if_statement"
                | "while_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "do_statement"
                | "switch_label"
                | "catch_clause"
                | "ternary_expression"
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
        Language::Go => matches!(
            node.kind(),
            "if_statement"
                | "for_statement"
                | "expression_switch_statement"
                | "type_switch_statement"
                | "select_statement"
        ),
        Language::Java => matches!(
            node.kind(),
            "if_statement"
                | "while_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "do_statement"
                | "switch_expression"
                | "switch_statement"
                | "try_statement"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn complexities(src: &str, lang: Language) -> Vec<FunctionComplexity> {
        function_complexity(&parse(src, lang).unwrap()).unwrap()
    }

    fn assert_branchy(cs: &[FunctionComplexity]) {
        assert_eq!(cs.len(), 1);
        assert!(cs[0].cyclomatic >= 5, "got {}", cs[0].cyclomatic);
        assert!(cs[0].max_nesting >= 2, "got {}", cs[0].max_nesting);
    }

    #[test]
    fn rust_simple_function_is_one() {
        let cs = complexities("fn straight() { let x = 1; let y = 2; }\n", Language::Rust);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].cyclomatic, 1, "{cs:?}");
        assert_eq!(cs[0].max_nesting, 0);
    }

    #[test]
    fn rust_branching_increments_cyclomatic_and_nesting() {
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
        assert_branchy(&complexities(src, Language::Rust));
    }

    #[test]
    fn go_branching_increments_cyclomatic_and_nesting() {
        let src = r#"
package main

func f(x int) int {
    if x > 0 {
        for i := 0; i < 5; i++ {
            switch x {
            case 1:
                return 1
            case 2:
                return 2
            }
        }
    }
    return 0
}
"#;
        assert_branchy(&complexities(src, Language::Go));
    }

    #[test]
    fn java_branching_increments_cyclomatic_and_nesting() {
        let src = r#"
class C {
    int f(int x) {
        if (x > 0) {
            for (int i = 0; i < 5; i++) {
                if (i == x) {
                    try {
                        return 1;
                    } catch (Exception e) {
                        return 0;
                    }
                }
            }
        }
        return 0;
    }
}
"#;
        assert_branchy(&complexities(src, Language::Java));
    }

    #[test]
    fn python_branching_increments_cyclomatic_and_nesting() {
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
        assert_branchy(&complexities(src, Language::Python));
    }
}
