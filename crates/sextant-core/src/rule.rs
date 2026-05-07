use serde::{Deserialize, Serialize};

use crate::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Diff,
    File,
    Repo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Complexity,
    Size,
    Duplication,
    Tests,
    Reliability,
    Style,
    Security,
    Docs,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    /// One-line description shown in `rules list` and report summaries.
    pub description: String,
    /// Full markdown documentation; surfaced via `explain_rule` and used as
    /// the prompt template for LLM-evaluated rules.
    #[serde(default)]
    pub body: String,
    pub severity: Severity,
    pub category: Category,
    pub scope: Scope,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Where this rule came from. Useful for `rules list` UX.
    #[serde(default)]
    pub source: RuleSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleSource {
    #[default]
    Builtin,
    Repo,
}

impl RuleSource {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleSource::Builtin => "builtin",
            RuleSource::Repo => "repo",
        }
    }
}

fn default_enabled() -> bool {
    true
}
