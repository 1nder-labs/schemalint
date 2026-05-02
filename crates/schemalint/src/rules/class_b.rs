use serde_json::Value;

use crate::ir::{Arena, Node, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

/// Return `true` if the node's schema describes an object type.
pub(crate) fn schema_is_object(node: &Node) -> bool {
    match &node.annotations.r#type {
        Some(Value::String(s)) => s == "object",
        Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some("object")),
        None => {
            // Infer object type from object-specific keywords when no explicit
            // type is present. This is safe because the None arm only fires
            // when type is absent; an explicit non-object type (e.g. "array")
            // is handled by the String/Array arms above.
            node.annotations.properties.is_some()
                || node.annotations.additional_properties.is_some()
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Class B structural rules generated from profile [structural] section
// ---------------------------------------------------------------------------

/// Generate all Class B structural rules from a loaded profile.
pub fn generate_class_b_rules(profile: &Profile) -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    let s = &profile.structural;

    if s.require_object_root {
        rules.push(Box::new(ObjectRootRule {
            profile_name: profile.name.clone(),
        }));
    }

    if s.require_additional_properties_false {
        rules.push(Box::new(AdditionalPropertiesFalseRule {
            profile_name: profile.name.clone(),
        }));
    }

    if s.require_all_properties_in_required {
        rules.push(Box::new(AllPropertiesRequiredRule {
            profile_name: profile.name.clone(),
        }));
    }

    if s.max_object_depth > 0 {
        rules.push(Box::new(MaxDepthRule {
            limit: s.max_object_depth,
            profile_name: profile.name.clone(),
        }));
    }

    if s.max_total_properties > 0 {
        rules.push(Box::new(MaxTotalPropertiesRule {
            limit: s.max_total_properties,
            profile_name: profile.name.clone(),
        }));
    }

    if s.max_total_enum_values > 0 {
        rules.push(Box::new(MaxTotalEnumValuesRule {
            limit: s.max_total_enum_values,
            profile_name: profile.name.clone(),
        }));
    }

    if s.max_string_length_total > 0 {
        rules.push(Box::new(MaxStringLengthRule {
            limit: s.max_string_length_total,
            profile_name: profile.name.clone(),
        }));
    }

    if s.external_refs {
        rules.push(Box::new(ExternalRefsRule {
            profile_name: profile.name.clone(),
        }));
    }

    if profile.code_prefix == "ANT" {
        rules.push(Box::new(AllOfWithRefRule {
            profile_name: profile.name.clone(),
        }));
    }

    rules
}

// ---------------------------------------------------------------------------
// Individual structural rules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ObjectRootRule {
    profile_name: String,
}

impl Rule for ObjectRootRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new(); // only check root
        }
        if !schema_is_object(&arena[node]) {
            return vec![Diagnostic {
                code: format!("{}-S-object-root", profile.code_prefix),
                severity: DiagnosticSeverity::Error,
                message: "root schema must be an object".to_string(),
                pointer: arena[node].json_pointer.clone(),
                source: None,
                profile: self.profile_name.clone(),
                hint: None,
            }];
        }
        Vec::new()
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "object-root".into(),
            code: "{prefix}-S-object-root".into(),
            description: "The root schema must be of type object".into(),
            rationale: "Structured-output providers require the top-level schema to be an object. Array, string, or primitive root schemas are rejected at the API level.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{
  "type": "array",
  "items": { "type": "string" }
}"#.into(),
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

#[derive(Debug, Clone)]
struct AdditionalPropertiesFalseRule {
    profile_name: String,
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
            rationale: "Providers require all object nodes to explicitly set additionalProperties: false to guarantee no unexpected properties appear in responses.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}"#.into(),
            good_example: r#"{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "name": { "type": "string" }
  }
}"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
struct AllPropertiesRequiredRule {
    profile_name: String,
}

impl Rule for AllPropertiesRequiredRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_object(node_ref) {
            return Vec::new();
        }
        let Some(Value::Object(props)) = &node_ref.annotations.properties else {
            return Vec::new();
        };
        let required: std::collections::HashSet<String> = match &node_ref.annotations.required {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => std::collections::HashSet::new(),
        };

        let mut diagnostics = Vec::new();
        for key in props.keys() {
            if !required.contains(key) {
                diagnostics.push(Diagnostic {
                    code: format!("{}-S-all-properties-required", profile.code_prefix),
                    severity: DiagnosticSeverity::Error,
                    message: format!("property '{}' is not listed in required", key),
                    pointer: format!("{}/properties/{}", node_ref.json_pointer, key),
                    source: None,
                    profile: self.profile_name.clone(),
                    hint: None,
                });
            }
        }
        diagnostics
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "all-properties-required".into(),
            code: "{prefix}-S-all-properties-required".into(),
            description: "Every property must be listed in the required array".into(),
            rationale: "Some providers require that all defined properties appear in the required array to enforce strict schema adherence and prevent ambiguity about optional fields.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "number" }
  },
  "required": ["name"]
}"#.into(),
            good_example: r#"{
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "number" }
  },
  "required": ["name", "age"]
}"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
struct MaxDepthRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxDepthRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].depth > self.limit {
            return vec![Diagnostic {
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
            }];
        }
        Vec::new()
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
                "{} limits object nesting depth to {} levels. Exceeding this causes API rejection.",
                self.profile_name, self.limit
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { \"a\": { \"type\": \"object\", \"properties\": { \"b\": { \"type\": \"object\", \"properties\": { \"c\": { \"type\": \"object\", \"properties\": { \"d\": { \"type\": \"object\", \"properties\": { \"e\": { \"type\": \"object\", \"properties\": { \"f\": { \"type\": \"object\", \"properties\": { \"g\": { \"type\": \"object\", \"properties\": { \"h\": { \"type\": \"object\", \"properties\": { \"i\": { \"type\": \"object\", \"properties\": { \"j\": { \"type\": \"object\", \"properties\": { \"k\": { \"type\": \"object\", \"properties\": {} } } } } } } } } } } } } } } } } } } } } } } }".into(),
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

#[derive(Debug, Clone)]
struct MaxTotalPropertiesRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxTotalPropertiesRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        // Global rule: only run on root.
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let total: usize = arena
            .iter()
            .filter_map(|(_, n)| n.annotations.properties.as_ref())
            .filter_map(|v| v.as_object().map(|o| o.len()))
            .sum();
        if total > self.limit as usize {
            return vec![Diagnostic {
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
            }];
        }
        Vec::new()
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-total-properties".into(),
            code: "{prefix}-S-max-total-properties".into(),
            description: format!(
                "Total number of properties across all objects must not exceed {}",
                self.limit
            ),
            rationale: format!(
                "{} limits the total number of object properties across the entire schema to {}.",
                self.profile_name, self.limit
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { ...many properties exceeding the limit... } }".into(),
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

#[derive(Debug, Clone)]
struct MaxTotalEnumValuesRule {
    limit: u32,
    profile_name: String,
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
        if total > self.limit as usize {
            return vec![Diagnostic {
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
            }];
        }
        Vec::new()
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "max-enum-values".into(),
            code: "{prefix}-S-max-enum-values".into(),
            description: format!(
                "Total number of enum values across the schema must not exceed {}",
                self.limit
            ),
            rationale: format!(
                "{} imposes a limit of {} total enum values across the entire schema.",
                self.profile_name, self.limit
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { \"color\": { \"enum\": [...1000+ values...] } } }".into(),
            good_example: r#"{
  "type": "object",
  "properties": {
    "color": { "enum": ["red", "green", "blue"] }
  }
}"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
struct MaxStringLengthRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxStringLengthRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let mut total: usize = 0;

        // Sum property name lengths.
        for (_, n) in arena.iter() {
            if let Some(Value::Object(props)) = &n.annotations.properties {
                total += props.keys().map(|k| k.len()).sum::<usize>();
            }
        }

        // Sum string enum value lengths.
        for (_, n) in arena.iter() {
            if let Some(Value::Array(arr)) = &n.annotations.enum_values {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        total += s.len();
                    }
                }
            }
        }

        if total > self.limit as usize {
            return vec![Diagnostic {
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
            }];
        }
        Vec::new()
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "string-length-budget".into(),
            code: "{prefix}-S-string-length-budget".into(),
            description: format!(
                "Total string length (property names + enum values) must not exceed {}",
                self.limit
            ),
            rationale: format!(
                "{} imposes a string length budget of {} across all property names and enum values.",
                self.profile_name, self.limit
            ),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: "{ \"type\": \"object\", \"properties\": { \"very_long_property_name\": { \"type\": \"string\" } } }".into(),
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

#[derive(Debug, Clone)]
struct ExternalRefsRule {
    profile_name: String,
}

impl Rule for ExternalRefsRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if let Some(Value::String(ref_str)) = &arena[node].annotations.r#ref {
            if ref_str.starts_with("http://")
                || ref_str.starts_with("https://")
                || ref_str.starts_with('/')
            {
                diagnostics.push(Diagnostic {
                    code: format!("{}-S-external-refs", profile.code_prefix),
                    severity: DiagnosticSeverity::Error,
                    message: format!("external $ref '{}' is not supported", ref_str),
                    pointer: arena[node].json_pointer.clone(),
                    source: None,
                    profile: self.profile_name.clone(),
                    hint: None,
                });
            }
        }
        diagnostics
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "external-refs".into(),
            code: "{prefix}-S-external-refs".into(),
            description: "External $ref values (URLs, absolute paths) are not supported".into(),
            rationale: "Providers require all $ref references to be internal to the schema (e.g., `#/$defs/Foo`). External references via URLs or file paths are rejected.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r##"{
  "type": "object",
  "properties": {
    "address": { "$ref": "https://example.com/schemas/address.json" }
  }
}"##.into(),
            good_example: r##"{
  "type": "object",
  "$defs": {
    "Address": {
      "type": "object",
      "properties": {
        "street": { "type": "string" }
      }
    }
  },
  "properties": {
    "address": { "$ref": "#/$defs/Address" }
  }
}"##.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

#[derive(Debug, Clone)]
struct AllOfWithRefRule {
    profile_name: String,
}

impl Rule for AllOfWithRefRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        let Some(Value::Array(branches)) = &node_ref.annotations.all_of else {
            return Vec::new();
        };

        fn contains_ref(val: &Value) -> bool {
            match val {
                Value::Object(map) => {
                    if map.contains_key("$ref") {
                        return true;
                    }
                    map.values().any(contains_ref)
                }
                Value::Array(arr) => arr.iter().any(contains_ref),
                _ => false,
            }
        }

        for branch in branches {
            if contains_ref(branch) {
                return vec![Diagnostic {
                    code: format!("{}-S-allof-with-ref", profile.code_prefix),
                    severity: DiagnosticSeverity::Error,
                    message:
                        "Anthropic Structured Outputs does not support allOf combined with $ref"
                            .to_string(),
                    pointer: node_ref.json_pointer.clone(),
                    source: None,
                    profile: self.profile_name.clone(),
                    hint: None,
                }];
            }
        }
        Vec::new()
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        Some(RuleMetadata {
            name: "allof-with-ref".into(),
            code: "{prefix}-S-allof-with-ref".into(),
            description: "allOf combined with $ref is not supported by Anthropic".into(),
            rationale: "Anthropic Structured Outputs does not support combining allOf with $ref references. Schemas using this pattern will be rejected by the Anthropic API.".into(),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: r##"{
  "type": "object",
  "allOf": [
    { "$ref": "#/$defs/Base" },
    { "properties": { "extra": { "type": "string" } } }
  ],
  "$defs": {
    "Base": {
      "type": "object",
      "properties": { "id": { "type": "string" } }
    }
  }
}"##.into(),
            good_example: r#"{
  "type": "object",
  "properties": {
    "id": { "type": "string" },
    "extra": { "type": "string" }
  }
}"#.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}
