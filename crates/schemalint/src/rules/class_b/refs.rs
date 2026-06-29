use serde_json::Value;

use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

#[derive(Debug, Clone)]
pub(super) struct ExternalRefsRule {
    pub(super) profile_name: String,
}

impl Rule for ExternalRefsRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let Some(Value::String(ref_str)) = &arena[node].annotations.r#ref else {
            return Vec::new();
        };
        if !is_external_ref(ref_str) {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-external-refs", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!("external $ref '{}' is not supported", ref_str),
            pointer: arena[node].json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "external-refs".into(),
            code: "{prefix}-S-external-refs".into(),
            description: "External $ref values are not supported".into(),
            rationale: "Providers require references to be internal to the submitted schema.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r##"{ "type": "object", "properties": { "address": { "$ref": "https://example.com/address.json" } } }"##.into(),
            good_example: r##"{ "type": "object", "$defs": { "Address": { "type": "object" } }, "properties": { "address": { "$ref": "#/$defs/Address" } } }"##.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct AllOfWithRefRule {
    pub(super) profile_name: String,
}

impl Rule for AllOfWithRefRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        let Some(Value::Array(branches)) = &node_ref.annotations.all_of else {
            return Vec::new();
        };
        if !branches.iter().any(contains_ref) {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-allof-with-ref", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: "Anthropic Structured Outputs does not support allOf combined with $ref"
                .to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "allof-with-ref".into(),
            code: "{prefix}-S-allof-with-ref".into(),
            description: "allOf combined with $ref is not supported by Anthropic".into(),
            rationale: "Anthropic rejects schemas that combine allOf with $ref references.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r##"{ "type": "object", "allOf": [{ "$ref": "#/$defs/Base" }] }"##.into(),
            good_example: r#"{ "type": "object", "properties": { "id": { "type": "string" } } }"#
                .into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

fn is_external_ref(ref_str: &str) -> bool {
    // A $ref is internal only when it is a pure JSON Pointer fragment (starts
    // with '#').  Everything else — http://, https://, file://, /, ./foo.json,
    // ../foo.json, bare foo.json — is treated as external and flagged when the
    // profile enables this rule.
    !ref_str.starts_with('#')
}

fn contains_ref(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.contains_key("$ref") || map.values().any(contains_ref),
        Value::Array(arr) => arr.iter().any(contains_ref),
        _ => false,
    }
}
