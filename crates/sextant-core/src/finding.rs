use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
}

impl Finding {
    pub fn new(
        rule_id: impl Into<String>,
        severity: Severity,
        path: impl Into<PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity,
            message: message.into(),
            path: path.into(),
            line: None,
            end_line: None,
        }
    }

    pub fn at_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    pub fn spanning(mut self, start: u32, end: u32) -> Self {
        self.line = Some(start);
        self.end_line = Some(end);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_as_str_round_trips() {
        assert_eq!(Severity::Info.as_str(), "info");
        assert_eq!(Severity::Warn.as_str(), "warn");
        assert_eq!(Severity::Error.as_str(), "error");
    }

    #[test]
    fn severity_orders_info_lt_warn_lt_error() {
        assert!(Severity::Info < Severity::Warn);
        assert!(Severity::Warn < Severity::Error);
    }

    #[test]
    fn new_starts_unspanned() {
        let f = Finding::new("r.id", Severity::Warn, "a.rs", "msg");
        assert_eq!(f.rule_id, "r.id");
        assert_eq!(f.severity, Severity::Warn);
        assert!(f.line.is_none() && f.end_line.is_none());
    }

    #[test]
    fn at_line_sets_only_start() {
        let f = Finding::new("r", Severity::Info, "a.rs", "m").at_line(7);
        assert_eq!(f.line, Some(7));
        assert!(f.end_line.is_none());
    }

    #[test]
    fn spanning_sets_both_endpoints() {
        let f = Finding::new("r", Severity::Info, "a.rs", "m").spanning(3, 11);
        assert_eq!(f.line, Some(3));
        assert_eq!(f.end_line, Some(11));
    }
}
