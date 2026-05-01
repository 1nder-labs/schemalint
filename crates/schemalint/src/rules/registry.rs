use serde::{Deserialize, Serialize};

use crate::ir::{Arena, Node, NodeId};
use crate::profile::{Profile, Severity};

/// Severity of a diagnostic emitted by the rule engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// Location in a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

/// A lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub pointer: String,
    pub source: Option<SourceSpan>,
    pub profile: String,
    pub hint: Option<String>,
}

/// Trait implemented by all lint rules.
pub trait Rule: Sync {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic>;
}

/// Stable identifier for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleId(pub u32);

// ---------------------------------------------------------------------------
// linkme distributed slice for compile-time rule registration
// ---------------------------------------------------------------------------

use linkme::distributed_slice;

#[distributed_slice]
pub static RULES: [&'static dyn Rule] = [..];

// ---------------------------------------------------------------------------
// RuleSet: combines static (linkme) and dynamic (profile-generated) rules
// ---------------------------------------------------------------------------

/// A collection of rules ready to run against a schema.
pub struct RuleSet {
    static_rules: &'static [&'static dyn Rule],
    dynamic_rules: Vec<Box<dyn Rule>>,
}

impl RuleSet {
    /// Build a RuleSet from a loaded profile. Generates Class A keyword and
    /// restriction rules from the profile data and includes all compile-time
    /// registered rules.
    pub fn from_profile(profile: &Profile) -> Self {
        let mut dynamic_rules: Vec<Box<dyn Rule>> = Vec::new();

        // Class A keyword rules.
        for (&keyword, &severity) in &profile.keyword_map {
            let diag_severity = match severity {
                Severity::Forbid | Severity::Strip => DiagnosticSeverity::Error,
                Severity::Warn => DiagnosticSeverity::Warning,
                _ => continue,
            };
            let accessor = keyword_accessor(keyword)
                .unwrap_or_else(|| panic!("profile contains unknown keyword '{}'", keyword));
            dynamic_rules.push(Box::new(super::class_a::KeywordRule {
                keyword,
                accessor,
                severity: diag_severity,
                code: format!("{}-K-{}", profile.code_prefix, keyword),
                profile_name: profile.name.clone(),
            }));
        }

        // Class A restriction rules.
        for (&keyword, restriction) in &profile.restrictions {
            let accessor = keyword_accessor(keyword)
                .unwrap_or_else(|| panic!("profile contains unknown keyword '{}'", keyword));
            dynamic_rules.push(Box::new(super::class_a::RestrictionRule {
                keyword,
                accessor,
                allowed_values: restriction.allowed_values.clone(),
                code: format!("{}-K-{}-restricted", profile.code_prefix, keyword),
                profile_name: profile.name.clone(),
            }));
        }

        // Class B structural rules.
        dynamic_rules.extend(super::class_b::generate_class_b_rules(profile));

        Self {
            static_rules: &*RULES,
            dynamic_rules,
        }
    }

    /// Run every rule in the set against a single node.
    pub fn check_node(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for &rule in self.static_rules {
            diagnostics.extend(rule.check(node, arena, profile));
        }
        for rule in &self.dynamic_rules {
            diagnostics.extend(rule.check(node, arena, profile));
        }
        diagnostics
    }

    /// Run every rule against every node in the arena and collect all diagnostics.
    pub fn check_all(&self, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (node_id, _) in arena.iter() {
            diagnostics.extend(self.check_node(node_id, arena, profile));
        }
        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Helper: check whether a keyword is present in a node's annotations
// ---------------------------------------------------------------------------

/// Function pointer type for extracting a keyword value from a node.
pub type KeywordAccessor = fn(&Node) -> Option<&serde_json::Value>;

/// Return `true` if the given keyword appears in `node.annotations`.
pub fn keyword_present(node: &Node, keyword: &str) -> bool {
    keyword_value(node, keyword).is_some()
}

/// Return a function pointer that extracts the value for a known keyword.
///
/// This compiles the 40-arm match into a single function-pointer dispatch,
/// eliminating string comparison overhead in hot rule loops.
pub fn keyword_accessor(keyword: &str) -> Option<KeywordAccessor> {
    match keyword {
        "type" => Some(|n| n.annotations.r#type.as_ref()),
        "properties" => Some(|n| n.annotations.properties.as_ref()),
        "required" => Some(|n| n.annotations.required.as_ref()),
        "additionalProperties" => Some(|n| n.annotations.additional_properties.as_ref()),
        "items" => Some(|n| n.annotations.items.as_ref()),
        "prefixItems" => Some(|n| n.annotations.prefix_items.as_ref()),
        "minItems" => Some(|n| n.annotations.min_items.as_ref()),
        "maxItems" => Some(|n| n.annotations.max_items.as_ref()),
        "uniqueItems" => Some(|n| n.annotations.unique_items.as_ref()),
        "contains" => Some(|n| n.annotations.contains.as_ref()),
        "minimum" => Some(|n| n.annotations.minimum.as_ref()),
        "maximum" => Some(|n| n.annotations.maximum.as_ref()),
        "exclusiveMinimum" => Some(|n| n.annotations.exclusive_minimum.as_ref()),
        "exclusiveMaximum" => Some(|n| n.annotations.exclusive_maximum.as_ref()),
        "multipleOf" => Some(|n| n.annotations.multiple_of.as_ref()),
        "minLength" => Some(|n| n.annotations.min_length.as_ref()),
        "maxLength" => Some(|n| n.annotations.max_length.as_ref()),
        "pattern" => Some(|n| n.annotations.pattern.as_ref()),
        "format" => Some(|n| n.annotations.format.as_ref()),
        "enum" => Some(|n| n.annotations.enum_values.as_ref()),
        "const" => Some(|n| n.annotations.const_value.as_ref()),
        "patternProperties" => Some(|n| n.annotations.pattern_properties.as_ref()),
        "unevaluatedProperties" => Some(|n| n.annotations.unevaluated_properties.as_ref()),
        "propertyNames" => Some(|n| n.annotations.property_names.as_ref()),
        "minProperties" => Some(|n| n.annotations.min_properties.as_ref()),
        "maxProperties" => Some(|n| n.annotations.max_properties.as_ref()),
        "description" => Some(|n| n.annotations.description.as_ref()),
        "title" => Some(|n| n.annotations.title.as_ref()),
        "default" => Some(|n| n.annotations.default.as_ref()),
        "discriminator" => Some(|n| n.annotations.discriminator.as_ref()),
        "$ref" => Some(|n| n.annotations.r#ref.as_ref()),
        "$defs" => Some(|n| n.annotations.defs.as_ref()),
        "definitions" => Some(|n| n.annotations.definitions.as_ref()),
        "anyOf" => Some(|n| n.annotations.any_of.as_ref()),
        "allOf" => Some(|n| n.annotations.all_of.as_ref()),
        "oneOf" => Some(|n| n.annotations.one_of.as_ref()),
        "not" => Some(|n| n.annotations.not.as_ref()),
        "if" => Some(|n| n.annotations.if_schema.as_ref()),
        "then" => Some(|n| n.annotations.then_schema.as_ref()),
        "else" => Some(|n| n.annotations.else_schema.as_ref()),
        "dependentRequired" => Some(|n| n.annotations.dependent_required.as_ref()),
        "dependentSchemas" => Some(|n| n.annotations.dependent_schemas.as_ref()),
        _ => None,
    }
}

/// Return the `Value` associated with a known keyword, if any.
pub fn keyword_value<'a>(node: &'a Node, keyword: &str) -> Option<&'a serde_json::Value> {
    keyword_accessor(keyword).and_then(|f| f(node))
}
