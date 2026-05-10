use std::borrow::Cow;
use std::fmt;

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

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

/// Provenance of a rule. Vendor packs are immutable, integrity-checked
/// bundles installed under `.sextant/rules/vendor/<pack>/`; their rules
/// can never be disabled by repo-level rules or `overrides:` lists.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum RuleSource {
    #[default]
    Builtin,
    /// A rule contributed by a vendor pack. The string is the pack name.
    Vendor(String),
    Repo,
}

impl RuleSource {
    /// Display name. `vendor:<pack>` for vendor packs, plain string for
    /// the rest. Used in `rules list` and JSON output.
    pub fn name(&self) -> Cow<'_, str> {
        match self {
            RuleSource::Builtin => Cow::Borrowed("builtin"),
            RuleSource::Repo => Cow::Borrowed("repo"),
            RuleSource::Vendor(pack) => Cow::Owned(format!("vendor:{pack}")),
        }
    }
}

impl fmt::Display for RuleSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

impl Serialize for RuleSource {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for RuleSource {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        match raw.as_str() {
            "builtin" => Ok(RuleSource::Builtin),
            "repo" => Ok(RuleSource::Repo),
            other if other.starts_with("vendor:") => {
                let pack = other.trim_start_matches("vendor:");
                if pack.is_empty() {
                    return Err(de::Error::custom("vendor source missing pack name"));
                }
                Ok(RuleSource::Vendor(pack.to_string()))
            }
            other => Err(de::Error::custom(format!("unknown rule source: {other}"))),
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
    fn rule_source_name_matches_serde_form() {
        assert_eq!(RuleSource::Builtin.name(), "builtin");
        assert_eq!(RuleSource::Repo.name(), "repo");
        assert_eq!(
            RuleSource::Vendor("typescript".into()).name(),
            "vendor:typescript"
        );
    }

    #[test]
    fn rule_source_round_trips_through_json() {
        for source in [
            RuleSource::Builtin,
            RuleSource::Repo,
            RuleSource::Vendor("typescript".into()),
        ] {
            let s = serde_json::to_string(&source).unwrap();
            let back: RuleSource = serde_json::from_str(&s).unwrap();
            assert_eq!(back, source);
        }
    }

    #[test]
    fn rule_source_rejects_empty_vendor_pack() {
        let err = serde_json::from_str::<RuleSource>("\"vendor:\"").unwrap_err();
        assert!(err.to_string().contains("missing pack name"));
    }
}
