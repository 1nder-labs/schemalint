use serde_json::Value;

use crate::ir::{Arena, NodeId};
use crate::profile::Profile;
use crate::rules::class_b::schema_is_object;
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};
use crate::Severity;

// ---------------------------------------------------------------------------
// EmptyObjectRule
// ---------------------------------------------------------------------------

/// Warn when an object schema has `additionalProperties: false` and either
/// missing `properties` or `properties: {}`.
#[derive(Debug)]
struct EmptyObjectRule;

impl Rule for EmptyObjectRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_object(node_ref) {
            return Vec::new();
        }
        let ap_false = matches!(
            &node_ref.annotations.additional_properties,
            Some(Value::Bool(false))
        );
        if !ap_false {
            return Vec::new();
        }
        let props_empty = match &node_ref.annotations.properties {
            None => true,
            Some(Value::Object(map)) => map.is_empty(),
            _ => false,
        };
        if !props_empty {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-empty-object", profile.code_prefix),
            severity: DiagnosticSeverity::Warning,
            message: "object schema with additionalProperties: false has no properties".to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: profile.name.clone(),
            hint: Some(
                "Consider adding properties or relaxing additionalProperties for provider compatibility"
                    .to_string(),
            ),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "empty-object".into(),
            code: "{prefix}-S-empty-object".into(),
            description: "Object schema with additionalProperties: false but no properties".into(),
            rationale: "Some providers may reject or misbehave when a schema permits no properties while also forbidding all extras via additionalProperties: false. This pattern is semantically valid but rarely intentional.".into(),
            severity: Severity::Warn,
            category: RuleCategory::Semantic,
            bad_example: r#"{
  "type": "object",
  "additionalProperties": false,
  "properties": {}
}"#.into(),
            good_example: r#"{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "name": { "type": "string" }
  }
}"#.into(),
            see_also: Vec::new(),
            profile: None,
        })
    }
}

// ---------------------------------------------------------------------------
// AdditionalPropertiesObjectRule
// ---------------------------------------------------------------------------

/// Error when `additionalProperties` is an object value instead of `false`.
#[derive(Debug)]
struct AdditionalPropertiesObjectRule;

impl Rule for AdditionalPropertiesObjectRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_object(node_ref) {
            return Vec::new();
        }
        match &node_ref.annotations.additional_properties {
            Some(Value::Object(_)) => vec![Diagnostic {
                code: format!("{}-S-additional-properties-object", profile.code_prefix),
                severity: DiagnosticSeverity::Error,
                message: "additionalProperties must be false, not an object schema".to_string(),
                pointer: node_ref.json_pointer.clone(),
                source: None,
                profile: profile.name.clone(),
                hint: Some(
                    "Set additionalProperties to false for provider compatibility".to_string(),
                ),
            }],
            _ => Vec::new(),
        }
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "additional-properties-object".into(),
            code: "{prefix}-S-additional-properties-object".into(),
            description: "additionalProperties must be set to false, not an object schema".into(),
            rationale: "LLM structured-output providers require additionalProperties: false to guarantee schema compliance. An object value indicates intent to define allowed extras, which most providers do not support.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Semantic,
            bad_example: r#"{
  "type": "object",
  "additionalProperties": { "type": "string" },
  "properties": {}
}"#.into(),
            good_example: r#"{
  "type": "object",
  "additionalProperties": false,
  "properties": {}
}"#.into(),
            see_also: Vec::new(),
            profile: None,
        })
    }
}

// ---------------------------------------------------------------------------
// AnyOfObjectsHint
// ---------------------------------------------------------------------------

/// Warn when `anyOf` contains only object-typed branches.
#[derive(Debug)]
struct AnyOfObjectsHint;

impl Rule for AnyOfObjectsHint {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        let Some(Value::Array(branches)) = &node_ref.annotations.any_of else {
            return Vec::new();
        };
        if branches.is_empty() {
            return Vec::new();
        }
        let all_objects = branches.iter().all(|branch| {
            let Value::Object(map) = branch else {
                return false;
            };
            if let Some(Value::String(t)) = map.get("type") {
                return t == "object";
            }
            false
        });
        if !all_objects {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-anyof-objects", profile.code_prefix),
            severity: DiagnosticSeverity::Warning,
            message: "anyOf with only object-typed branches may not be fully supported".to_string(),
            pointer: node_ref.json_pointer.clone(),
            source: None,
            profile: profile.name.clone(),
            hint: Some(
                "Consider merging object branches into a single object schema for better provider compatibility"
                    .to_string(),
            ),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "anyof-objects".into(),
            code: "{prefix}-S-anyof-objects".into(),
            description: "anyOf with only object-typed branches may not be fully supported".into(),
            rationale: "When all anyOf branches are object-typed, some providers may not correctly resolve the union. Merging branches into a single object schema when appropriate improves compatibility across providers.".into(),
            severity: Severity::Warn,
            category: RuleCategory::Semantic,
            bad_example: r#"{
  "type": "object",
  "anyOf": [
    { "type": "object", "properties": { "x": { "type": "string" } } },
    { "type": "object", "properties": { "y": { "type": "number" } } }
  ]
}"#.into(),
            good_example: r#"{
  "type": "object",
  "properties": {
    "x": { "type": "string" },
    "y": { "type": "number" }
  }
}"#.into(),
            see_also: Vec::new(),
            profile: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Static registration
// ---------------------------------------------------------------------------

#[linkme::distributed_slice(crate::rules::RULES)]
static EMPTY_OBJECT_RULE: &dyn Rule = &EmptyObjectRule;

#[linkme::distributed_slice(crate::rules::RULES)]
static ADDITIONAL_PROPERTIES_OBJECT_RULE: &dyn Rule = &AdditionalPropertiesObjectRule;

#[linkme::distributed_slice(crate::rules::RULES)]
static ANY_OF_OBJECTS_HINT: &dyn Rule = &AnyOfObjectsHint;
