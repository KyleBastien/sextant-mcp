//! Common parser plumbing: language registry, parsed-file type, and the
//! `parse()` entry point.

use thiserror::Error;
use tree_sitter::{Parser, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    Go,
    Java,
}

impl Language {
    pub fn from_hint(hint: &str) -> Option<Self> {
        match hint {
            "rust" => Some(Language::Rust),
            "python" => Some(Language::Python),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            _ => None,
        }
    }

    pub(crate) fn ts_language(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::language(),
            Language::Python => tree_sitter_python::language(),
            Language::Go => tree_sitter_go::language(),
            Language::Java => tree_sitter_java::language(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_hint_recognizes_supported_languages() {
        assert_eq!(Language::from_hint("rust"), Some(Language::Rust));
        assert_eq!(Language::from_hint("python"), Some(Language::Python));
        assert_eq!(Language::from_hint("nope"), None);
    }

    #[test]
    fn parse_round_trips_source_and_language() {
        let src = "fn x() {}\n";
        let p = parse(src, Language::Rust).unwrap();
        assert_eq!(p.language, Language::Rust);
        assert_eq!(p.source, src);
        assert_eq!(p.tree.root_node().kind(), "source_file");
    }
}
