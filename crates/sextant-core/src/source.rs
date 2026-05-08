use std::path::{Path, PathBuf};

/// File-extension → tree-sitter language name. Order doesn't matter; the
/// table is searched linearly and entries are mutually exclusive.
const EXTENSION_TO_LANGUAGE: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("py", "python"),
    ("go", "go"),
    ("java", "java"),
    ("ts", "typescript"),
    // `.tsx` needs the TSX grammar; the plain TS grammar can't parse JSX.
    ("tsx", "tsx"),
    ("js", "javascript"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    // tree-sitter-javascript handles both .js and .jsx in one grammar.
    ("jsx", "javascript"),
];

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub contents: String,
}

impl SourceFile {
    pub fn new(path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }

    pub fn line_count(&self) -> usize {
        if self.contents.is_empty() {
            return 0;
        }
        let trailing_newline = self.contents.ends_with('\n');
        let raw = self.contents.split('\n').count();
        if trailing_newline {
            raw - 1
        } else {
            raw
        }
    }

    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    pub fn language_hint(&self) -> Option<&'static str> {
        let ext = self.extension()?;
        EXTENSION_TO_LANGUAGE
            .iter()
            .find(|(e, _)| *e == ext)
            .map(|(_, lang)| *lang)
    }

    pub fn relative_to(&self, root: &Path) -> PathBuf {
        self.path
            .strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| self.path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_count_handles_trailing_newline() {
        assert_eq!(SourceFile::new("a", "").line_count(), 0);
        assert_eq!(SourceFile::new("a", "x").line_count(), 1);
        assert_eq!(SourceFile::new("a", "x\n").line_count(), 1);
        assert_eq!(SourceFile::new("a", "x\ny").line_count(), 2);
        assert_eq!(SourceFile::new("a", "x\ny\n").line_count(), 2);
    }

    #[test]
    fn language_hint_from_extension() {
        assert_eq!(SourceFile::new("a.rs", "").language_hint(), Some("rust"));
        assert_eq!(SourceFile::new("a.go", "").language_hint(), Some("go"));
        assert_eq!(
            SourceFile::new("a.ts", "").language_hint(),
            Some("typescript")
        );
        assert_eq!(SourceFile::new("a.tsx", "").language_hint(), Some("tsx"));
        assert_eq!(
            SourceFile::new("a.jsx", "").language_hint(),
            Some("javascript")
        );
        assert_eq!(SourceFile::new("a.unknown", "").language_hint(), None);
    }

    #[test]
    fn extension_returns_lowercase_suffix() {
        assert_eq!(SourceFile::new("a.rs", "").extension(), Some("rs"));
        assert_eq!(SourceFile::new("noext", "").extension(), None);
    }

    #[test]
    fn relative_to_strips_root_prefix() {
        let f = SourceFile::new("/tmp/proj/src/lib.rs", "");
        let r = f.relative_to(Path::new("/tmp/proj"));
        assert_eq!(r, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn relative_to_returns_full_path_when_outside_root() {
        let f = SourceFile::new("/elsewhere/x.rs", "");
        let r = f.relative_to(Path::new("/tmp/proj"));
        assert_eq!(r, PathBuf::from("/elsewhere/x.rs"));
    }
}
