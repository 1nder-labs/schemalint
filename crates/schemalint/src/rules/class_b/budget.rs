use serde_json::Value;

use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::class_b::helpers::missing_required_properties;
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

#[derive(Debug, Clone)]
pub(super) struct MaxDepthRule {
    pub(super) limit: u32,
    pub(super) profile_name: String,
}

impl Rule for MaxDepthRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].depth <= self.limit {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-max-depth", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "object nesting depth {} exceeds limit of {}",
                arena[node].depth, self.limit
            ),
            pointer: arena[node].json_pointer.clone(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-depth".into(),
            code: "{prefix}-S-max-depth".into(),
            description: format!(
                "Object nesting depth must not exceed {} levels",
                self.limit
            ),
            rationale: format!(
                "{} limits object nesting depth to {} levels.",
                self.profile_name, self.limit
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { \"nested\": { \"type\": \"object\", \"properties\": { \"too_deep\": { \"type\": \"object\" } } } } }".into(),
            good_example: r#"{ "type": "object", "properties": { "name": { "type": "string" } } }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct MaxTotalPropertiesRule {
    pub(super) limit: u32,
    pub(super) profile_name: String,
}

impl Rule for MaxTotalPropertiesRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let total: usize = arena
            .iter()
            .filter_map(|(_, n)| n.annotations.properties.as_ref())
            .filter_map(|v| v.as_object().map(|o| o.len()))
            .sum();
        if total <= self.limit as usize {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-max-total-properties", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "total property count {} exceeds limit of {}",
                total, self.limit
            ),
            pointer: String::new(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-total-properties".into(),
            code: "{prefix}-S-max-total-properties".into(),
            description: format!("Total object properties must not exceed {}", self.limit),
            rationale: format!(
                "{} limits the total number of object properties.",
                self.profile_name
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { \"...many\": {} } }".into(),
            good_example: r#"{ "type": "object", "properties": { "name": { "type": "string" } } }"#
                .into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct MaxTotalEnumValuesRule {
    pub(super) limit: u32,
    pub(super) profile_name: String,
}

impl Rule for MaxTotalEnumValuesRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let total: usize = arena
            .iter()
            .filter_map(|(_, n)| n.annotations.enum_values.as_ref())
            .filter_map(|v| v.as_array().map(|a| a.len()))
            .sum();
        if total <= self.limit as usize {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-max-enum-values", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "total enum value count {} exceeds limit of {}",
                total, self.limit
            ),
            pointer: String::new(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-enum-values".into(),
            code: "{prefix}-S-max-enum-values".into(),
            description: format!("Total enum values must not exceed {}", self.limit),
            rationale: format!("{} limits total enum values.", self.profile_name),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"string\", \"enum\": [\"...1000+ values\"] }".into(),
            good_example: r#"{ "type": "string", "enum": ["red", "green", "blue"] }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct MaxStringLengthRule {
    pub(super) limit: u32,
    pub(super) profile_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct MaxOptionalPropertiesRule {
    pub(super) limit: u32,
    pub(super) profile_name: String,
}

impl Rule for MaxOptionalPropertiesRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let total = arena
            .iter()
            .map(|(_, node)| optional_property_count(node))
            .sum::<usize>();
        if total <= self.limit as usize {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-max-optional-properties", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "optional property count {} exceeds limit of {}",
                total, self.limit
            ),
            pointer: String::new(),
            source: None,
            profile: self.profile_name.clone(),
            hint: Some("Mark more properties as required or split the schema".into()),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-optional-properties".into(),
            code: "{prefix}-S-max-optional-properties".into(),
            description: format!("Optional properties must not exceed {}", self.limit),
            rationale: format!(
                "{} limits optional parameters across strict schemas.",
                self.profile_name
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example:
                r#"{ "type": "object", "properties": { "optional": { "type": "string" } } }"#
                    .into(),
            good_example: r#"{ "type": "object", "properties": { "required": { "type": "string" } }, "required": ["required"], "additionalProperties": false }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct MaxUnionPropertiesRule {
    pub(super) limit: u32,
    pub(super) profile_name: String,
}

impl Rule for MaxUnionPropertiesRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let total = arena
            .iter()
            .filter(|(_, node)| is_union_parameter(node))
            .count();
        if total <= self.limit as usize {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-max-union-properties", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "union parameter count {} exceeds limit of {}",
                total, self.limit
            ),
            pointer: String::new(),
            source: None,
            profile: self.profile_name.clone(),
            hint: Some("Reduce anyOf/type-array usage or split the schema".into()),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-union-properties".into(),
            code: "{prefix}-S-max-union-properties".into(),
            description: format!("Union parameters must not exceed {}", self.limit),
            rationale: format!(
                "{} limits parameters that use anyOf or type arrays across strict schemas.",
                self.profile_name
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example:
                r#"{ "type": "object", "properties": { "value": { "anyOf": [{ "type": "string" }, { "type": "number" }] } } }"#
                    .into(),
            good_example: r#"{ "type": "object", "properties": { "value": { "type": "string" } }, "required": ["value"], "additionalProperties": false }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

fn optional_property_count(node: &crate::ir::Node) -> usize {
    missing_required_properties(node).len()
}

fn is_union_parameter(node: &crate::ir::Node) -> bool {
    node.annotations.any_of.is_some()
        || matches!(
            &node.annotations.r#type,
            Some(Value::Array(types)) if types.len() > 1
        )
}

impl Rule for MaxStringLengthRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let property_names = arena
            .iter()
            .filter_map(|(_, n)| n.annotations.properties.as_ref())
            .filter_map(|v| v.as_object())
            .flat_map(|props| props.keys())
            .map(|k| k.len())
            .sum::<usize>();
        let enum_strings = arena
            .iter()
            .filter_map(|(_, n)| n.annotations.enum_values.as_ref())
            .filter_map(|v| match v {
                Value::Array(arr) => Some(arr),
                _ => None,
            })
            .flatten()
            .filter_map(|v| v.as_str())
            .map(str::len)
            .sum::<usize>();
        let total = property_names + enum_strings;
        if total <= self.limit as usize {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-string-length-budget", profile.code_prefix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "total string length {} exceeds limit of {}",
                total, self.limit
            ),
            pointer: String::new(),
            source: None,
            profile: self.profile_name.clone(),
            hint: None,
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "string-length-budget".into(),
            code: "{prefix}-S-string-length-budget".into(),
            description: format!("Total property and enum string length must not exceed {}", self.limit),
            rationale: format!("{} enforces a schema string-length budget.", self.profile_name),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { \"very_long_property_name\": { \"type\": \"string\" } } }".into(),
            good_example: r#"{ "type": "object", "properties": { "name": { "type": "string" } } }"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
