use std::path::{Path, PathBuf};

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
        match self.extension()? {
            "rs" => Some("rust"),
            "ts" => Some("typescript"),
            "tsx" => Some("typescript"),
            "js" | "mjs" | "cjs" => Some("javascript"),
            "jsx" => Some("javascript"),
            "py" => Some("python"),
            _ => None,
        }
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
        assert_eq!(
            SourceFile::new("a.tsx", "").language_hint(),
            Some("typescript")
        );
        assert_eq!(SourceFile::new("a.unknown", "").language_hint(), None);
    }
}
