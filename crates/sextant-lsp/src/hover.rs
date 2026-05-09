//! Hover handler. Renders matching `Finding`s as markdown plus the rule's
//! own documentation body so the user gets `engine::explain_rule` text in
//! a popover instead of having to context-switch to the CLI.

use sextant_core::{Finding, Severity};
use sextant_engine::RuleSummary;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

use crate::convert::finding_to_diagnostic;

/// Build a hover popover for any findings whose range contains `position`.
/// `lookup` resolves a rule id to its full `RuleSummary` (cached by the
/// caller). Returns `None` when no finding overlaps the cursor.
pub(crate) fn hover_for_findings(
    findings: &[Finding],
    text: Option<&str>,
    position: Position,
    mut lookup: impl FnMut(&str) -> Option<RuleSummary>,
) -> Option<Hover> {
    let mut blocks: Vec<String> = Vec::new();
    let mut span: Option<Range> = None;
    for finding in findings {
        let diag = finding_to_diagnostic(finding, text);
        if !range_contains(&diag.range, position) {
            continue;
        }
        let rule = lookup(&finding.rule_id);
        blocks.push(render_block(finding, rule.as_ref()));
        span = Some(span.map(|r| union(&r, &diag.range)).unwrap_or(diag.range));
    }
    if blocks.is_empty() {
        return None;
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: blocks.join("\n\n---\n\n"),
        }),
        range: span,
    })
}

fn render_block(finding: &Finding, rule: Option<&RuleSummary>) -> String {
    let title = rule
        .map(|r| r.name.as_str())
        .unwrap_or(finding.rule_id.as_str());
    let severity = severity_label(finding.severity);
    let mut out = format!(
        "**{}** &middot; `{}` &middot; _{}_\n\n{}",
        title, finding.rule_id, severity, finding.message
    );
    if let Some(rule) = rule {
        if !rule.description.is_empty() {
            out.push_str("\n\n");
            out.push_str(&rule.description);
        }
        if !rule.body.is_empty() {
            out.push_str("\n\n");
            out.push_str(&rule.body);
        }
    }
    out
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warn => "warn",
        Severity::Info => "info",
    }
}

fn range_contains(range: &Range, position: Position) -> bool {
    let after_start = position.line > range.start.line
        || (position.line == range.start.line && position.character >= range.start.character);
    let before_end = position.line < range.end.line
        || (position.line == range.end.line && position.character <= range.end.character);
    after_start && before_end
}

fn union(a: &Range, b: &Range) -> Range {
    let start = if (a.start.line, a.start.character) <= (b.start.line, b.start.character) {
        a.start
    } else {
        b.start
    };
    let end = if (a.end.line, a.end.character) >= (b.end.line, b.end.character) {
        a.end
    } else {
        b.end
    };
    Range { start, end }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sextant_core::Finding;
    use std::path::PathBuf;

    fn warn_finding(rule_id: &str, line: u32) -> Finding {
        Finding::new(
            String::from(rule_id),
            Severity::Warn,
            PathBuf::from("a.rs"),
            format!("violated {rule_id}"),
        )
        .at_line(line)
    }

    #[test]
    fn hover_returns_none_without_overlap() {
        let findings = vec![warn_finding("rule.a", 5)];
        let hov = hover_for_findings(
            &findings,
            None,
            Position {
                line: 0,
                character: 0,
            },
            |_| None,
        );
        assert!(hov.is_none());
    }

    #[test]
    fn hover_renders_finding_message() {
        let findings = vec![warn_finding("rule.a", 1)];
        let hov = hover_for_findings(
            &findings,
            None,
            Position {
                line: 0,
                character: 0,
            },
            |_| None,
        )
        .expect("hover");
        let HoverContents::Markup(content) = hov.contents else {
            panic!("expected markup")
        };
        assert!(content.value.contains("violated rule.a"));
        assert!(content.value.contains("rule.a"));
        assert!(content.value.contains("warn"));
    }
}
