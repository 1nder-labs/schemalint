use crate::ir::{Arena, NodeId};
use crate::profile::{Keyword, Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, KeywordAccessor, Rule};

/// Class A auto-generated keyword rule.
///
/// Fires when a node carries the watched keyword in its annotations.
#[derive(Debug, Clone)]
pub struct KeywordRule {
    pub keyword: Keyword,
    pub accessor: KeywordAccessor,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub profile_name: String,
}

impl Rule for KeywordRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if (self.accessor)(&arena[node]).is_some() {
            let message = format!(
                "keyword '{}' is not supported by {}",
                self.keyword, self.profile_name
            );
            let hint = match self.severity {
                DiagnosticSeverity::Error => rewrite_hint(
                    self.keyword,
                    profile,
                    arena[node].json_pointer.is_empty(),
                )
                .unwrap_or_else(|| {
                    format!(
                        "remove '{}' from the schema. If you need the constraint, check it in your own code after the model responds.",
                        self.keyword
                    )
                }),
                DiagnosticSeverity::Warning => format!(
                    "{} may ignore or strip '{}'. Do not rely on it; check the constraint in your own code after the model responds.",
                    self.profile_name, self.keyword
                ),
            };
            diagnostics.push(Diagnostic {
                target: None,
                code: self.code.clone(),
                severity: self.severity,
                message,
                pointer: arena[node].json_pointer.clone(),
                source: None,
                profile: self.profile_name.clone(),
                provider_evidence: None,
                hint: Some(hint),
            });
        }
        diagnostics
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        let sev = match self.severity {
            DiagnosticSeverity::Error => Severity::Forbid,
            DiagnosticSeverity::Warning => Severity::Warn,
        };
        Some(RuleMetadata {
            name: self.keyword.to_string(),
            code: format!("{{prefix}}-K-{}", self.keyword),
            description: format!(
                "Flag usage of the '{}' keyword, which is {} by {}",
                self.keyword,
                match sev {
                    Severity::Forbid => "not supported",
                    Severity::Strip => "stripped",
                    Severity::Warn => "discouraged",
                    _ => "restricted",
                },
                self.profile_name
            ),
            rationale: format!(
                "The {} structured-output provider {} the '{}' keyword. Schemas using this keyword may be rejected or silently altered.",
                self.profile_name,
                match sev {
                    Severity::Forbid => "rejects",
                    Severity::Strip => "strips",
                    Severity::Warn => "discourages use of",
                    _ => "restricts",
                },
                self.keyword
            ),
            severity: sev,
            category: RuleCategory::Keyword,
            bad_example: format!(
                r#"{{ "type": "object", "{}": true, "properties": {{}} }}"#,
                self.keyword
            ),
            good_example: r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

fn is_allowed(profile: &Profile, keyword: Keyword) -> bool {
    profile.keyword_map.get(&keyword) == Some(&Severity::Allow)
}

/// A keyword-specific rewrite for an unsupported keyword, or `None` to fall
/// back to the generic hint. Every suggested keyword is checked against the
/// profile so a hint never recommends something the profile also rejects.
fn rewrite_hint(keyword: Keyword, profile: &Profile, at_root: bool) -> Option<String> {
    match keyword {
        Keyword::OneOf if is_allowed(profile, Keyword::AnyOf) => {
            let root_note = if at_root && profile.structural.forbid_root_any_of {
                " anyOf is not allowed at the schema root either; wrap the union in an object property."
            } else {
                ""
            };
            Some(format!(
                "replace 'oneOf' with 'anyOf'. anyOf does not enforce that exactly one branch matches, so keep the branches mutually exclusive in practice.{root_note}"
            ))
        }
        Keyword::AllOf => Some(
            "merge the 'allOf' branches into one object schema: combine their 'properties' and 'required' lists and drop the 'allOf'."
                .to_string(),
        ),
        Keyword::Const if is_allowed(profile, Keyword::Enum) => Some(
            "replace 'const': <value> with 'enum': [<value>], a single-value enum.".to_string(),
        ),
        Keyword::Pattern
        | Keyword::MinLength
        | Keyword::MaxLength
        | Keyword::Minimum
        | Keyword::Maximum
        | Keyword::ExclusiveMinimum
        | Keyword::ExclusiveMaximum
        | Keyword::MultipleOf
        | Keyword::MinItems
        | Keyword::MaxItems
        | Keyword::UniqueItems
        | Keyword::MinProperties
        | Keyword::MaxProperties
            if is_allowed(profile, Keyword::Description) =>
        {
            Some(format!(
                "remove '{keyword}' from the schema and state the constraint in the field's 'description' so the model sees it. The provider will not enforce it, so validate the response in your own code."
            ))
        }
        _ => None,
    }
}

/// Class A auto-generated restriction rule.
///
/// Fires when a keyword is present and its value is not in the allowed set.
#[derive(Debug, Clone)]
pub struct RestrictionRule {
    pub keyword: Keyword,
    pub accessor: KeywordAccessor,
    pub allowed_values: Vec<serde_json::Value>,
    pub code: String,
    pub profile_name: String,
}

impl Rule for RestrictionRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if let Some(value) = (self.accessor)(&arena[node]) {
            if !self.allowed_values.contains(value) {
                // `{:?}` on a serde_json::Value prints `String("email")`; use
                // Display so the hint carries valid JSON a consumer can reuse.
                let allowed = self
                    .allowed_values
                    .iter()
                    .map(|allowed| allowed.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let hint = format!(
                    "'{}' accepts only {} here, but found {}. Use an allowed value; if none fits, remove '{}' and check the constraint in your own code after the model responds.",
                    self.keyword, allowed, value, self.keyword
                );
                diagnostics.push(Diagnostic {
                    target: None,
                    code: self.code.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "keyword '{}' has a restricted value not accepted by {}",
                        self.keyword, self.profile_name
                    ),
                    pointer: arena[node].json_pointer.clone(),
                    source: None,
                    profile: self.profile_name.clone(),
                    provider_evidence: None,
                    hint: Some(hint),
                });
            }
        }
        diagnostics
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: format!("{}-restricted", self.keyword),
            code: format!("{{prefix}}-K-{}-restricted", self.keyword),
            description: format!(
                "Restrict values of the '{}' keyword to those accepted by {}",
                self.keyword, self.profile_name
            ),
            rationale: format!(
                "{} only supports specific values for the '{}' keyword. Using unsupported values will cause validation errors at the API level.",
                self.profile_name, self.keyword
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Restriction,
            bad_example: format!(
                r#"{{ "type": "object", "{}": "invalid-value", "properties": {{}} }}"#,
                self.keyword
            ),
            good_example: format!(
                r#"{{ "type": "object", "{}": {}, "properties": {{}} }}"#,
                self.keyword,
                self.allowed_values
                    .first()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "\"<allowed-value>\"".to_string())
            ),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
