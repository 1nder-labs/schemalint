use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity, UnknownKeywordPolicy};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

/// Flags every key in `Node::unknown` — a keyword the engine's IR parser does
/// not carry an accessor for at all.  This is a different fact from a Class A
/// `KeywordRule`, which reports a keyword the engine DOES recognize but that
/// a provider forbids, strips, or has not verified. That distinction is why
/// this rule reads `Node::unknown` instead of a typed `Keyword` accessor, and
/// why its message never claims a provider stance the engine cannot verify.
#[derive(Debug, Clone)]
pub(super) struct UnknownKeywordRule {
    pub(super) severity: DiagnosticSeverity,
    pub(super) profile_name: String,
}

impl Rule for UnknownKeywordRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        node_ref
            .unknown
            .keys()
            .map(|key| {
                let hint = match self.severity {
                    DiagnosticSeverity::Error => format!(
                        "remove '{}' from the schema. schemalint does not recognize this keyword and cannot validate it.",
                        key
                    ),
                    DiagnosticSeverity::Warning => format!(
                        "schemalint does not recognize '{}' and cannot validate it. Confirm the provider accepts it, or remove it if it is not needed.",
                        key
                    ),
                };
                Diagnostic {
                    code: format!("{}-S-unknown-keyword", profile.code_prefix),
                    severity: self.severity,
                    message: format!("keyword '{}' is not recognized by schemalint", key),
                    pointer: node_ref.json_pointer.clone(),
                    source: None,
                    profile: self.profile_name.clone(),
                    provider_evidence: None,
                    hint: Some(hint),
                }
            })
            .collect()
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        let sev = match self.severity {
            DiagnosticSeverity::Error => Severity::Forbid,
            DiagnosticSeverity::Warning => Severity::Warn,
        };
        Some(RuleMetadata {
            name: "unknown-keyword".into(),
            code: "{prefix}-S-unknown-keyword".into(),
            description: "Flag a keyword the engine does not recognize at all".into(),
            rationale: "schemalint knows a fixed set of JSON Schema keywords. A keyword outside \
                that set was never checked against provider behavior, so schemalint cannot say \
                whether the provider accepts, ignores, or rejects it."
                .into(),
            severity: sev,
            category: RuleCategory::Structural,
            bad_example: r#"{ "type": "string", "contentEncoding": "base64" }"#.into(),
            good_example: r#"{ "type": "string" }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

/// Map an `UnknownKeywordPolicy` to the rule's severity. `Allow` means no
/// rule should be constructed at all — callers check for that before this.
pub(super) fn policy_severity(policy: UnknownKeywordPolicy) -> Option<DiagnosticSeverity> {
    match policy {
        UnknownKeywordPolicy::Allow => None,
        UnknownKeywordPolicy::Warn => Some(DiagnosticSeverity::Warning),
        UnknownKeywordPolicy::Forbid => Some(DiagnosticSeverity::Error),
    }
}
