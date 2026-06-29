use serde_json::Value;

use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::class_b::helpers::{missing_required_properties, schema_is_object};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

#[derive(Debug, Clone)]
pub(super) struct ObjectRootRule {
    pub(super) profile_name: String,
}

impl Rule for ObjectRootRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() || schema_is_object(&arena[node]) {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-object-root", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: "root schema must be an object".to_string(),
            pointer: arena[node].json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "object-root".into(),
            code: "{prefix}-S-object-root".into(),
            description: "The root schema must be of type object".into(),
            rationale: "Structured-output providers require the top-level schema to be an object."
                .into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{
  "type": "array",
  "items": { "type": "string" }
}"#
            .into(),
            good_example: r#"{
  "type": "object",
  "properties": { "name": { "type": "string" } }
}"#
            .into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct AdditionalPropertiesFalseRule {
    pub(super) profile_name: String,
}

impl Rule for AdditionalPropertiesFalseRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_object(node_ref) {
            return Vec::new();
        }
        match &node_ref.annotations.additional_properties {
            Some(Value::Bool(false)) => Vec::new(),
            _ => vec![Diagnostic {
                code: format!("{}-S-additional-properties-false", profile.code_prefix),
                severity: DiagnosticSeverity::Error,
                message: "object must declare additionalProperties: false".to_string(),
                pointer: node_ref.json_pointer.clone(),
                source: None,
                profile: self.profile_name.clone(),
                hint: None,
            }],
        }
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "additional-properties-false".into(),
            code: "{prefix}-S-additional-properties-false".into(),
            description: "Every object schema must declare additionalProperties: false".into(),
            rationale: "Providers require object nodes to explicitly reject extra properties."
                .into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{ "type": "object", "properties": { "name": { "type": "string" } } }"#
                .into(),
            good_example: r#"{ "type": "object", "additionalProperties": false, "properties": { "name": { "type": "string" } } }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct AllPropertiesRequiredRule {
    pub(super) profile_name: String,
}

impl Rule for AllPropertiesRequiredRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_object(node_ref) {
            return Vec::new();
        }
        let missing = missing_required_properties(node_ref);
        if missing.is_empty() {
            return Vec::new();
        }

        vec![Diagnostic {
            code: format!("{}-S-all-properties-required", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "object has {} propert{} not listed in required: {}",
                missing.len(),
                if missing.len() == 1 { "y" } else { "ies" },
                missing
                    .iter()
                    .take(8)
                    .map(|key| format!("'{}'", key))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: if missing.len() > 8 {
                Some(format!(
                    "Add all object properties to required; first 8 shown, {} total missing",
                    missing.len()
                ))
            } else {
                Some("Add every object property to required".to_string())
            },
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "all-properties-required".into(),
            code: "{prefix}-S-all-properties-required".into(),
            description: "Every property must be listed in the required array".into(),
            rationale: "Some providers reject schemas with optional object properties.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{ "type": "object", "properties": { "name": { "type": "string" }, "age": { "type": "number" } }, "required": ["name"] }"#.into(),
            good_example: r#"{ "type": "object", "properties": { "name": { "type": "string" }, "age": { "type": "number" } }, "required": ["name", "age"] }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
