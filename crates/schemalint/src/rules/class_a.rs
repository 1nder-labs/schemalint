use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, KeywordAccessor, Rule};

/// Class A auto-generated keyword rule.
///
/// Fires when a node carries the watched keyword in its annotations.
#[derive(Debug, Clone)]
pub struct KeywordRule {
    pub keyword: &'static str,
    pub accessor: KeywordAccessor,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub profile_name: String,
}

impl Rule for KeywordRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if (self.accessor)(&arena[node]).is_some() {
            let message = format!(
                "keyword '{}' is not supported by {}",
                self.keyword, self.profile_name
            );
            diagnostics.push(Diagnostic {
                code: self.code.clone(),
                severity: self.severity,
                message,
                pointer: arena[node].json_pointer.clone(),
                source: None,
                profile: self.profile_name.clone(),
                hint: None,
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

/// Class A auto-generated restriction rule.
///
/// Fires when a keyword is present and its value is not in the allowed set.
#[derive(Debug, Clone)]
pub struct RestrictionRule {
    pub keyword: &'static str,
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
                let hint = format!("allowed values: {:?}", self.allowed_values);
                diagnostics.push(Diagnostic {
                    code: self.code.clone(),
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "keyword '{}' has a restricted value not accepted by {}",
                        self.keyword, self.profile_name
                    ),
                    pointer: arena[node].json_pointer.clone(),
                    source: None,
                    profile: self.profile_name.clone(),
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
