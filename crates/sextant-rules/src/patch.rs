//! Minimal unified-diff helpers used by native patch generators.
//!
//! Sextant doesn't depend on a diff library: the patches we produce here
//! are line-granular substitutions and append-at-end inserts, both of
//! which fit in a few lines of hand-rolled code. Keeping this in-tree
//! means downstream consumers (CLI renderers, the LSP code-action
//! handler) get a consistent shape — `--- a/<path>` / `+++ b/<path>` with
//! one or more `@@` hunks — without having to negotiate library output.

use std::path::Path;

/// Build a unified diff for replacing `line_no`'s content with `new_line`.
/// Returns `None` when `line_no` is out of range. The diff uses the file
/// path verbatim (no `a/` `b/` rewrite); we trust the caller to have
/// already resolved it relative to the repo root.
pub fn replace_line_diff(
    path: &Path,
    contents: &str,
    line_no: u32,
    new_line: &str,
) -> Option<String> {
    let lines: Vec<&str> = contents.split_inclusive('\n').collect();
    let idx = line_no.checked_sub(1)? as usize;
    let original = lines.get(idx)?;
    // Preserve the original line's trailing newline (or absence thereof)
    // so the rebuilt file's final byte doesn't drift.
    let trailing = if original.ends_with('\n') { "\n" } else { "" };
    let stripped = original.strip_suffix('\n').unwrap_or(original);
    if stripped == new_line {
        return None;
    }
    let header = unified_header(path);
    let hunk = format!(
        "@@ -{ln},1 +{ln},1 @@\n-{old}\n+{new}{trail}",
        ln = line_no,
        old = stripped,
        new = new_line,
        trail = trailing,
    );
    Some(format!("{header}{hunk}"))
}

/// Build a unified diff that creates a new file at `path` with `contents`.
/// The contents should end with `\n` so the resulting file ends with a
/// newline. Uses `/dev/null` for the old path per `git diff` convention.
pub fn create_file_diff(path: &Path, contents: &str) -> String {
    let p = path.display();
    let header = format!("--- /dev/null\n+++ b/{p}\n");
    let lines: Vec<&str> = contents.split_inclusive('\n').collect();
    let trailing_newline = contents.ends_with('\n') || contents.is_empty();
    let context = if trailing_newline {
        ""
    } else {
        "\\ No newline at end of file\n"
    };
    let mut hunk_body = String::new();
    for line in &lines {
        let stripped = line.strip_suffix('\n').unwrap_or(line);
        hunk_body.push('+');
        hunk_body.push_str(stripped);
        if line.ends_with('\n') {
            hunk_body.push('\n');
        }
    }
    let added_count = lines.len() as u32;
    format!("{header}@@ -0,0 +1,{added_count} @@\n{hunk_body}{context}")
}

fn unified_header(path: &Path) -> String {
    let p = path.display();
    format!("--- a/{p}\n+++ b/{p}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn replace_line_diff_emits_one_hunk() {
        let p = PathBuf::from("a.rs");
        let src = "alpha\nbeta\ngamma\n";
        let d = replace_line_diff(&p, src, 2, "BETA").unwrap();
        assert!(d.starts_with("--- a/a.rs\n+++ b/a.rs\n"));
        assert!(d.contains("@@ -2,1 +2,1 @@\n-beta\n+BETA\n"));
    }

    #[test]
    fn replace_line_diff_returns_none_on_no_op() {
        let p = PathBuf::from("a.rs");
        let src = "alpha\nbeta\n";
        assert!(replace_line_diff(&p, src, 1, "alpha").is_none());
    }

    #[test]
    fn replace_line_diff_returns_none_when_out_of_range() {
        let p = PathBuf::from("a.rs");
        let src = "alpha\n";
        assert!(replace_line_diff(&p, src, 5, "x").is_none());
        assert!(replace_line_diff(&p, src, 0, "x").is_none());
    }

    #[test]
    fn create_file_diff_writes_new_file_hunk() {
        let p = PathBuf::from("a.rs");
        let d = create_file_diff(&p, "alpha\nbeta\n");
        assert!(d.starts_with("--- /dev/null\n+++ b/a.rs\n"));
        assert!(d.contains("@@ -0,0 +1,2 @@\n+alpha\n+beta\n"));
    }

    #[test]
    fn create_file_diff_handles_missing_final_newline() {
        let p = PathBuf::from("a.rs");
        let d = create_file_diff(&p, "alpha");
        assert!(d.contains("\\ No newline at end of file"));
    }

}
