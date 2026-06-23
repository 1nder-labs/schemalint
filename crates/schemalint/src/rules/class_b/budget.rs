use serde_json::Value;

use crate::ir::{Arena, NodeId};
use crate::profile::{Profile, Severity};
use crate::rules::class_b::helpers::missing_required_properties;
use crate::rules::metadata::{RuleCategory, RuleMetadata};
use crate::rules::registry::{Diagnostic, DiagnosticSeverity, Rule};

// ---------------------------------------------------------------------------
// MaxDepthRule — structurally different: no root-only guard, reads node depth
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// BudgetRule — shared shape: root-only guard + arena counter + one diagnostic
// ---------------------------------------------------------------------------

/// Data that varies per budget rule. The counter fn takes the whole arena so
/// it can iterate; it never reads the current `NodeId` (root-only guard fires
/// first). Each variant carries its own hint, examples, and metadata strings.
#[derive(Debug)]
struct BudgetRuleData {
    code_suffix: &'static str,
    message_label: &'static str,
    hint: Option<&'static str>,
    desc_subject: &'static str,
    rationale_tail: &'static str,
    bad_example: &'static str,
    good_example: &'static str,
    counter: fn(&Arena) -> usize,
}

#[derive(Debug, Clone)]
pub(super) struct BudgetRule {
    limit: u32,
    profile_name: String,
    data: &'static BudgetRuleData,
}

// ---------------------------------------------------------------------------
// Static data tables — one per budget variant
// ---------------------------------------------------------------------------

static MAX_TOTAL_PROPERTIES: BudgetRuleData = BudgetRuleData {
    code_suffix: "max-total-properties",
    message_label: "total property count",
    hint: None,
    desc_subject: "Total object properties",
    rationale_tail: "limits the total number of object properties.",
    bad_example: "{ \"type\": \"object\", \"properties\": { \"...many\": {} } }",
    good_example: r#"{ "type": "object", "properties": { "name": { "type": "string" } } }"#,
    counter: count_total_properties,
};

static MAX_TOTAL_ENUM_VALUES: BudgetRuleData = BudgetRuleData {
    code_suffix: "max-enum-values",
    message_label: "total enum value count",
    hint: None,
    desc_subject: "Total enum values",
    rationale_tail: "limits total enum values.",
    bad_example: "{ \"type\": \"string\", \"enum\": [\"...1000+ values\"] }",
    good_example: r#"{ "type": "string", "enum": ["red", "green", "blue"] }"#,
    counter: count_total_enum_values,
};

static MAX_STRING_LENGTH: BudgetRuleData = BudgetRuleData {
    code_suffix: "string-length-budget",
    message_label: "total string length",
    hint: None,
    desc_subject: "Total property and enum string length",
    rationale_tail: "enforces a schema string-length budget.",
    bad_example: "{ \"type\": \"object\", \"properties\": { \"very_long_property_name\": { \"type\": \"string\" } } }",
    good_example: r#"{ "type": "object", "properties": { "name": { "type": "string" } } }"#,
    counter: count_string_length,
};

static MAX_OPTIONAL_PROPERTIES: BudgetRuleData = BudgetRuleData {
    code_suffix: "max-optional-properties",
    message_label: "optional property count",
    hint: Some("Mark more properties as required or split the schema"),
    desc_subject: "Optional properties",
    rationale_tail: "limits optional parameters across strict schemas.",
    bad_example: r#"{ "type": "object", "properties": { "optional": { "type": "string" } } }"#,
    good_example: r#"{ "type": "object", "properties": { "required": { "type": "string" } }, "required": ["required"], "additionalProperties": false }"#,
    counter: count_optional_properties,
};

static MAX_UNION_PROPERTIES: BudgetRuleData = BudgetRuleData {
    code_suffix: "max-union-properties",
    message_label: "union parameter count",
    hint: Some("Reduce anyOf/type-array usage or split the schema"),
    desc_subject: "Union parameters",
    rationale_tail: "limits parameters that use anyOf or type arrays across strict schemas.",
    bad_example: r#"{ "type": "object", "properties": { "value": { "anyOf": [{ "type": "string" }, { "type": "number" }] } } }"#,
    good_example: r#"{ "type": "object", "properties": { "value": { "type": "string" } }, "required": ["value"], "additionalProperties": false }"#,
    counter: count_union_properties,
};

// ---------------------------------------------------------------------------
// Named counter functions (one per variant — named for readability in traces)
// ---------------------------------------------------------------------------

fn count_total_properties(arena: &Arena) -> usize {
    arena
        .iter()
        .filter_map(|(_, n)| n.annotations.properties.as_ref())
        .filter_map(|v| v.as_object().map(|o| o.len()))
        .sum()
}

fn count_total_enum_values(arena: &Arena) -> usize {
    arena
        .iter()
        .filter_map(|(_, n)| n.annotations.enum_values.as_ref())
        .filter_map(|v| v.as_array().map(|a| a.len()))
        .sum()
}

fn count_string_length(arena: &Arena) -> usize {
    let property_names: usize = arena
        .iter()
        .filter_map(|(_, n)| n.annotations.properties.as_ref())
        .filter_map(|v| v.as_object())
        .flat_map(|props| props.keys())
        .map(|k| k.len())
        .sum();
    let enum_strings: usize = arena
        .iter()
        .filter_map(|(_, n)| n.annotations.enum_values.as_ref())
        .filter_map(|v| match v {
            Value::Array(arr) => Some(arr),
            _ => None,
        })
        .flatten()
        .filter_map(|v| v.as_str())
        .map(str::len)
        .sum();
    property_names + enum_strings
}

fn count_optional_properties(arena: &Arena) -> usize {
    arena
        .iter()
        .map(|(_, node)| missing_required_properties(node).len())
        .sum()
}

fn count_union_properties(arena: &Arena) -> usize {
    arena
        .iter()
        .filter(|(_, node)| {
            node.annotations.any_of.is_some()
                || matches!(
                    &node.annotations.r#type,
                    Some(Value::Array(types)) if types.len() > 1
                )
        })
        .count()
}

// ---------------------------------------------------------------------------
// Rule impl — identical logic for all five variants
// ---------------------------------------------------------------------------

impl Rule for BudgetRule {
    fn check(&self, node: NodeId, arena: &Arena, profile: &Profile) -> Vec<Diagnostic> {
        if arena[node].parent.is_some() {
            return Vec::new();
        }
        let d = self.data;
        let total = (d.counter)(arena);
        if total <= self.limit as usize {
            return Vec::new();
        }
        vec![Diagnostic {
            code: format!("{}-S-{}", profile.code_prefix, d.code_suffix),
            severity: DiagnosticSeverity::Error,
            message: format!(
                "{} {} exceeds limit of {}",
                d.message_label, total, self.limit
            ),
            pointer: String::new(),
            source: None,
            profile: self.profile_name.clone(),
            hint: d.hint.map(str::to_owned),
        }]
    }

    fn metadata(&self) -> Option<RuleMetadata> {
        let d = self.data;
        Some(RuleMetadata {
            name: d.code_suffix.into(),
            code: format!("{{prefix}}-S-{}", d.code_suffix),
            description: format!("{} must not exceed {}", d.desc_subject, self.limit),
            rationale: format!("{} {}", self.profile_name, d.rationale_tail),
            severity: Severity::Forbid,
            category: RuleCategory::Structural,
            bad_example: d.bad_example.into(),
            good_example: d.good_example.into(),
            see_also: Vec::new(),
            profile: Some(self.profile_name.clone()),
        })
    }
}

// ---------------------------------------------------------------------------
// Public constructors — used by class_b.rs
// ---------------------------------------------------------------------------

impl BudgetRule {
    pub(super) fn max_total_properties(limit: u32, profile_name: String) -> Self {
        Self {
            limit,
            profile_name,
            data: &MAX_TOTAL_PROPERTIES,
        }
    }

    pub(super) fn max_total_enum_values(limit: u32, profile_name: String) -> Self {
        Self {
            limit,
            profile_name,
            data: &MAX_TOTAL_ENUM_VALUES,
        }
    }

    pub(super) fn max_string_length(limit: u32, profile_name: String) -> Self {
        Self {
            limit,
            profile_name,
            data: &MAX_STRING_LENGTH,
        }
    }

    pub(super) fn max_optional_properties(limit: u32, profile_name: String) -> Self {
        Self {
            limit,
            profile_name,
            data: &MAX_OPTIONAL_PROPERTIES,
        }
    }

    pub(super) fn max_union_properties(limit: u32, profile_name: String) -> Self {
        Self {
            limit,
            profile_name,
            data: &MAX_UNION_PROPERTIES,
        }
    }
}
