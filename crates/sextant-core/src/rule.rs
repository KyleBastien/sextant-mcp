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

impl Category {
    /// Display name for `rules list` / `rules explain` output. Custom
    /// categories are rendered as `custom:<name>`. Built-in variants use
    /// their serde-rendered lowercase name, which keeps this in lockstep
    /// with the JSON wire format whenever new variants are added.
    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        if let Category::Custom(s) = self {
            return format!("custom:{s}").into();
        }
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".into())
            .into()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_name_is_lowercase_for_built_ins() {
        assert_eq!(Category::Tests.name(), "tests");
        assert_eq!(Category::Complexity.name(), "complexity");
        assert_eq!(Category::Security.name(), "security");
    }

    #[test]
    fn category_name_prefixes_custom_with_namespace() {
        assert_eq!(Category::Custom("perf".into()).name(), "custom:perf",);
    }

    #[test]
    fn rule_source_as_str_matches_serde_form() {
        assert_eq!(RuleSource::Builtin.as_str(), "builtin");
        assert_eq!(RuleSource::Repo.as_str(), "repo");
    }
}
