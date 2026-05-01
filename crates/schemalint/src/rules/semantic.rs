use serde_json::Value;

use crate::ir::{Arena, NodeId};
use crate::profile::Profile;
use crate::rules::class_b::schema_is_object;
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

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
