//! Convert a Sextant unified-diff patch into LSP `TextEdit`s.
//!
//! We only ever emit patches we generated ourselves (regex `replacement`,
//! pub_fn_test stub append, LLM-rule patches, LLM-synthesis patches), so
//! the supported shape is narrow: `--- a/<path>` / `+++ b/<path>` headers
//! followed by one or more `@@ -<old>,<n> +<new>,<m> @@` hunks containing
//! `-`/`+`/` ` lines. A failed parse drops the action — it does not
//! corrupt the buffer.

use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// Parse a unified diff and emit the LSP edits that, applied to the file
/// described by the diff's old side, produce the new side.
pub(crate) fn patch_to_edits(patch: &str) -> Option<Vec<TextEdit>> {
    let mut edits = Vec::new();
    let mut lines = patch.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("--- ") || line.starts_with("+++ ") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            let old_start = parse_hunk_old_start(rest)?;
            consume_hunk(old_start, &mut lines, &mut edits)?;
        }
    }
    if edits.is_empty() {
        None
    } else {
        Some(edits)
    }
}

/// Walk one hunk's body until the next hunk header (or EOF), pushing
/// `TextEdit`s into `edits`. Returns `None` on malformed content so the
/// whole patch is dropped — the editor falls back to its other actions.
fn consume_hunk<'a, I>(
    start: u32,
    lines: &mut std::iter::Peekable<I>,
    edits: &mut Vec<TextEdit>,
) -> Option<()>
where
    I: Iterator<Item = &'a str>,
{
    let mut state = HunkState::new(start);
    while let Some(peek) = lines.peek() {
        if peek.starts_with("@@ ") || peek.starts_with("--- ") || peek.starts_with("+++ ") {
            break;
        }
        let body = lines.next().unwrap();
        state.consume(body, edits)?;
    }
    state.flush(edits);
    Some(())
}

struct HunkState {
    old_line: u32,
    deleted: Vec<String>,
    inserted: Vec<String>,
    consumed_old: u32,
}

impl HunkState {
    fn new(old_start: u32) -> Self {
        Self {
            old_line: old_start,
            deleted: Vec::new(),
            inserted: Vec::new(),
            consumed_old: 0,
        }
    }

    /// Apply one hunk-body line. Returns `None` for an unrecognized
    /// prefix — callers treat that as a hard parse error.
    fn consume(&mut self, body: &str, edits: &mut Vec<TextEdit>) -> Option<()> {
        if let Some(stripped) = body.strip_prefix('-') {
            self.deleted.push(stripped.to_string());
            self.consumed_old += 1;
            return Some(());
        }
        if let Some(stripped) = body.strip_prefix('+') {
            self.inserted.push(stripped.to_string());
            return Some(());
        }
        if body.strip_prefix(' ').is_some() {
            self.flush_pending(edits);
            self.old_line += 1;
            return Some(());
        }
        if body.starts_with("\\ No newline") {
            // SARIF-friendly note from `create_file_diff`. Ignored at edit time.
            return Some(());
        }
        None
    }

    /// Push any pending replacement from before a context line.
    fn flush_pending(&mut self, edits: &mut Vec<TextEdit>) {
        if self.deleted.is_empty() && self.inserted.is_empty() {
            return;
        }
        edits.push(make_edit(self.old_line, &self.deleted, &self.inserted));
        self.old_line += self.consumed_old;
        self.consumed_old = 0;
        self.deleted.clear();
        self.inserted.clear();
    }

    /// Push the trailing replacement when the hunk ends without a context.
    fn flush(self, edits: &mut Vec<TextEdit>) {
        if self.deleted.is_empty() && self.inserted.is_empty() {
            return;
        }
        edits.push(make_edit(self.old_line, &self.deleted, &self.inserted));
    }
}

fn parse_hunk_old_start(rest: &str) -> Option<u32> {
    // shape: `-<old>,<n> +<new>,<m> @@[ optional context]`
    let body = rest.split_once("@@").map(|(b, _)| b).unwrap_or(rest);
    let mut parts = body.split_whitespace();
    let old = parts.next()?;
    let _new = parts.next()?;
    let old = old.strip_prefix('-')?;
    old.split(',').next()?.parse().ok()
}

/// Replace `deleted.len()` lines starting at `old_start` (1-indexed) with
/// `inserted`. Append-style hunks (`@@ -N,0 +N+1,K @@`) come through with
/// `deleted` empty and `old_start` pointing at the line after which to
/// insert; LSP `Range` semantics handle that uniformly when start == end.
fn make_edit(old_start: u32, deleted: &[String], inserted: &[String]) -> TextEdit {
    let start_line = old_start.saturating_sub(1);
    let end_line = start_line + deleted.len() as u32;
    let mut new_text = inserted.join("\n");
    if !inserted.is_empty() && !deleted.is_empty() {
        // Replacing complete lines: keep the trailing newline so the
        // following line stays on its own row.
        new_text.push('\n');
    } else if !inserted.is_empty() {
        // Pure insert: deletions are empty, so range is collapsed. Still
        // need a trailing newline to keep one-line-per-row semantics.
        new_text.push('\n');
    }
    TextEdit {
        range: Range {
            start: Position {
                line: start_line,
                character: 0,
            },
            end: Position {
                line: end_line,
                character: 0,
            },
        },
        new_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_line_replacement() {
        let patch = "--- a/a.rs\n+++ b/a.rs\n@@ -2,1 +2,1 @@\n-old line\n+new line\n";
        let edits = patch_to_edits(patch).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range.start.line, 1);
        assert_eq!(edits[0].range.end.line, 2);
        assert_eq!(edits[0].new_text, "new line\n");
    }

    #[test]
    fn parses_pure_append_hunk() {
        let patch = "--- a/a.rs\n+++ b/a.rs\n@@ -3,0 +4,2 @@\n+gamma\n+delta\n";
        let edits = patch_to_edits(patch).unwrap();
        assert_eq!(edits.len(), 1);
        // Insert at line 3 (0-indexed 2), no deletion: start == end.
        assert_eq!(edits[0].range.start.line, 2);
        assert_eq!(edits[0].range.end.line, 2);
        assert!(edits[0].new_text.contains("gamma"));
        assert!(edits[0].new_text.contains("delta"));
    }

    #[test]
    fn returns_none_for_empty_patch() {
        assert!(patch_to_edits("").is_none());
        assert!(patch_to_edits("--- a/x\n+++ b/x\n").is_none());
    }

    #[test]
    fn ignores_no_newline_marker() {
        let patch =
            "--- a/a.rs\n+++ b/a.rs\n@@ -1,0 +2,1 @@\n\\ No newline at end of file\n+extra\n";
        let edits = patch_to_edits(patch).unwrap();
        assert_eq!(edits.len(), 1);
        assert!(edits[0].new_text.contains("extra"));
    }
}
