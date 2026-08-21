use crate::ir::{Arena, NodeId};
use crate::profile::{Keyword, Profile, ProviderEvidence, RuleKey, Severity};
use serde::{Deserialize, Serialize};

pub use crate::profile::KeywordAccessor;

/// Severity of a diagnostic emitted by the rule engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// Location in a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

/// A lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub pointer: String,
    pub source: Option<SourceSpan>,
    pub profile: String,
    pub hint: Option<String>,
    pub provider_evidence: Option<ProviderEvidence>,
}

/// Trait implemented by all lint rules.
pub trait Rule: Sync {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic>;

    /// Return descriptive metadata for documentation generation.
    /// Returns `None` if the rule predates the metadata system — doc
    /// generation skips such rules gracefully.
    fn metadata(&self) -> Option<super::metadata::RuleMetadata> {
        None
    }
}

// ---------------------------------------------------------------------------
// linkme distributed slice for compile-time rule registration
// ---------------------------------------------------------------------------

use linkme::distributed_slice;

#[distributed_slice]
pub static RULES: [&'static dyn Rule] = [..];

// ---------------------------------------------------------------------------
// RuleSet: combines static (linkme) and dynamic (profile-generated) rules
// ---------------------------------------------------------------------------

/// A collection of rules ready to run against a schema.
pub struct RuleSet {
    static_rules: &'static [&'static dyn Rule],
    dynamic_rules: Vec<Box<dyn Rule>>,
}

/// Profile configuration errors discovered while constructing dynamic rules.
#[derive(Debug, thiserror::Error)]
pub enum RuleSetError {
    #[error("keyword '{0}' cannot define both a severity and a restriction")]
    ConflictingKeyword(Keyword),
}

impl RuleSet {
    /// Build a RuleSet from a loaded profile. Generates Class A keyword and
    /// restriction rules from the profile data and includes all compile-time
    /// registered rules.
    pub fn from_profile(profile: &Profile) -> Result<Self, RuleSetError> {
        let mut dynamic_rules: Vec<Box<dyn Rule>> = Vec::new();

        if let Some(keyword) = profile
            .keyword_map
            .keys()
            .find(|keyword| profile.restrictions.contains_key(*keyword))
        {
            return Err(RuleSetError::ConflictingKeyword(*keyword));
        }

        // Class A keyword rules.
        for (&keyword, &severity) in &profile.keyword_map {
            let diag_severity = match severity {
                Severity::Forbid | Severity::Strip => DiagnosticSeverity::Error,
                Severity::Warn => DiagnosticSeverity::Warning,
                _ => continue,
            };
            dynamic_rules.push(Box::new(super::class_a::KeywordRule {
                keyword,
                accessor: keyword.accessor(),
                severity: diag_severity,
                code: format!("{}-K-{}", profile.code_prefix, keyword.as_str()),
                profile_name: profile.name.clone(),
            }));
        }

        // Class A restriction rules.
        for (&keyword, restriction) in &profile.restrictions {
            dynamic_rules.push(Box::new(super::class_a::RestrictionRule {
                keyword,
                accessor: keyword.accessor(),
                allowed_values: restriction.allowed_values.clone(),
                code: format!("{}-K-{}-restricted", profile.code_prefix, keyword.as_str()),
                profile_name: profile.name.clone(),
            }));
        }

        // Class B structural rules.
        dynamic_rules.extend(super::class_b::generate_class_b_rules(profile));

        Ok(Self {
            static_rules: &*RULES,
            dynamic_rules,
        })
    }

    /// Run every rule in the set against a single node.
    pub fn check_node(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for &rule in self.static_rules {
            diagnostics.extend(rule.check(node, arena, profile));
        }
        for rule in &self.dynamic_rules {
            diagnostics.extend(rule.check(node, arena, profile));
        }
        attach_provider_evidence(&mut diagnostics, profile);
        diagnostics
    }

    /// Run every rule against every node in the arena and collect all diagnostics.
    pub fn check_all(&self, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (node_id, _) in arena.iter() {
            diagnostics.extend(self.check_node(node_id, arena, profile));
        }
        diagnostics
    }

    /// Iterate over all dynamic rules (profile-generated).
    pub fn dynamic_rules(&self) -> impl Iterator<Item = &dyn Rule> {
        self.dynamic_rules.iter().map(|r| r.as_ref())
    }
}

pub(crate) fn attach_provider_evidence(diagnostics: &mut [Diagnostic], profile: &Profile) {
    for diagnostic in diagnostics {
        diagnostic.provider_evidence = RuleKey::from_code(&diagnostic.code, &profile.code_prefix)
            .and_then(|key| profile.evidence.get(&key).cloned());
    }
}

// ---------------------------------------------------------------------------
// Keyword accessor
// ---------------------------------------------------------------------------

/// Return a function pointer that extracts the value for a known keyword.
///
/// This compiles the 40-arm match into a single function-pointer dispatch,
/// eliminating string comparison overhead in hot rule loops.
pub fn keyword_accessor(keyword: &str) -> Option<KeywordAccessor> {
    keyword.parse::<Keyword>().ok().map(Keyword::accessor)
}
