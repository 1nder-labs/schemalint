use crate::profile::Severity;

/// Category of a lint rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleCategory {
    Keyword,
    Restriction,
    Structural,
    Semantic,
}

impl RuleCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleCategory::Keyword => "keyword",
            RuleCategory::Restriction => "restriction",
            RuleCategory::Structural => "structural",
            RuleCategory::Semantic => "semantic",
        }
    }
}

/// Descriptive metadata for a lint rule, used by the documentation generator.
#[derive(Debug, Clone)]
pub struct RuleMetadata {
    /// Short slug for the rule page filename (e.g. "allof", "max-depth").
    pub name: String,
    /// Canonical error code template. Uses `{prefix}` placeholder for
    /// profile-driven prefixes (e.g. `"{prefix}-K-allOf"`).
    pub code: String,
    /// One-sentence description of what the rule checks.
    pub description: String,
    /// Why the rule exists — provider behavior context.
    pub rationale: String,
    /// Severity from the profile (for static rules, the canonical severity).
    pub severity: Severity,
    /// Category for grouping in documentation.
    pub category: RuleCategory,
    /// A minimal JSON Schema snippet that triggers the rule.
    pub bad_example: String,
    /// A minimal JSON Schema snippet that passes the rule.
    pub good_example: String,
    /// Related resources or documentation links (empty if none).
    pub see_also: Vec<String>,
    /// Profile name if this is a profile-gated rule, `None` for universal rules.
    pub profile: Option<String>,
}
