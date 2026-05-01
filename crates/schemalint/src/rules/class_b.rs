use serde_json::Value;

use crate::ir::{Arena, Node, NodeId};
use crate::profile::Profile;
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

/// Return `true` if the node's schema describes an object type.
fn schema_is_object(node: &Node) -> bool {
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

    rules.push(Box::new(ExternalRefsRule {
        profile_name: profile.name.clone(),
    }));

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
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new(); // only check root
        }
        if !schema_is_object(&arena[node]) {
            return vec![Diagnostic {
                code: "OAI-S-object-root".to_string(),
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
}

#[derive(Debug, Clone)]
struct AdditionalPropertiesFalseRule {
    profile_name: String,
}

impl Rule for AdditionalPropertiesFalseRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        let node_ref = &arena[node];
        if !schema_is_object(node_ref) {
            return Vec::new();
        }
        match &node_ref.annotations.additional_properties {
            Some(Value::Bool(false)) => Vec::new(),
            _ => vec![Diagnostic {
                code: "OAI-S-additional-properties-false".to_string(),
                severity: DiagnosticSeverity::Error,
                message: "object must declare additionalProperties: false".to_string(),
                pointer: node_ref.json_pointer.clone(),
                source: None,
                profile: self.profile_name.clone(),
                hint: None,
            }],
        }
    }
}

#[derive(Debug, Clone)]
struct AllPropertiesRequiredRule {
    profile_name: String,
}

impl Rule for AllPropertiesRequiredRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
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
                    code: "OAI-S-all-properties-required".to_string(),
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
}

#[derive(Debug, Clone)]
struct MaxDepthRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxDepthRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].depth > self.limit {
            return vec![Diagnostic {
                code: "OAI-S-max-depth".to_string(),
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
}

#[derive(Debug, Clone)]
struct MaxTotalPropertiesRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxTotalPropertiesRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
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
                code: "OAI-S-max-total-properties".to_string(),
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
}

#[derive(Debug, Clone)]
struct MaxTotalEnumValuesRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxTotalEnumValuesRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
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
                code: "OAI-S-max-enum-values".to_string(),
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
}

#[derive(Debug, Clone)]
struct MaxStringLengthRule {
    limit: u32,
    profile_name: String,
}

impl Rule for MaxStringLengthRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
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
                code: "OAI-S-string-length-budget".to_string(),
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
}

#[derive(Debug, Clone)]
struct ExternalRefsRule {
    profile_name: String,
}

impl Rule for ExternalRefsRule {
    fn check(&self, node: NodeId, arena: &Arena, _profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if let Some(Value::String(ref_str)) = &arena[node].annotations.r#ref {
            if ref_str.starts_with("http://")
                || ref_str.starts_with("https://")
                || ref_str.starts_with('/')
            {
                diagnostics.push(Diagnostic {
                    code: "OAI-S-external-refs".to_string(),
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
}
