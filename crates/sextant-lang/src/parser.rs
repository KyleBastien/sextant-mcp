//! Common parser plumbing: language registry, parsed-file type, and the
//! `parse()` entry point.

use thiserror::Error;
use tree_sitter::{Parser, Tree};

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

    pub(crate) fn ts_language(self) -> tree_sitter::Language {
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
