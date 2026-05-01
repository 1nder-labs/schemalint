use crate::ir::{Arena, Node, NodeId};
use crate::profile::{Profile, Severity};

/// Severity of a diagnostic emitted by the rule engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

/// A lint diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub pointer: String,
    pub source: Option<()>, // placeholder for SourceSpan (Phase 3+)
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
            dynamic_rules.push(Box::new(super::class_a::KeywordRule {
                keyword,
                severity: diag_severity,
                code: format!("OAI-K-{}", keyword),
                profile_name: profile.name.clone(),
            }));
        }

        // Class A restriction rules.
        for (&keyword, restriction) in &profile.restrictions {
            dynamic_rules.push(Box::new(super::class_a::RestrictionRule {
                keyword,
                allowed_values: restriction.allowed_values.clone(),
                code: format!("OAI-K-{}-restricted", keyword),
                profile_name: profile.name.clone(),
            }));
        }

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

/// Return `true` if the given keyword appears in `node.annotations`.
pub fn keyword_present(node: &Node, keyword: &str) -> bool {
    keyword_value(node, keyword).is_some()
}

/// Return the `Value` associated with a known keyword, if any.
pub fn keyword_value<'a>(node: &'a Node, keyword: &str) -> Option<&'a serde_json::Value> {
    match keyword {
        "type" => node.annotations.r#type.as_ref(),
        "properties" => node.annotations.properties.as_ref(),
        "required" => node.annotations.required.as_ref(),
        "additionalProperties" => node.annotations.additional_properties.as_ref(),
        "items" => node.annotations.items.as_ref(),
        "prefixItems" => node.annotations.prefix_items.as_ref(),
        "minItems" => node.annotations.min_items.as_ref(),
        "maxItems" => node.annotations.max_items.as_ref(),
        "uniqueItems" => node.annotations.unique_items.as_ref(),
        "contains" => node.annotations.contains.as_ref(),
        "minimum" => node.annotations.minimum.as_ref(),
        "maximum" => node.annotations.maximum.as_ref(),
        "exclusiveMinimum" => node.annotations.exclusive_minimum.as_ref(),
        "exclusiveMaximum" => node.annotations.exclusive_maximum.as_ref(),
        "multipleOf" => node.annotations.multiple_of.as_ref(),
        "minLength" => node.annotations.min_length.as_ref(),
        "maxLength" => node.annotations.max_length.as_ref(),
        "pattern" => node.annotations.pattern.as_ref(),
        "format" => node.annotations.format.as_ref(),
        "enum" => node.annotations.enum_values.as_ref(),
        "const" => node.annotations.const_value.as_ref(),
        "patternProperties" => node.annotations.pattern_properties.as_ref(),
        "unevaluatedProperties" => node.annotations.unevaluated_properties.as_ref(),
        "propertyNames" => node.annotations.property_names.as_ref(),
        "minProperties" => node.annotations.min_properties.as_ref(),
        "maxProperties" => node.annotations.max_properties.as_ref(),
        "description" => node.annotations.description.as_ref(),
        "title" => node.annotations.title.as_ref(),
        "default" => node.annotations.default.as_ref(),
        "discriminator" => node.annotations.discriminator.as_ref(),
        "$ref" => node.annotations.r#ref.as_ref(),
        "$defs" => node.annotations.defs.as_ref(),
        "definitions" => node.annotations.definitions.as_ref(),
        "anyOf" => node.annotations.any_of.as_ref(),
        "allOf" => node.annotations.all_of.as_ref(),
        "oneOf" => node.annotations.one_of.as_ref(),
        "not" => node.annotations.not.as_ref(),
        "if" => node.annotations.if_schema.as_ref(),
        "then" => node.annotations.then_schema.as_ref(),
        "else" => node.annotations.else_schema.as_ref(),
        "dependentRequired" => node.annotations.dependent_required.as_ref(),
        "dependentSchemas" => node.annotations.dependent_schemas.as_ref(),
        _ => None,
    }
}
