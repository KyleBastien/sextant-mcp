//! `Finding` → LSP `Diagnostic` conversion.

use sextant_core::{Finding, Severity};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

/// Convert a Sextant finding into an LSP diagnostic. `text` is the live
/// buffer for the file (used to compute the end column in UTF-16 units;
/// LSP's default position encoding). When `text` is `None`, the diagnostic
/// extends to a sentinel large column so editors render the squiggle to
/// end-of-line.
pub(crate) fn finding_to_diagnostic(finding: &Finding, text: Option<&str>) -> Diagnostic {
    let (start_line, end_line) = match (finding.line, finding.end_line) {
        (Some(s), Some(e)) => (s.saturating_sub(1), e.saturating_sub(1)),
        (Some(s), None) => (s.saturating_sub(1), s.saturating_sub(1)),
        (None, _) => (0, 0),
    };
    let end_col = end_of_line_utf16(text, end_line).unwrap_or(u32::MAX / 2);
    Diagnostic {
        range: Range {
            start: Position {
                line: start_line,
                character: 0,
            },
            end: Position {
                line: end_line,
                character: end_col,
            },
        },
        severity: Some(map_severity(finding.severity)),
        code: Some(NumberOrString::String(finding.rule_id.to_string())),
        code_description: None,
        source: Some("sextant".into()),
        message: finding.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

pub(crate) fn map_severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warn => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
    }
}

/// UTF-16 code-unit length of the given line (0-indexed). Returns `None`
/// when the buffer is missing or the line is past the end.
fn end_of_line_utf16(text: Option<&str>, line: u32) -> Option<u32> {
    let text = text?;
    let mut current = 0u32;
    for l in text.split('\n') {
        if current == line {
            let stripped = l.strip_suffix('\r').unwrap_or(l);
            let units: u32 = stripped.chars().map(|c| c.len_utf16() as u32).sum();
            return Some(units);
        }
        current = current.checked_add(1)?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn finding(line: Option<u32>, end_line: Option<u32>, sev: Severity) -> Finding {
        let mut f = Finding::new(
            String::from("rule.x"),
            sev,
            PathBuf::from("a.rs"),
            String::from("msg"),
        );
        if let Some(s) = line {
            f = match end_line {
                Some(e) => f.spanning(s, e),
                None => f.at_line(s),
            };
        }
        f
    }

    #[test]
    fn maps_severity() {
        assert_eq!(map_severity(Severity::Error), DiagnosticSeverity::ERROR);
        assert_eq!(map_severity(Severity::Warn), DiagnosticSeverity::WARNING);
        assert_eq!(
            map_severity(Severity::Info),
            DiagnosticSeverity::INFORMATION
        );
    }

    #[test]
    fn line_is_zero_indexed_in_diagnostic() {
        let d = finding_to_diagnostic(&finding(Some(3), None, Severity::Warn), Some("a\nb\nc\n"));
        assert_eq!(d.range.start.line, 2);
        assert_eq!(d.range.end.line, 2);
    }

    #[test]
    fn end_line_extends_range() {
        let d = finding_to_diagnostic(&finding(Some(1), Some(3), Severity::Error), None);
        assert_eq!(d.range.start.line, 0);
        assert_eq!(d.range.end.line, 2);
    }

    #[test]
    fn no_line_anchors_at_zero() {
        let d = finding_to_diagnostic(&finding(None, None, Severity::Info), None);
        assert_eq!(d.range.start.line, 0);
        assert_eq!(d.range.end.line, 0);
    }

    #[test]
    fn rule_id_becomes_code() {
        let d = finding_to_diagnostic(&finding(Some(1), None, Severity::Warn), None);
        assert_eq!(d.code, Some(NumberOrString::String(String::from("rule.x"))));
        assert_eq!(d.source.as_deref(), Some("sextant"));
    }

    #[test]
    fn end_column_counts_utf16_units() {
        let d = finding_to_diagnostic(&finding(Some(1), None, Severity::Warn), Some("abc\n"));
        assert_eq!(d.range.end.character, 3);
    }

    #[test]
    fn end_of_line_utf16_handles_emoji() {
        // U+1F600 is two UTF-16 code units (a surrogate pair), one char.
        let n = end_of_line_utf16(Some("a😀b\n"), 0).unwrap();
        assert_eq!(n, 1 + 2 + 1);
    }
}
