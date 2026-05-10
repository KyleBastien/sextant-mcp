//! Repo-root resolution for the LSP backend.
//!
//! The LSP wire format gives us either workspace folders (when the editor
//! has an open project) or only document URIs (single-file mode). Sextant
//! always grades against a repo root containing `.sextant/` or `.git/`, so
//! we reduce both inputs to a single `Option<PathBuf>` here.

use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::WorkspaceFolder;
use url::Url;

/// Walk up from `start` looking for `.sextant/` then `.git/`. Returns the
/// first ancestor that contains either marker.
pub(crate) fn walk_up_for_marker(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".sextant").is_dir() || ancestor.join(".git").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Pick the first workspace folder whose URI converts to a filesystem path,
/// fall back to walking up from `fallback_doc`.
pub(crate) fn resolve_repo_root(
    folders: Option<&[WorkspaceFolder]>,
    fallback_doc: Option<&Path>,
) -> Option<PathBuf> {
    folders
        .into_iter()
        .flatten()
        .filter_map(|f| url_to_path(&f.uri))
        .map(|p| walk_up_for_marker(&p).unwrap_or(p))
        .next()
        .or_else(|| fallback_doc.and_then(walk_up_for_marker))
}

pub(crate) fn url_to_path(uri: &Url) -> Option<PathBuf> {
    uri.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn assert_marker_found_from(marker: &str, child: &str) {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(marker)).unwrap();
        std::fs::create_dir_all(root.join(child)).unwrap();
        let found = walk_up_for_marker(&root.join(child)).expect("found");
        assert_eq!(found, root);
    }

    #[test]
    fn walk_up_finds_sextant_marker() {
        assert_marker_found_from(".sextant", "a/b/c");
    }

    #[test]
    fn walk_up_finds_git_marker() {
        assert_marker_found_from(".git", "a");
    }

    #[test]
    fn walk_up_returns_none_when_no_marker() {
        let dir = tempdir().unwrap();
        assert!(walk_up_for_marker(dir.path()).is_none());
    }

    #[test]
    fn resolve_repo_root_walks_up_from_fallback() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".sextant")).unwrap();
        let nested = root.join("src/lib.rs");
        let found = resolve_repo_root(None, Some(&nested)).expect("found");
        assert_eq!(found, root);
    }
}
