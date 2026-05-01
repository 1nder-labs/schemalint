use crate::ir::{Arena, NodeId};
use crate::profile::Profile;
use crate::rules::registry::{
    keyword_present, keyword_value, Diagnostic, DiagnosticSeverity, Rule,
};

/// Class A auto-generated keyword rule.
///
/// Fires when a node carries the watched keyword in its annotations.
#[derive(Debug, Clone)]
pub struct KeywordRule {
    pub keyword: &'static str,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub profile_name: String,
}

impl Rule for KeywordRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if keyword_present(&arena[node], self.keyword) {
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
}

/// Class A auto-generated restriction rule.
///
/// Fires when a keyword is present and its value is not in the allowed set.
#[derive(Debug, Clone)]
pub struct RestrictionRule {
    pub keyword: &'static str,
    pub allowed_values: Vec<serde_json::Value>,
    pub code: String,
    pub profile_name: String,
}

impl Rule for RestrictionRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if let Some(value) = keyword_value(&arena[node], self.keyword) {
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
}
